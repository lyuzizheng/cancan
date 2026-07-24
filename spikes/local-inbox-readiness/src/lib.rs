use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSnapshot {
    identity: FileIdentity,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    NotRegularFile,
    Unreadable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedFile {
    pub bytes: Vec<u8>,
    pub sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferReason {
    ChangedBeforeRead,
    ChangedAfterRead,
    Unreadable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureOutcome {
    Captured(CapturedFile),
    Deferred(DeferReason),
    Suppressed { sha256: String },
}

pub fn snapshot(path: &Path) -> Result<FileSnapshot, SnapshotError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| SnapshotError::Unreadable)?;
    snapshot_metadata(&metadata)
}

fn snapshot_metadata(metadata: &fs::Metadata) -> Result<FileSnapshot, SnapshotError> {
    if !metadata.file_type().is_file() {
        return Err(SnapshotError::NotRegularFile);
    }

    Ok(FileSnapshot {
        identity: FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        },
        size: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
    })
}

fn open_read_only(path: &Path) -> Result<fs::File, SnapshotError> {
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| SnapshotError::Unreadable)
}

fn snapshot_open_file(file: &fs::File) -> Result<FileSnapshot, SnapshotError> {
    let metadata = file.metadata().map_err(|_| SnapshotError::Unreadable)?;
    snapshot_metadata(&metadata)
}

fn read_open_file(file: &mut fs::File) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub fn capture_after_second_scan(
    path: &Path,
    first: FileSnapshot,
    tombstoned_hashes: &BTreeSet<String>,
) -> CaptureOutcome {
    capture_after_second_scan_with_reader(path, first, tombstoned_hashes, |file| {
        read_open_file(file)
    })
}

