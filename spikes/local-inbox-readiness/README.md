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

The tests create only synthetic files beneath the operating system temporary
directory and remove their directories afterward. The 20 ms pause in tests
only forces separate scan operations; it is not a proposed production settle
interval. See [EVIDENCE.md](./EVIDENCE.md) for what this does and does not
prove.
