use super::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;

impl VaultRuntime {
    pub(super) fn start_local_inbox_watcher(&self, app: AppHandle) -> Result<(), RuntimeError> {
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
                if event.is_ok() {
                    let _ = wake_sender.send(());
                }
            })
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?;
        watcher
            .watch(&inbox, RecursiveMode::NonRecursive)
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?;
        *self
            .inner
            .local_inbox_watcher
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))? = Some(watcher);

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
                    let _ = rescan_and_process_local_inbox(&app, runtime).await;
                });
            }
        });
        Ok(())
    }
}
