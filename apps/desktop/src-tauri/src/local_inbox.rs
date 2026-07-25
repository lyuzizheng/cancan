use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
use zeroize::Zeroizing;

#[cfg(target_os = "macos")]
use objc2::{rc::Retained, runtime::Bool};
#[cfg(target_os = "macos")]
use objc2_foundation::{
    NSData, NSNumber, NSString, NSURL, NSURLBookmarkCreationOptions,
    NSURLBookmarkResolutionOptions, NSURLIsRegularFileKey, NSURLIsUbiquitousItemKey,
    NSURLUbiquitousItemDownloadingStatusCurrent, NSURLUbiquitousItemDownloadingStatusKey,
    NSURLUbiquitousItemIsDownloadingKey,
};

pub(crate) const BACKUPS_DIRECTORY_NAME: &str = "Backups";
pub(crate) const INBOX_DIRECTORY_NAME: &str = "Inbox";
pub(crate) const SETTLE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileSnapshot {
    pub(crate) identity: FileIdentity,
    pub(crate) size: u64,
    pub(crate) modified_nanoseconds: i64,
    pub(crate) modified_seconds: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CaptureDeferReason {
    ChangedAfterRead,
    ChangedBeforeRead,
    NativePreflight,
    Unreadable,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CaptureOutcome {
    Captured { sha256: String },
    Deferred(CaptureDeferReason),
    Suppressed { sha256: String },
}

pub(crate) trait NativePreflight {
    fn permits_read(&self, path: &Path) -> bool;
}

pub(crate) struct SystemNativePreflight;

impl NativePreflight for SystemNativePreflight {
    fn permits_read(&self, path: &Path) -> bool {
        foundation_permits_read(path)
    }
}

pub(crate) struct LocalInboxPaths {
    pub(crate) inbox: PathBuf,
}

pub(crate) struct AuthorizedRoot {
    root: PathBuf,
    #[cfg(target_os = "macos")]
    url: Retained<NSURL>,
}

impl AuthorizedRoot {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(target_os = "macos")]
impl Drop for AuthorizedRoot {
    fn drop(&mut self) {
        unsafe { self.url.stopAccessingSecurityScopedResource() };
    }
}

pub(crate) enum BookmarkResolution {
    Active(AuthorizedRoot),
    Stale,
}

pub(crate) fn authorize_root(path: &Path) -> io::Result<(Vec<u8>, AuthorizedRoot)> {
    validate_cancan_root(path)?;
    #[cfg(target_os = "macos")]
    {
        let url = file_url(path)?;
        let bookmark = url
            .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                NSURLBookmarkCreationOptions::WithSecurityScope,
                None,
                None,
            )
            .map_err(|_| io::Error::other("could not create local Inbox bookmark"))?;
        let bytes = nsdata_bytes(&bookmark);
        if !unsafe { url.startAccessingSecurityScopedResource() } {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "could not access selected local Inbox root",
            ));
        }
        Ok((
            bytes,
            AuthorizedRoot {
                root: path.to_path_buf(),
                url,
            },
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "local Inbox authorization requires macOS",
        ))
    }
}

pub(crate) fn resolve_root_bookmark(bookmark: &[u8]) -> io::Result<BookmarkResolution> {
    #[cfg(target_os = "macos")]
    {
        let data =
            unsafe { NSData::dataWithBytes_length(bookmark.as_ptr().cast(), bookmark.len()) };
        let mut stale: Bool = false.into();
        let url = unsafe {
            NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                &data,
                NSURLBookmarkResolutionOptions::WithSecurityScope,
                None,
                &mut stale,
            )
        }
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "local Inbox access expired",
            )
        })?;
        if bool::from(stale) {
            return Ok(BookmarkResolution::Stale);
        }
        let Some(path) = url.path() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "local Inbox bookmark did not resolve to a path",
            ));
        };
        let root = PathBuf::from(path.to_string());
        validate_cancan_root(&root)?;
        if !unsafe { url.startAccessingSecurityScopedResource() } {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "local Inbox access was denied",
            ));
        }
        Ok(BookmarkResolution::Active(AuthorizedRoot { root, url }))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bookmark;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "local Inbox authorization requires macOS",
        ))
    }
}

