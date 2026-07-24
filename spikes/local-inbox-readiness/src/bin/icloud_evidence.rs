use cancan_local_inbox_readiness::{
    CaptureOutcome, FileIdentity, FileSnapshot, capture_after_second_scan, snapshot,
};
use std::{
    collections::BTreeSet,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

const ICLOUD_RELATIVE_ROOT: &str = "Library/Mobile Documents/com~apple~CloudDocs/Cancan";
const RUN_DIRECTORY_PREFIX: &str = ".cancan-local-inbox-evidence-";
const OWNER_MARKER: &str = ".cancan-local-inbox-evidence-owner";

struct CreatedRun {
    path: PathBuf,
    token: String,
}

struct SavedSnapshot {
    snapshot: FileSnapshot,
    first_process_id: u32,
}

fn main() {
    if let Err(message) = run(env::args_os().skip(1).collect()) {
        eprintln!("icloud-evidence: {message}");
        process::exit(2);
    }
}

fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), String> {
    let Some((command, arguments)) = arguments.split_first() else {
        return Err(usage());
    };
    let command = command
        .to_str()
        .ok_or_else(|| "command is not valid UTF-8".to_owned())?;

    match command {
        "create" => {
            let [root] = arguments else {
                return Err(usage());
            };
            let expected_root = expected_icloud_root()?;
            let root = validate_evidence_root(Path::new(root), &expected_root)?;
            let created = create_run_directory(&root)?;
            println!("run-directory={}", created.path.display());
            println!("run-token={}", created.token);
        }
        "snapshot" => {
            let [file, state_file] = arguments else {
                return Err(usage());
            };
            let file = Path::new(file);
            let state_file = Path::new(state_file);
            let first = snapshot(file)
                .map_err(|error| format!("snapshot {}: {error:?}", file.display()))?;
            save_snapshot(state_file, &first)?;
            println!("process-id={}", process::id());
        }
        "capture" => {
            let [file, state_file] = arguments else {
                return Err(usage());
            };
            let file = Path::new(file);
            let saved = load_snapshot(Path::new(state_file))?;
            let outcome = capture_after_second_scan(file, saved.snapshot.clone(), &BTreeSet::new());
            println!("first-process-id={}", saved.first_process_id);
            println!("process-id={}", process::id());
            match outcome {
                CaptureOutcome::Captured(captured) => {
                    println!("outcome=captured");
                    println!("captured-sha256={}", captured.sha256);
                    println!(
                        "source-snapshot-preserved={}",
                        snapshot(file).is_ok_and(|after| after == saved.snapshot)
                    );
                }
                CaptureOutcome::Deferred(reason) => {
                    println!("outcome=deferred-{}", defer_reason_name(reason));
                    println!("source-snapshot-preserved=false");
                }
                CaptureOutcome::Suppressed { sha256 } => {
                    println!("outcome=suppressed");
                    println!("captured-sha256={sha256}");
                    println!(
                        "source-snapshot-preserved={}",
                        snapshot(file).is_ok_and(|after| after == saved.snapshot)
                    );
                }
            }
        }
        "cleanup" => {
            let [run_directory, token] = arguments else {
                return Err(usage());
            };
            let expected_root = expected_icloud_root()?;
            cleanup_run_directory(
                Path::new(run_directory),
                &token.to_string_lossy(),
                &expected_root,
            )?;
            println!("cleanup=removed");
        }
        _ => return Err(usage()),
    }

    Ok(())
}

fn usage() -> String {
    "usage: icloud-evidence create <iCloud-root> | snapshot <file> <state-file> | capture <file> <state-file> | cleanup <run-directory> <run-token>".to_owned()
}

fn expected_icloud_root() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or_else(|| "HOME is not set".to_owned())?;
    Ok(PathBuf::from(home).join(ICLOUD_RELATIVE_ROOT))
}

fn validate_evidence_root(requested_root: &Path, expected_root: &Path) -> Result<PathBuf, String> {
    let expected = fs::canonicalize(expected_root).map_err(|error| {
        format!(
            "canonicalize expected iCloud root {}: {error}",
            expected_root.display()
        )
    })?;
    let requested = fs::canonicalize(requested_root).map_err(|error| {
        format!(
            "canonicalize requested iCloud root {}: {error}",
            requested_root.display()
        )
    })?;
    let metadata = fs::symlink_metadata(&requested).map_err(|error| {
        format!(
            "stat requested iCloud root {}: {error}",
            requested.display()
        )
    })?;

    if requested != expected || !metadata.file_type().is_dir() {
        return Err(format!(
            "requested root {} is not the exact authorized iCloud root {}",
            requested.display(),
            expected.display()
        ));
    }

    Ok(requested)
}

fn create_run_directory(root: &Path) -> Result<CreatedRun, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("read system time: {error}"))?
        .as_nanos();
    for attempt in 0..100 {
        let token = format!("{timestamp}-{}-{attempt}", process::id());
        let path = root.join(format!("{RUN_DIRECTORY_PREFIX}{token}"));
        match fs::create_dir(&path) {
            Ok(()) => {
                let marker = path.join(OWNER_MARKER);
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&marker)
                    .and_then(|mut file| file.write_all(token.as_bytes()))
                    .map_err(|error| format!("write owner marker {}: {error}", marker.display()))?;
                return Ok(CreatedRun { path, token });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "create synthetic evidence directory {}: {error}",
                    path.display()
                ));
            }
        }
    }

    Err("could not allocate a unique synthetic evidence directory".to_owned())
}

