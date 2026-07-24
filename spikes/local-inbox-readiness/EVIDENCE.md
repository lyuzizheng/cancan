# Local Inbox readiness evidence

Status: investigating on 2026-07-25

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

On the evidence machine, this passed without inspecting iCloud Drive:

```text
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked: 12 protocol tests and 2 runner safety/CLI-contract tests passed
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

The default gate never accesses iCloud Drive. A separate opt-in run used the
user-created root
`$HOME/Library/Mobile Documents/com~apple~CloudDocs/Cancan`. It created only a
unique synthetic child, did not enumerate real files, and printed
`cleanup=removed` after each run.

The repeatable command is:

```text
cd spikes/local-inbox-readiness
CANCAN_ICLOUD_EVIDENCE_ROOT="$HOME/Library/Mobile Documents/com~apple~CloudDocs/Cancan" scripts/run-icloud-evidence.sh
```

The live run established the following limited facts for its synthetic files:

| Gate | Evidence |
| --- | --- |
| Separate CLI-process rescan | `snapshot` and `capture` printed distinct process IDs; stable capture returned bytes, preserved the original synthetic bytes, and preserved identity/size/mtime. This is a CLI process boundary, not an app restart lifecycle proof. |
| Changing source | A synthetic source changed between those processes returned `deferred-changed-before-read` rather than captured bytes. |
| Upload-ready before eviction | The supported URL resource values reported `uploaded=true` and `uploading=false` within the runner's bounded 15-attempt poll. |
| Real placeholder request | `/usr/bin/brctl evict` returned exit 0, then `ubiquitousItemDownloadingStatus` was `not-downloaded`; `/usr/bin/brctl download` later returned exit 0. |
| Actual defer | The snapshot immediately before Rust capture still reported `not-downloaded`. That capture returned `deferred-changed-after-read` and did not return bytes. |
| Hydration boundary | Immediately after that Rust capture, the supported status was `current`. The run records a status transition during capture; it does not attribute the transition to one exact operation, and it must not assume Rust open/read leaves a placeholder offline. |

The status probe reads `URLResourceValues.ubiquitousItemDownloadingStatus`, not
the deprecated downloaded boolean. It is a disposable evidence helper, not a
production Foundation bridge.

The observed defer proves that the handle-bound post-read check can withhold
bytes in this exercised iCloud path. It does **not** prove a pure Rust
open/read protocol can defer a placeholder before hydration. The status
transition leaves the `Local Inbox filesystem readiness` blocker in place: a
future production design needs a native downloading-status preflight before
Rust opens/reads a candidate, plus its own deterministic tests. This spike
does not implement that preflight, a watcher, a Vault capture, a database, a
job, UI, or provider connector.

No settle interval is selected. The 15-attempt upload poll and 10-second
`brctl` limits bound this evidence command only; they do not measure or choose
a production settle delay. The injected read/provider-error test likewise
remains only fail-closed control-flow evidence, not a general iCloud offline or
provider-error guarantee.