pub(crate) fn ensure_inbox_paths(root: &Path) -> io::Result<LocalInboxPaths> {
    validate_cancan_root(root)?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "local Inbox root is not a directory",
        ));
    }
    let inbox = root.join(INBOX_DIRECTORY_NAME);
    let backups = root.join(BACKUPS_DIRECTORY_NAME);
    ensure_child_directory(&inbox)?;
    ensure_child_directory(&backups)?;
    Ok(LocalInboxPaths { inbox })
}

fn validate_cancan_root(root: &Path) -> io::Result<()> {
    if root.file_name().and_then(|name| name.to_str()) == Some("Cancan") {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "local Inbox root must be named Cancan",
    ))
}

fn ensure_child_directory(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "local Inbox child conflicts with a non-directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(path),
        Err(error) => Err(error),
    }
}

pub(crate) fn supported_source(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if name.starts_with('.') || name.starts_with('~') {
        return false;
    }
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("pdf" | "csv" | "png" | "jpg" | "jpeg")
    )
}

#[cfg(test)]
pub(crate) fn capture_after_settle<P, F>(
    path: &Path,
    preflight: &P,
    tombstoned_hashes: &BTreeSet<String>,
    persist: F,
) -> CaptureOutcome
where
    P: NativePreflight,
    F: FnOnce(Zeroizing<Vec<u8>>) -> io::Result<()>,
{
    let first = match first_snapshot_after_preflight(path, preflight) {
        Ok(snapshot) => snapshot,
        Err(reason) => return CaptureOutcome::Deferred(reason),
    };
    std::thread::sleep(SETTLE_INTERVAL);
    if !preflight.permits_read(path) {
        return CaptureOutcome::Deferred(CaptureDeferReason::NativePreflight);
    }
    capture_after_second_scan(path, first, tombstoned_hashes, persist)
}

pub(crate) fn first_snapshot_after_preflight<P>(
    path: &Path,
    preflight: &P,
) -> Result<FileSnapshot, CaptureDeferReason>
where
    P: NativePreflight,
{
    if !supported_source(path) {
        return Err(CaptureDeferReason::Unsupported);
    }
    if !preflight.permits_read(path) {
        return Err(CaptureDeferReason::NativePreflight);
    }
    snapshot(path).map_err(|()| CaptureDeferReason::Unreadable)
}

pub(crate) fn capture_after_second_scan<F>(
    path: &Path,
    first: FileSnapshot,
    tombstoned_hashes: &BTreeSet<String>,
    persist: F,
) -> CaptureOutcome
where
    F: FnOnce(Zeroizing<Vec<u8>>) -> io::Result<()>,
{
    let before_read = match snapshot(path) {
        Ok(snapshot) => snapshot,
        Err(()) => return CaptureOutcome::Deferred(CaptureDeferReason::Unreadable),
    };
    if before_read != first {
        return CaptureOutcome::Deferred(CaptureDeferReason::ChangedBeforeRead);
    }
    let mut file = match open_read_only(path) {
        Ok(file) => file,
        Err(()) => return CaptureOutcome::Deferred(CaptureDeferReason::Unreadable),
    };
    let opened = match snapshot_open_file(&file) {
        Ok(snapshot) => snapshot,
        Err(()) => return CaptureOutcome::Deferred(CaptureDeferReason::Unreadable),
    };
    if opened != before_read {
        return CaptureOutcome::Deferred(CaptureDeferReason::ChangedBeforeRead);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    if file.read_to_end(&mut bytes).is_err() {
        return CaptureOutcome::Deferred(CaptureDeferReason::Unreadable);
    }
    if !matches!(snapshot_open_file(&file), Ok(after_read) if after_read == opened)
        || !matches!(snapshot(path), Ok(after_read) if after_read == opened)
    {
        return CaptureOutcome::Deferred(CaptureDeferReason::ChangedAfterRead);
    }
    let sha256 = sha256_hex(&bytes);
    if tombstoned_hashes.contains(&sha256) {
        return CaptureOutcome::Suppressed { sha256 };
    }
    if persist(bytes).is_err() {
        return CaptureOutcome::Deferred(CaptureDeferReason::Unreadable);
    }
    CaptureOutcome::Captured { sha256 }
}

fn snapshot(path: &Path) -> Result<FileSnapshot, ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    snapshot_metadata(&metadata)
}

