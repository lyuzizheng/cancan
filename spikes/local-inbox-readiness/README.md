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

This default gate does not inspect or touch iCloud Drive. Its tests create only
synthetic files beneath the operating system temporary directory and remove
their directories afterward. The 20 ms pause in tests only forces separate
scan operations; it is not a proposed production settle interval.

## Opt-in live iCloud evidence

On the evidence machine only, run:

```text
CANCAN_ICLOUD_EVIDENCE_ROOT="$HOME/Library/Mobile Documents/com~apple~CloudDocs/Cancan" scripts/run-icloud-evidence.sh
```

The script rejects every other root. It does not enumerate that root: it asks
the Rust CLI to create one uniquely named `.cancan-local-inbox-evidence-*`
directory, writes only synthetic files inside it, and removes only that exact
directory when its matching run token is present. `snapshot` and `capture` are
separate CLI processes with a persisted synthetic snapshot state.

The opt-in run checks upload state with the supported
`URLResourceValues.ubiquitousItemDownloadingStatus` key, then makes bounded
`/usr/bin/brctl evict` and `download` attempts against only its synthetic file.
The upload poll (15 attempts) and each `brctl` command limit (10 seconds) are
liveness bounds, not a selected settle interval. The Swift status probe is
evidence-only; it does not add a production Foundation bridge.

See [EVIDENCE.md](./EVIDENCE.md) for the observed iCloud result and its
remaining blocker.
