use super::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;

fn wake_for_notify_event(wake_sender: &mpsc::Sender<()>, _event: notify::Result<notify::Event>) {
    let _ = wake_sender.send(());
}

impl VaultRuntime {
    pub(super) fn start_local_inbox_watcher(&self, app: AppHandle) -> Result<(), RuntimeError> {
        if self
            .inner
            .local_inbox_watcher
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?
            .is_some()
        {
            return Ok(());
        }
        let inbox = {
            let access = self
                .inner
                .local_inbox_access
                .lock()
                .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?;
            let root = access
                .as_ref()
                .ok_or_else(|| RuntimeError::new("local_inbox_not_configured"))?;
            ensure_inbox_paths(root.root())
                .map_err(|_| RuntimeError::new("local_inbox_setup_failed"))?
                .inbox
        };
        let (wake_sender, wake_receiver) = mpsc::channel();
        let mut watcher: RecommendedWatcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                wake_for_notify_event(&wake_sender, event);
            })
            .map_err(|_| {
                self.inner
                    .local_inbox_needs_attention
                    .store(true, Ordering::SeqCst);
                RuntimeError::new("local_inbox_unavailable")
            })?;
        watcher
            .watch(&inbox, RecursiveMode::NonRecursive)
            .map_err(|_| {
                self.inner
                    .local_inbox_needs_attention
                    .store(true, Ordering::SeqCst);
                RuntimeError::new("local_inbox_unavailable")
            })?;
        *self
            .inner
            .local_inbox_watcher
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))? = Some(watcher);
        self.inner
            .local_inbox_needs_attention
            .store(false, Ordering::SeqCst);

        let runtime = self.clone();
        std::thread::spawn(move || {
            while wake_receiver.recv().is_ok() {
                while wake_receiver
                    .recv_timeout(Duration::from_millis(250))
                    .is_ok()
                {}
                let app = app.clone();
                let runtime = runtime.clone();
                tauri::async_runtime::spawn(async move {
                    if rescan_and_process_local_inbox(&app, runtime.clone())
                        .await
                        .is_err()
                    {
                        runtime.mark_local_inbox_needs_attention();
                    }
                });
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_errors_still_wake_the_inbox_scanner() {
        let (sender, receiver) = mpsc::channel();

        wake_for_notify_event(&sender, Err(notify::Error::generic("test error")));

        receiver.try_recv().expect("notify error wakes scanner");
    }
}
