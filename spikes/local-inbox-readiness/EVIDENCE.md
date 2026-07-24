# Local Inbox readiness evidence

Status: investigating on 2026-07-24

This disposable spike tests only the pre-scan / settle / read / post-read
protocol that a future Rust/Tauri local-folder boundary can use. It does not
authorize production folder observation, Vault capture, a database schema,
jobs, UI, or a cloud-drive connector.

## Candidate under test

- `symlink_metadata` rejects symlinks and non-regular files before capture.
- A snapshot contains the Unix file identity `(device, inode)`, size, and
  modification seconds/nanoseconds.
- A second snapshot must match the first before a read-only handle is opened
  with `O_NOFOLLOW`; that handle's metadata must also match before reading.
- The handle is re-statted after reading, then the path is re-snapshotted; a
  changed identity, size, or modification time discards the bytes and defers
  the candidate.
- SHA-256 is computed only after a stable post-read snapshot. A supplied
  tombstone hash produces a suppressed outcome rather than a capture.
- Snapshot/read failures return a deferred outcome and do not surface bytes.

## Reproducible commands and result

```text
cd spikes/local-inbox-readiness
scripts/verify.sh
```

On the evidence machine, this passed:

```text
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked: 12 passed
iCloud Drive path is absent or unreadable; live iCloud evidence remains blocked.
```

The run used macOS 26.5.2 (25F84), `arm64`, Rust 1.97.0
(`aarch64-apple-darwin`), and Cargo 1.97.0. Test files are synthetic and live
only in a temporary ordinary local directory.

## What the local filesystem run proved

| Gate | Evidence |
| --- | --- |
| Stable capture | A synthetic regular file has matching first/second/post-read snapshots and returns its bytes and SHA-256. |
| Source preservation | Stable capture and tombstone suppression leave source bytes and identity/size/mtime unchanged. |
| Changing size or mtime | A larger write and a same-size modification-time change between the first and second snapshots are each deferred before any read. |
| File replacement | An atomic same-content replacement has a different `(device, inode)` identity and is deferred before any read. |
| Symlink swap | A candidate replaced by a symlink before open is rejected with `O_NOFOLLOW`; no target bytes are returned. |
| Post-read mutation | A controlled local mutation immediately after reading the opened handle discards the previously read bytes. |
| Fresh rescan recovery | After a stale snapshot is deferred, a fresh independent rescan captures the now-stable local file. The harness deliberately retains no watcher or scan state. |
| Tombstone suppression | A SHA-256 derived from a real local read is supplied as a tombstone and rescanning returns suppression without restoring/capturing bytes. |
| Read failure | A source removed after its first snapshot and an injected read/provider error both defer without bytes. |

The 20 ms pause used between test scans is only a deterministic test ordering
device. It is not evidence for, or a selection of, a production settle
interval. The only supported conclusion so far is that the two-snapshot plus
handle-bound post-read protocol detects the exercised changes on this ordinary
local filesystem. A forged-device snapshot boundary test covers comparison of
the device component; it is not live mount/provider-transition evidence.

Fresh rescan recovery is not app/process-restart lifecycle evidence. The spike
does not model app shutdown, persisted scanner state, or a process restart.

## iCloud and provider boundary

The expected path
`$HOME/Library/Mobile Documents/com~apple~CloudDocs` was absent or unreadable
on the evidence machine. The check is included in `scripts/verify.sh` and does
not turn that absence into a pass for iCloud behavior.

The injected read/provider-error test proves only fail-closed control flow. It
is not actual iCloud Drive, placeholder, sync-delay, offline, or provider-error
evidence. A readable supported iCloud Drive path and a separate live run remain
required before this slice can choose a settle interval, remove the `Local
Inbox filesystem readiness` blocker, or authorize `local-inbox-automation`.
