use crate::runtime::VaultRuntime;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

const VAULT_LOCKED_EVENT: &str = "vault-locked";

#[derive(Clone, Copy)]
enum SystemEvent {
    DidWake,
    SessionBecameActive,
    SessionResigned,
    WillSleep,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SystemLockAction {
    KeepLocked,
    Lock,
    Resume,
}

struct SystemLockState {
    session_active: bool,
    sleeping: bool,
}

impl SystemLockState {
    fn new() -> Self {
        Self {
            session_active: true,
            sleeping: false,
        }
    }

    fn apply(&mut self, event: SystemEvent) -> SystemLockAction {
        match event {
            SystemEvent::WillSleep => {
                self.sleeping = true;
                SystemLockAction::Lock
            }
            SystemEvent::SessionResigned => {
                self.session_active = false;
                SystemLockAction::Lock
            }
            SystemEvent::DidWake => {
                self.sleeping = false;
                if self.session_active {
                    SystemLockAction::Resume
                } else {
                    SystemLockAction::KeepLocked
                }
            }
            SystemEvent::SessionBecameActive => {
                self.session_active = true;
                if self.sleeping {
                    SystemLockAction::KeepLocked
                } else {
                    SystemLockAction::Resume
                }
            }
        }
    }
}

fn lock_and_notify(app: &AppHandle) {
    request_lock_and_notify(app.state::<VaultRuntime>().inner(), || {
        let _ = app.emit(VAULT_LOCKED_EVENT, ());
    });
}

fn request_lock_and_notify(runtime: &VaultRuntime, notify: impl FnOnce()) {
    let _ = runtime.request_system_lock();
    notify();
}

fn resume_and_notify(app: &AppHandle) {
    let _ = app.state::<VaultRuntime>().resume_system_session();
    let _ = app.emit(VAULT_LOCKED_EVENT, ());
}

fn handle_system_event(app: &AppHandle, state: &Mutex<SystemLockState>, event: SystemEvent) {
    let action = state
        .lock()
        .map(|mut state| state.apply(event))
        .unwrap_or(SystemLockAction::KeepLocked);
    match action {
        SystemLockAction::Lock => lock_and_notify(app),
        SystemLockAction::Resume => resume_and_notify(app),
        SystemLockAction::KeepLocked => {
            let _ = app.emit(VAULT_LOCKED_EVENT, ());
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn install(app: AppHandle) {
    use block2::RcBlock;
    use objc2_app_kit::{
        NSWorkspace, NSWorkspaceDidWakeNotification, NSWorkspaceSessionDidBecomeActiveNotification,
        NSWorkspaceSessionDidResignActiveNotification, NSWorkspaceWillSleepNotification,
    };
    use objc2_foundation::NSNotification;
    use std::ptr::NonNull;

    let center = NSWorkspace::sharedWorkspace().notificationCenter();
    let state = Arc::new(Mutex::new(SystemLockState::new()));
    // AppKit publishes these process-lifetime notification-name constants.
    let notifications = unsafe {
        [
            (NSWorkspaceWillSleepNotification, SystemEvent::WillSleep),
            (NSWorkspaceDidWakeNotification, SystemEvent::DidWake),
            (
                NSWorkspaceSessionDidResignActiveNotification,
                SystemEvent::SessionResigned,
            ),
            (
                NSWorkspaceSessionDidBecomeActiveNotification,
                SystemEvent::SessionBecameActive,
            ),
        ]
    };
    for (name, event) in notifications {
        let app = app.clone();
        let state = state.clone();
        let observer = RcBlock::new(move |_: NonNull<NSNotification>| {
            handle_system_event(&app, &state, event)
        });
        unsafe {
            center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &observer);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn install(_app: AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::VaultStatus;

    #[test]
    fn locks_the_runtime_before_notifying_the_renderer() {
        let temporary = tempfile::tempdir().expect("temporary Vault parent");
        let runtime = VaultRuntime::new(temporary.path().join("vault"));
        runtime.create(b"vault-password").expect("create Vault");

        request_lock_and_notify(&runtime, || {
            assert_eq!(
                runtime.status().expect("locked status"),
                VaultStatus::Locked
            );
        });
        assert_eq!(
            runtime
                .unlock(b"vault-password")
                .expect_err("reject unlock while the system session is inactive")
                .code(),
            "vault_locked"
        );
        assert_eq!(
            runtime
                .unlock_with_keychain()
                .expect_err("reject Keychain unlock while the system session is inactive")
                .code(),
            "vault_locked"
        );

        runtime
            .resume_system_session()
            .expect("resume system session");
        assert_eq!(
            runtime
                .unlock(b"vault-password")
                .expect("unlock after resume"),
            VaultStatus::Unlocked
        );
    }

    #[test]
    fn wake_resumes_only_while_the_user_session_is_active() {
        let mut state = SystemLockState::new();
        assert_eq!(state.apply(SystemEvent::WillSleep), SystemLockAction::Lock);
        assert_eq!(state.apply(SystemEvent::DidWake), SystemLockAction::Resume);

        assert_eq!(
            state.apply(SystemEvent::SessionResigned),
            SystemLockAction::Lock
        );
        assert_eq!(state.apply(SystemEvent::WillSleep), SystemLockAction::Lock);
        assert_eq!(
            state.apply(SystemEvent::DidWake),
            SystemLockAction::KeepLocked
        );
        assert_eq!(
            state.apply(SystemEvent::SessionBecameActive),
            SystemLockAction::Resume
        );
    }
}
