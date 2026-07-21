use crate::runtime::VaultRuntime;
use tauri::{AppHandle, Emitter, Manager};

const VAULT_LOCKED_EVENT: &str = "vault-locked";

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
    // AppKit publishes these process-lifetime notification-name constants.
    let lock_names = unsafe {
        [
            NSWorkspaceWillSleepNotification,
            NSWorkspaceSessionDidResignActiveNotification,
        ]
    };
    for name in lock_names {
        let app = app.clone();
        let observer = RcBlock::new(move |_: NonNull<NSNotification>| lock_and_notify(&app));
        unsafe {
            center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &observer);
        }
    }
    // Wake and session activation re-enable explicit unlock only after the store is closed.
    let resume_names = unsafe {
        [
            NSWorkspaceDidWakeNotification,
            NSWorkspaceSessionDidBecomeActiveNotification,
        ]
    };
    for name in resume_names {
        let app = app.clone();
        let observer = RcBlock::new(move |_: NonNull<NSNotification>| resume_and_notify(&app));
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
}