fn snapshot_open_file(file: &fs::File) -> Result<FileSnapshot, ()> {
    snapshot_metadata(&file.metadata().map_err(|_| ())?)
}

fn snapshot_metadata(metadata: &fs::Metadata) -> Result<FileSnapshot, ()> {
    if !metadata.file_type().is_file() {
        return Err(());
    }
    Ok(FileSnapshot {
        identity: FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        },
        size: metadata.len(),
        modified_nanoseconds: metadata.mtime_nsec(),
        modified_seconds: metadata.mtime(),
    })
}

fn open_read_only(path: &Path) -> Result<fs::File, ()> {
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| ())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(target_os = "macos")]
fn file_url(path: &Path) -> io::Result<Retained<NSURL>> {
    let path = path.to_str().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "local Inbox path is not UTF-8")
    })?;
    Ok(NSURL::fileURLWithPath(&NSString::from_str(path)))
}

#[cfg(target_os = "macos")]
fn nsdata_bytes(data: &NSData) -> Vec<u8> {
    let mut bytes = vec![0; data.length()];
    if !bytes.is_empty() {
        let pointer = std::ptr::NonNull::new(bytes.as_mut_ptr().cast())
            .expect("non-empty Vec has a non-null pointer");
        unsafe { data.getBytes_length(pointer, bytes.len()) };
    }
    bytes
}

#[cfg(target_os = "macos")]
fn foundation_permits_read(path: &Path) -> bool {
    let Some(path) = path.to_str() else {
        return false;
    };
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    let Some(regular_file) = foundation_bool(&url, unsafe { NSURLIsRegularFileKey }) else {
        return false;
    };
    if !regular_file {
        return false;
    }
    let ubiquitous = foundation_bool(&url, unsafe { NSURLIsUbiquitousItemKey });
    if ubiquitous != Some(true) {
        return true;
    }
    if foundation_bool(&url, unsafe { NSURLUbiquitousItemIsDownloadingKey }) == Some(true) {
        return false;
    }
    foundation_status_is_current(&url, unsafe { NSURLUbiquitousItemDownloadingStatusKey })
}

#[cfg(target_os = "macos")]
fn foundation_bool(url: &NSURL, key: &NSString) -> Option<bool> {
    let mut value = None;
    unsafe { url.getResourceValue_forKey_error(&mut value, key) }.ok()?;
    value
        .as_deref()?
        .downcast_ref::<NSNumber>()
        .map(NSNumber::boolValue)
}

#[cfg(target_os = "macos")]
fn foundation_status_is_current(url: &NSURL, key: &NSString) -> bool {
    let mut value = None;
    if unsafe { url.getResourceValue_forKey_error(&mut value, key) }.is_err() {
        return false;
    }
    let Some(status) = value
        .as_deref()
        .and_then(|value| value.downcast_ref::<NSString>())
    else {
        return false;
    };
    *status == *unsafe { NSURLUbiquitousItemDownloadingStatusCurrent }
}