fn cleanup_run_directory(
    run_directory: &Path,
    token: &str,
    authorized_root: &Path,
) -> Result<(), String> {
    let parent = run_directory
        .parent()
        .ok_or_else(|| "run directory has no parent".to_owned())?;
    let root = validate_evidence_root(parent, authorized_root)?;
    let metadata = fs::symlink_metadata(run_directory)
        .map_err(|error| format!("stat run directory {}: {error}", run_directory.display()))?;
    let name = run_directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "run directory name is not valid UTF-8".to_owned())?;
    if !metadata.file_type().is_dir() || !name.starts_with(RUN_DIRECTORY_PREFIX) {
        return Err(
            "refusing to clean a directory that is not a synthetic evidence run".to_owned(),
        );
    }

    let run_directory = fs::canonicalize(run_directory).map_err(|error| {
        format!(
            "canonicalize run directory {}: {error}",
            run_directory.display()
        )
    })?;
    if run_directory.parent() != Some(root.as_path()) {
        return Err("refusing to clean a directory outside the exact authorized root".to_owned());
    }

    let marker = run_directory.join(OWNER_MARKER);
    let marker_metadata = fs::symlink_metadata(&marker)
        .map_err(|error| format!("stat owner marker {}: {error}", marker.display()))?;
    if !marker_metadata.file_type().is_file() {
        return Err("refusing to clean a run directory without a regular owner marker".to_owned());
    }
    let marker_token = fs::read_to_string(&marker)
        .map_err(|error| format!("read owner marker {}: {error}", marker.display()))?;
    if marker_token != token {
        return Err("refusing to clean a run directory without this run token".to_owned());
    }

    fs::remove_dir_all(&run_directory).map_err(|error| {
        format!(
            "remove exact synthetic evidence directory {}: {error}",
            run_directory.display()
        )
    })
}

fn save_snapshot(state_file: &Path, snapshot: &FileSnapshot) -> Result<(), String> {
    let mut state_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(state_file)
        .map_err(|error| format!("create snapshot state {}: {error}", state_file.display()))?;
    writeln!(
        state_file,
        "{} {} {} {} {} {}",
        snapshot.identity.device,
        snapshot.identity.inode,
        snapshot.size,
        snapshot.modified_seconds,
        snapshot.modified_nanoseconds,
        process::id()
    )
    .map_err(|error| format!("write snapshot state: {error}"))
}

fn load_snapshot(state_file: &Path) -> Result<SavedSnapshot, String> {
    let content = fs::read_to_string(state_file)
        .map_err(|error| format!("read snapshot state {}: {error}", state_file.display()))?;
    let mut values = content.split_whitespace();

    Ok(SavedSnapshot {
        snapshot: FileSnapshot {
            identity: FileIdentity {
                device: next_state_value(&mut values, "device")?,
                inode: next_state_value(&mut values, "inode")?,
            },
            size: next_state_value(&mut values, "size")?,
            modified_seconds: next_state_value(&mut values, "modified-seconds")?,
            modified_nanoseconds: next_state_value(&mut values, "modified-nanoseconds")?,
        },
        first_process_id: next_state_value(&mut values, "first-process-id")?,
    })
}

fn next_state_value<'a, T>(
    values: &mut impl Iterator<Item = &'a str>,
    key: &str,
) -> Result<T, String>
where
    T: std::str::FromStr,
{
    values
        .next()
        .ok_or_else(|| format!("snapshot state is missing {key}"))?
        .parse()
        .map_err(|_| format!("snapshot state has an invalid {key}"))
}

fn defer_reason_name(reason: cancan_local_inbox_readiness::DeferReason) -> &'static str {
    match reason {
        cancan_local_inbox_readiness::DeferReason::ChangedBeforeRead => "changed-before-read",
        cancan_local_inbox_readiness::DeferReason::ChangedAfterRead => "changed-after-read",
        cancan_local_inbox_readiness::DeferReason::Unreadable => "unreadable",
    }
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

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let suffix = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "cancan-icloud-evidence-contract-{}-{suffix}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
            Self { path }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn runner_accepts_only_the_exact_authorized_root() {
        let test = TestDirectory::new();
        let parent = test.path.join("com~apple~CloudDocs");
        let authorized_root = parent.join("Cancan");
        fs::create_dir(&parent).expect("create synthetic provider parent");
        fs::create_dir(&authorized_root).expect("create synthetic authorized root");

        assert_eq!(
            validate_evidence_root(&authorized_root, &authorized_root)
                .expect("exact root is accepted"),
            fs::canonicalize(&authorized_root).expect("canonical authorized root")
        );
        assert!(validate_evidence_root(&parent, &authorized_root).is_err());
        assert!(validate_evidence_root(&authorized_root.join("nested"), &authorized_root).is_err());
    }

    #[test]
    fn cleanup_requires_the_created_directory_and_its_token() {
        let test = TestDirectory::new();
        let authorized_root = test.path.join("Cancan");
        fs::create_dir(&authorized_root).expect("create synthetic authorized root");
        let created = create_run_directory(&authorized_root).expect("create owned run directory");
        let unrelated = authorized_root.join("ordinary-user-directory");
        fs::create_dir(&unrelated).expect("create unrelated directory");

        assert!(cleanup_run_directory(&unrelated, &created.token, &authorized_root).is_err());
        assert!(cleanup_run_directory(&created.path, "wrong-token", &authorized_root).is_err());
        cleanup_run_directory(&created.path, &created.token, &authorized_root)
            .expect("cleanup created synthetic directory");
        assert!(!created.path.exists());
    }
}
