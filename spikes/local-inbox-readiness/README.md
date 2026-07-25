# Local Inbox readiness

Disposable macOS filesystem evidence for the `local-inbox-readiness` slice.
It is not imported by the desktop app and does not add watched-folder, Vault,
database, job, UI, or cloud-provider behavior.

The small Rust harness uses the same ownership boundary planned for future
Tauri filesystem access: Rust takes no-follow snapshots, opens the candidate
read-only with no-follow semantics after two matching scans, verifies the
opened handle before and after reading, then verifies the path still names that
same file before returning bytes for Vault capture, a deferred outcome, or a
tombstone suppression outcome.

Run the full spike gate with:

```text
scripts/verify.sh
```

This default gate does not inspect or touch iCloud Drive. Its Rust tests and
Foundation mapping self-test use only synthetic files. The local runner proves
that process A observes without saving a scan snapshot, process B starts from a
fresh first scan, waits the fixed two-second settle interval, then performs the
second-scan/capture. A continuous writer remains deferred until it stops, then
the next fresh two-second capture succeeds. The two-second interval is a
protocol constant, not a user setting or production watcher implementation.

## Opt-in live iCloud evidence

On the evidence machine only, run:

```text
CANCAN_ICLOUD_EVIDENCE_ROOT="$HOME/Library/Mobile Documents/com~apple~CloudDocs/Cancan" scripts/run-icloud-evidence.sh
```

The script rejects every other root. It does not enumerate that root: it asks
the Rust CLI to create one uniquely named `.cancan-local-inbox-evidence-*`
directory, writes only synthetic files inside it, and removes only that exact
directory when its matching run token is present. Process A only observes a
candidate and exits without saving a scan snapshot. Process B starts from a
fresh first scan, waits two seconds, and then captures.

The opt-in run uses `URLResourceValues.ubiquitousItemDownloadingStatus` before
any Rust candidate access. Only a regular local file or an iCloud `current`
candidate is ready; `notDownloaded`, `downloaded`, unknown, downloading, and
provider-error states defer. It checks `not-downloaded` twice after eviction to
prove preflight itself did not hydrate the placeholder, then explicitly
downloads to `current` before the fresh-process two-second capture. The upload
poll (15 attempts) and each `brctl` command limit (10 seconds) are liveness
bounds, not the settle interval. The Swift helper remains spike-only; it does
not add a production Foundation bridge.

See [EVIDENCE.md](./EVIDENCE.md) for the observed iCloud result and remaining
limits.
