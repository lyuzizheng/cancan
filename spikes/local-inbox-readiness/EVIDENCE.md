# Local Inbox readiness evidence

Status: completed on 2026-07-25

This disposable spike selects only the local/iCloud candidate-readiness
protocol for the future `local-inbox-automation` slice. It does not implement
folder authorization/bookmarks, a watcher, Vault capture, a database, jobs,
UI, a cloud-drive connector, or any production Rust/Foundation bridge.

## Selected protocol

- A regular ordinary-local candidate is ready when Foundation reports
  `isUbiquitousItem != true` (Foundation returns `nil` for the exercised local
  file).
- An iCloud candidate is ready only when
  `URLResourceValues.ubiquitousItemDownloadingStatus` is `current` and it is
  not downloading. `notDownloaded`, `downloaded` (which may be stale), nil or
  unknown status, downloading, and a resource-value/provider error defer
  before Rust opens or reads the candidate.
- A candidate needs matching Unix `(device, inode)`, size, and modification
  seconds/nanoseconds across two scans separated by a fixed two-second settle
  interval. This is a capture-protocol constant, not a user setting.
- Rust opens read-only with `O_NOFOLLOW`, validates the opened handle before
  reading, re-stats it after reading, and re-snapshots the path. Any changed
  identity, size, or modification time discards bytes and defers.
- SHA-256 is computed only after the stable post-read check. A tombstoned hash
  remains suppressed without modifying the source file.
- Process A may observe a candidate but saves no scan snapshot. A restarted
  process B starts a fresh first scan, waits two seconds, then runs its second
  scan/capture.

## Deterministic local gate

```text
cd spikes/local-inbox-readiness
scripts/verify.sh
```

The default gate never accesses iCloud Drive. On the evidence machine it
passed `cargo fmt --check`, clippy, 12 Rust capture-protocol tests, three Rust
CLI/safety tests, the Foundation mapping self-test, and the synthetic local
runner.

The runner output established:

| Gate | Evidence |
| --- | --- |
| Foundation mapping | A regular local file is `ready`; the deterministic helper maps iCloud `current` to ready and `notDownloaded`, `downloaded`, downloading, nil/unknown status, and provider failure to defer. |
| Fresh restart lifecycle | Process A printed `phase=observe-only` and `observation-snapshot-persisted=false`; process B had a different PID, printed `restart-snapshot-imported=false`, made its own fresh first scan, waited at least 2,000 ms, and captured. |
| No snapshot hand-off | The runner verifies no `*.snapshot` exists after process A and the CLI no longer exposes the old persisted-snapshot command path. |
| Continuous write | While a writer changed the source every 250 ms for four seconds, a two-second candidate attempt returned `deferred-changed-before-read` while the writer was still live. After it stopped, a new fresh attempt waited two seconds and captured. |
| Read containment | Existing focused tests retain no-follow, identity/size/mtime rejection before read, post-read re-stat rejection, tombstone suppression, deletion/error deferral, and no source mutation. |

## Opt-in live iCloud gate

On the evidence machine only:

```text
cd spikes/local-inbox-readiness
CANCAN_ICLOUD_EVIDENCE_ROOT="$HOME/Library/Mobile Documents/com~apple~CloudDocs/Cancan" scripts/run-icloud-evidence.sh
```

The script accepts only that exact user-created root. It creates one uniquely
named `.cancan-local-inbox-evidence-*` child, writes only synthetic PDF bytes
inside it, and removes only that exact child after its matching token validates.
Default verification/CI never invokes this command.

The successful 2026-07-25 live output included:

```text
upload-ready=observed
download-status=current
capture-decision=ready
brctl-evict-result=accepted
download-status=not-downloaded
capture-decision=defer
capture-reason=not-downloaded
placeholder-preflight-did-not-hydrate=true
brctl-download-result=accepted
explicit-download-current=true
restart-rescan-fresh-process=true
restart-snapshot-state-transferred=false
iCloud-source-bytes-identity-size-mtime-preserved=true
live-iCloud-evidence-run=passed
cleanup=removed
```

The live runner performed two Foundation-only preflights while the synthetic
candidate remained `not-downloaded`; both returned `defer`, and the second
status remained `not-downloaded`. It did not run Rust snapshot/open/read until
after explicit `brctl download` reached `current`. Then process A observed and
exited without state, process B had a different PID, waited 2,001 ms, captured,
and preserved the source's bytes, identity, size, and modification time.

## Remaining limits

This evidence selects the native preflight-before-open rule and two-second
settle interval. It is not production folder automation. The later
`local-inbox-automation` slice still owns user authorization/bookmarks, native
integration, watcher wake-up hints, scheduled rescans, Vault/job routing,
document handling, duplicate/tombstone integration, source/account resolution,
coverage prompts, and UI states.
