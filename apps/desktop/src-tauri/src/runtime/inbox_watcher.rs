use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{io, path::Path};

pub(super) struct LocalInboxWatcher {
    _watcher: RecommendedWatcher,
}

impl LocalInboxWatcher {
    pub(super) fn start(
        inbox: &Path,
        mut on_change: impl FnMut() + Send + 'static,
    ) -> io::Result<Self> {
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                if result
                    .as_ref()
                    .is_ok_and(|event| event_kind_wakes_scan(&event.kind))
                {
                    on_change();
                }
            })
            .map_err(notify_error)?;
        watcher
            .watch(inbox, RecursiveMode::NonRecursive)
            .map_err(notify_error)?;
        Ok(Self { _watcher: watcher })
    }
}

fn event_kind_wakes_scan(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any
            | EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Remove(_)
            | EventKind::Other
    )
}

fn notify_error(error: notify::Error) -> io::Error {
    io::Error::other(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{LocalInboxWatcher, event_kind_wakes_scan};
    use notify::{
        EventKind,
        event::{AccessKind, CreateKind, ModifyKind, RemoveKind},
    };
    use std::{fs, sync::mpsc, time::Duration};

    #[test]
    fn wakes_only_for_directory_content_changes() {
        assert!(event_kind_wakes_scan(&EventKind::Create(CreateKind::Any)));
        assert!(event_kind_wakes_scan(&EventKind::Modify(ModifyKind::Any)));
        assert!(event_kind_wakes_scan(&EventKind::Remove(RemoveKind::Any)));
        assert!(!event_kind_wakes_scan(&EventKind::Access(AccessKind::Any)));
    }

    #[test]
    fn wakes_when_a_file_arrives_in_the_inbox() {
        let inbox = tempfile::tempdir().expect("temporary Inbox");
        let (sender, receiver) = mpsc::channel();
        let _watcher = LocalInboxWatcher::start(inbox.path(), move || {
            let _ = sender.send(());
        })
        .expect("watch Inbox");

        fs::write(inbox.path().join("statement.pdf"), b"%PDF-1.7")
            .expect("write synthetic statement");

        receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("filesystem change hint");
    }
}