pub fn capture_after_second_scan_with_reader<F>(
    path: &Path,
    first: FileSnapshot,
    tombstoned_hashes: &BTreeSet<String>,
    read: F,
) -> CaptureOutcome
where
    F: FnOnce(&mut fs::File) -> io::Result<Vec<u8>>,
{
    let before_read = match snapshot(path) {
        Ok(snapshot) => snapshot,
        Err(_) => return CaptureOutcome::Deferred(DeferReason::Unreadable),
    };
    if before_read != first {
        return CaptureOutcome::Deferred(DeferReason::ChangedBeforeRead);
    }

    let mut file = match open_read_only(path) {
        Ok(file) => file,
        Err(_) => return CaptureOutcome::Deferred(DeferReason::Unreadable),
    };
    let opened = match snapshot_open_file(&file) {
        Ok(snapshot) => snapshot,
        Err(_) => return CaptureOutcome::Deferred(DeferReason::Unreadable),
    };
    if opened != before_read {
        return CaptureOutcome::Deferred(DeferReason::ChangedBeforeRead);
    }

    let bytes = match read(&mut file) {
        Ok(bytes) => bytes,
        Err(_) => return CaptureOutcome::Deferred(DeferReason::Unreadable),
    };

    match snapshot_open_file(&file) {
        Ok(after_read) if after_read == opened => {}
        Ok(_) => return CaptureOutcome::Deferred(DeferReason::ChangedAfterRead),
        Err(_) => return CaptureOutcome::Deferred(DeferReason::Unreadable),
    }

    match snapshot(path) {
        Ok(after_read) if after_read == opened => {}
        Ok(_) => return CaptureOutcome::Deferred(DeferReason::ChangedAfterRead),
        Err(_) => return CaptureOutcome::Deferred(DeferReason::Unreadable),
    }

    let sha256 = sha256_hex(&bytes);
    if tombstoned_hashes.contains(&sha256) {
        return CaptureOutcome::Suppressed { sha256 };
    }

    CaptureOutcome::Captured(CapturedFile { bytes, sha256 })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::BTreeSet,
        fs,
        io::{self, Read},
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        thread,
        time::{Duration, SystemTime},
    };

    static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let suffix = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "cancan-local-inbox-readiness-{}-{suffix}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create synthetic inbox");
            Self { path }
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.path.join(name);
            fs::write(&path, bytes).expect("write synthetic evidence");
            path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn second_scan() {
        thread::sleep(Duration::from_millis(20));
    }

    #[test]
    fn stable_local_file_is_captured_without_mutating_source_bytes_or_metadata() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"synthetic local statement");
        let first = snapshot(&file).expect("first scan");
        let source_bytes = fs::read(&file).expect("source bytes before capture");

        second_scan();
        let outcome = capture_after_second_scan(&file, first.clone(), &BTreeSet::new());

        let CaptureOutcome::Captured(captured) = outcome else {
            panic!("stable source was not captured");
        };
        assert_eq!(captured.bytes, source_bytes);
        assert_eq!(
            fs::read(&file).expect("source bytes after capture"),
            source_bytes
        );
        assert_eq!(
            snapshot(&file).expect("source metadata after capture"),
            first
        );
    }

    #[test]
    fn changed_size_before_read_is_deferred() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"first");
        let first = snapshot(&file).expect("first scan");
        fs::write(&file, b"larger replacement content").expect("change source before read");
        assert_ne!(snapshot(&file).expect("changed scan").size, first.size);

        second_scan();
        assert_eq!(
            capture_after_second_scan(&file, first, &BTreeSet::new()),
            CaptureOutcome::Deferred(DeferReason::ChangedBeforeRead)
        );
    }

    #[test]
    fn changed_mtime_before_read_is_deferred() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"same-size source");
        let first = snapshot(&file).expect("first scan");
        fs::File::open(&file)
            .expect("open source")
            .set_times(
                fs::FileTimes::new().set_modified(SystemTime::now() + Duration::from_secs(60)),
            )
            .expect("change source modification time");
        let changed = snapshot(&file).expect("changed scan");
        assert_eq!(changed.identity, first.identity);
        assert_eq!(changed.size, first.size);
        assert_ne!(changed, first);

        second_scan();
        assert_eq!(
            capture_after_second_scan(&file, first, &BTreeSet::new()),
            CaptureOutcome::Deferred(DeferReason::ChangedBeforeRead)
        );
    }

    #[test]
    fn changed_device_identity_before_read_is_deferred() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"same-source");
        let mut first = snapshot(&file).expect("first scan");
        first.identity.device ^= 1;

        second_scan();
        assert_eq!(
            capture_after_second_scan(&file, first, &BTreeSet::new()),
            CaptureOutcome::Deferred(DeferReason::ChangedBeforeRead)
        );
    }

    #[test]
    fn replaced_file_identity_before_read_is_deferred() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"same-length-source");
        let first = snapshot(&file).expect("first scan");
        let replacement = inbox.file("replacement.pdf", b"same-length-source");
        fs::rename(&replacement, &file).expect("atomically replace source");
        assert_ne!(
            snapshot(&file).expect("replacement scan").identity,
            first.identity
        );

        second_scan();
        assert_eq!(
            capture_after_second_scan(&file, first, &BTreeSet::new()),
            CaptureOutcome::Deferred(DeferReason::ChangedBeforeRead)
        );
    }

    #[test]
    fn change_after_read_discards_bytes_and_defers() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"source before read");
        let first = snapshot(&file).expect("first scan");

        second_scan();
        let source = file.clone();
        let outcome =
            capture_after_second_scan_with_reader(&file, first, &BTreeSet::new(), |handle| {
                let mut bytes = Vec::new();
                handle.read_to_end(&mut bytes)?;
                fs::write(&source, b"source changed after read")?;
                Ok(bytes)
            });

        assert_eq!(
            outcome,
            CaptureOutcome::Deferred(DeferReason::ChangedAfterRead)
        );
    }

    #[test]
    fn fresh_rescan_captures_a_file_after_an_earlier_unstable_scan() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"first scan bytes");
        let stale_first = snapshot(&file).expect("first scan");
        fs::write(&file, b"stable after rescan").expect("change source");

        second_scan();
        assert_eq!(
            capture_after_second_scan(&file, stale_first, &BTreeSet::new()),
            CaptureOutcome::Deferred(DeferReason::ChangedBeforeRead)
        );

        let rescan_first = snapshot(&file).expect("fresh scan after unstable attempt");
        second_scan();
        assert!(matches!(
            capture_after_second_scan(&file, rescan_first, &BTreeSet::new()),
            CaptureOutcome::Captured(_)
        ));
    }

    #[test]
    fn tombstoned_hash_is_suppressed_without_source_mutation() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"synthetic deleted statement");
        let first = snapshot(&file).expect("first scan");
        second_scan();
        let CaptureOutcome::Captured(captured) =
            capture_after_second_scan(&file, first, &BTreeSet::new())
        else {
            panic!("first capture");
        };
        let source_bytes = fs::read(&file).expect("source bytes before suppression");
        let source_metadata = snapshot(&file).expect("source metadata before suppression");
        let tombstones = BTreeSet::from([captured.sha256.clone()]);

        second_scan();
        assert_eq!(
            capture_after_second_scan(&file, source_metadata.clone(), &tombstones),
            CaptureOutcome::Suppressed {
                sha256: captured.sha256
            }
        );
        assert_eq!(
            fs::read(&file).expect("source bytes after suppression"),
            source_bytes
        );
        assert_eq!(
            snapshot(&file).expect("source metadata after suppression"),
            source_metadata
        );
    }

    #[test]
    fn missing_source_after_first_scan_fails_closed_as_deferred() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"source that disappears");
        let first = snapshot(&file).expect("first scan");
        fs::remove_file(&file).expect("remove source");

        second_scan();
        assert_eq!(
            capture_after_second_scan(&file, first, &BTreeSet::new()),
            CaptureOutcome::Deferred(DeferReason::Unreadable)
        );
    }

    #[test]
    fn injected_provider_read_error_fails_closed_as_deferred() {
        let inbox = TestDirectory::new();
        let file = inbox.file("statement.pdf", b"read-error fixture");
        let first = snapshot(&file).expect("first scan");

        second_scan();
        let outcome = capture_after_second_scan_with_reader(&file, first, &BTreeSet::new(), |_| {
            Err(io::Error::other("synthetic provider unavailable"))
        });

        assert_eq!(outcome, CaptureOutcome::Deferred(DeferReason::Unreadable));
    }

    #[test]
    fn scanner_never_follows_a_symlink() {
        let inbox = TestDirectory::new();
        let target = inbox.file("target.pdf", b"synthetic target");
        let link = inbox.path.join("linked.pdf");
        std::os::unix::fs::symlink(&target, &link).expect("create synthetic symlink");

        assert_eq!(snapshot(&link), Err(SnapshotError::NotRegularFile));
    }

    #[test]
    fn symlink_swap_before_open_is_rejected_without_following_the_target() {
        let inbox = TestDirectory::new();
        let target = inbox.file("target.pdf", b"synthetic target");
        let candidate = inbox.file("statement.pdf", b"synthetic candidate");
        let first = snapshot(&candidate).expect("first scan");
        fs::remove_file(&candidate).expect("remove candidate before open");
        std::os::unix::fs::symlink(&target, &candidate).expect("replace candidate with symlink");

        assert!(matches!(
            open_read_only(&candidate),
            Err(SnapshotError::Unreadable)
        ));
        assert_eq!(
            capture_after_second_scan(&candidate, first, &BTreeSet::new()),
            CaptureOutcome::Deferred(DeferReason::Unreadable)
        );
    }
}