#[cfg(not(target_os = "macos"))]
fn foundation_permits_read(path: &Path) -> bool {
    snapshot(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let suffix = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!(
                    "cancan-local-inbox-{suffix}-{}",
                    std::process::id()
                ))
                .join("Cancan");
            fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, bytes).expect("write test source");
            path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = self.0.parent().map(fs::remove_dir_all);
        }
    }

    struct ReadyPreflight;

    impl NativePreflight for ReadyPreflight {
        fn permits_read(&self, _path: &Path) -> bool {
            true
        }
    }

    struct DeferredPreflight;

    impl NativePreflight for DeferredPreflight {
        fn permits_read(&self, _path: &Path) -> bool {
            false
        }
    }

    #[test]
    fn creates_only_the_inbox_and_backups_children() {
        let root = TestDirectory::new();
        let paths = ensure_inbox_paths(&root.0).expect("prepare children");
        assert!(paths.inbox.is_dir());
        assert!(root.0.join(BACKUPS_DIRECTORY_NAME).is_dir());
        assert_eq!(fs::read_dir(&root.0).expect("root entries").count(), 2);
    }

    #[test]
    fn rejects_a_root_that_is_not_named_cancan() {
        let root = TestDirectory::new();
        assert!(ensure_inbox_paths(root.0.parent().expect("test parent")).is_err());
    }

    #[test]
    fn ignores_hidden_temporary_and_partial_suffix_candidates() {
        for name in [
            ".statement.pdf",
            "~statement.pdf",
            "~$statement.pdf",
            "statement.pdf.partial",
            "statement.csv.tmp",
        ] {
            assert!(!supported_source(Path::new(name)), "{name} must be ignored");
        }
        assert!(supported_source(Path::new("statement.PDF")));
    }

    #[test]
    fn rejects_a_conflicting_child_without_replacing_it() {
        let root = TestDirectory::new();
        fs::write(root.0.join(INBOX_DIRECTORY_NAME), b"not a folder").expect("conflict");
        assert!(ensure_inbox_paths(&root.0).is_err());
        assert_eq!(
            fs::read(root.0.join(INBOX_DIRECTORY_NAME)).expect("conflict remains"),
            b"not a folder"
        );
    }

    #[test]
    fn captures_a_stable_supported_file_without_source_mutation() {
        let root = TestDirectory::new();
        let source = root.file("statement.pdf", b"synthetic statement");
        let before = fs::read(&source).expect("source before");
        let first = snapshot(&source).expect("first snapshot");
        let outcome =
            capture_after_second_scan(&source, first.clone(), &BTreeSet::new(), |_| Ok(()));
        assert!(matches!(outcome, CaptureOutcome::Captured { .. }));
        assert_eq!(fs::read(&source).expect("source after"), before);
        assert_eq!(snapshot(&source).expect("snapshot after"), first);
    }

    #[test]
    fn rejects_a_changed_symlink_or_placeholder_before_persisting() {
        let root = TestDirectory::new();
        let source = root.file("statement.pdf", b"first");
        let first = snapshot(&source).expect("first snapshot");
        fs::write(&source, b"changed before read").expect("change source");
        assert_eq!(
            capture_after_second_scan(&source, first, &BTreeSet::new(), |_| Ok(())),
            CaptureOutcome::Deferred(CaptureDeferReason::ChangedBeforeRead)
        );
        let target = root.file("target.pdf", b"target");
        let link = root.0.join("link.pdf");
        std::os::unix::fs::symlink(&target, &link).expect("create link");
        assert_eq!(snapshot(&link), Err(()));
        assert_eq!(
            capture_after_settle(&link, &ReadyPreflight, &BTreeSet::new(), |_| Ok(())),
            CaptureOutcome::Deferred(CaptureDeferReason::Unreadable)
        );
    }

    #[test]
    fn native_preflight_defers_before_any_file_open() {
        let root = TestDirectory::new();
        let source = root.file("statement.pdf", b"placeholder");
        assert_eq!(
            capture_after_settle(&source, &DeferredPreflight, &BTreeSet::new(), |_| Ok(())),
            CaptureOutcome::Deferred(CaptureDeferReason::NativePreflight)
        );
    }

    #[test]
    fn suppresses_tombstoned_hash_without_persisting_or_changing_source() {
        let root = TestDirectory::new();
        let source = root.file("statement.pdf", b"deleted statement");
        let first = snapshot(&source).expect("first snapshot");
        let sha256 = sha256_hex(&fs::read(&source).expect("source bytes"));
        let before = fs::read(&source).expect("source before");
        let outcome =
            capture_after_second_scan(&source, first, &BTreeSet::from([sha256.clone()]), |_| {
                panic!("tombstone must not persist")
            });
        assert_eq!(outcome, CaptureOutcome::Suppressed { sha256 });
        assert_eq!(fs::read(&source).expect("source after"), before);
    }
}
