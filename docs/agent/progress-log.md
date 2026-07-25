# Progress Log

Use this file to keep future AI coding agents oriented. Add a dated entry whenever product decisions, implementation scope, or architecture assumptions change.

## 2026-07-25

### Completed

- Implemented the non-renderer `review-ledger-backend`: production SQLCipher migration and projections, version-checked review actions, durable batch commit/recovery, deterministic cross-history transfer/repayment matching, typed reversal-backed Undo, presentation-safe Tauri/TypeScript contracts, and focused migration/runtime tests. Kimi still owns `review-ledger-ui`.
- Implemented the first production `local-inbox-backend` checkpoint: device-local Keychain bookmark state, exact `Cancan/Inbox` and `Backups` preparation, native iCloud preflight, one two-second stable scanner, verified-byte Vault capture, dedupe/tombstone suppression, durable ingest/parse/reconcile chaining, trusted statement-period coverage, and presentation-safe Tauri/TypeScript contracts. The slice remains in progress until a production structured-record provider path proves the persisted scan-to-Review/Activity completion gate; Kimi can use the frozen host contract while working on the independent review UI.
- Split `local-inbox-backend` from its Kimi renderer integration so the authorized folder, native scan, shared ingest jobs, and coverage logic can proceed against the completed review host boundary without treating unfinished UI as a business-logic blocker.
- Accepted Kimi Code CLI ownership of customer-facing renderer craft. The component foundation and Figma role are no longer product blockers: Kimi may choose pinned renderer dependencies and use Figma exploratorily or omit it, while canonical specs and production code remain authoritative for finance, evidence, jobs, security, accessibility, and product states.
- Marked `review-ledger-ui` ready and added canonical production boundaries for it and the following `local-inbox-automation` slice. The review and evidence specs now freeze host-owned state transitions, presentation-safe commands, pull-request checkpoints, deterministic tests, and the backend/Kimi handoff; the machine implementation plan only maps their delivery order.
- Kept `local-inbox-automation` dependency-blocked until `review-ledger-ui` completes. It will reuse the production review/job/ingestion boundaries instead of creating a second folder-specific pipeline.
- Preserved `packages/core` as the one pure financial-rule implementation by adding a protocol-separated deterministic mode to the existing bundled Tauri-controlled Node worker; Rust remains the only database/job/audit writer. Matching now uses event-specific signed effects, so transfers require opposite cash effects while credit-card repayment requires both cash and liability to decrease.
- Kept the CanCan-root security-scoped bookmark and enabled state device-local in Keychain and outside Vault backups. Restores cannot silently reactivate a stale path capability and require root authorization on the restored device.
- Accepted the initial automatic-folder contract in [0017](../specs/0017-evidence-documents-source-ux.md): the user authorizes one iCloud Drive `Cancan` root, CanCan only owns its `Inbox` and `Backups` child names, and only `Inbox` is a future ingestion source. Preparing `Backups` is not backup enablement or success. The accepted layout does not implement folder-picker authorization/bookmarks, native preflight, watcher UI, backup engine, schedule, bundle writing, restore, migration, or cleanup of old roots.
- Completed `local-inbox-readiness` with a default synthetic gate and one opt-in real iCloud run. The Foundation helper maps only ordinary local files and iCloud `current` to ready; `notDownloaded`, `downloaded`, unknown, downloading, and provider errors defer. The default runner proves no persisted scan snapshot crosses from observe-only process A to fresh-scan process B, fixed two-second settling, continuous-write deferral, post-read re-stat, and source preservation.
- The live runner accepted only the exact user-created CanCan root and token-cleaned one synthetic child. It observed upload-ready, used `brctl evict` to reach `not-downloaded`, ran two Foundation-only preflights that both deferred while status remained `not-downloaded`, explicitly downloaded to `current`, then captured after a fresh-process two-second settle with source bytes, identity, size, and modification time preserved. This removes only the Local Inbox filesystem-readiness blocker; it does not implement production folder automation.

### Next

- Complete the remaining `local-inbox-backend` persisted scan-to-Review/Activity gate when the production structured-record provider path exists. Kimi may independently integrate `review-ledger-ui`; `local-inbox-automation` remains the final renderer integration over both frozen host contracts.

## 2026-07-24

### Completed

- Merged the mock-only `cloud-ocr-provider-contract` checkpoint. It now selects only a complete dedicated OCR or analyser configuration, builds the fixed one-bounded-PNG OpenAI-compatible request, parses plain transcription text, and redacts credentials and provider bodies from contract errors.
- Merged `cloud-ocr-transport`. The completed checkpoint executes the accepted request through an injected fetch boundary and proves deterministic success plus redacted network, non-2xx, and malformed-response failures. It does not add credential storage/loading, app/runtime wiring, provider-specific parsing, a live request, or an opt-in smoke.
- Reconciled the cumulative #32–#35 review with the shipped manual-evidence path. Protected-PDF unlock notices now match available routing, the executable worker rejects unproduced `document_region` observations, and one shared golden command fixture is asserted by Rust serialization, the TypeScript parser, and the packaged-sidecar smoke.
- Kept OCR fail-closed when a page has no usable native content, while allowing usable non-empty native text to continue through grounding and validation when only supplemental OCR fails. The intended broader OCR-planning contract remains canonical and the current marker-only subset is documented as implementation state.
- Unified PDF password byte validation across PDFKit extraction and Core Graphics rendering, moved image validation out of the file-vault storage primitive, avoided validation-only PNG encoding, composited transparent image evidence onto opaque white, and removed the duplicate source-document row mapper.
- Ran the evidence-only `local-inbox-readiness` Rust spike against synthetic files in an ordinary macOS temporary folder. It passes no-follow handle-bound stable capture, size/mtime/inode mutation rejection, a device-identity comparison boundary, fresh rescan recovery, SHA-256 tombstone suppression, source preservation, and fail-closed local/injected read errors. Fresh rescan recovery does not prove app/process-restart lifecycle. The supported iCloud Drive path is absent or unreadable on this machine, so the result does not choose a settle interval, remove the Local Inbox filesystem-readiness blocker, or authorize production folder behavior.

### Next

- Obtain a separate supported-iCloud-Drive run before removing the `local-inbox-readiness` blocker or selecting a settle interval. Do not add watched-folder production behavior, UI, database, jobs, provider-specific cloud wiring, or downstream automation from the ordinary-local result.

## 2026-07-23

### Completed

- Added production job-scoped extraction bundles for manual PDF/CSV evidence. macOS PDFKit emits one native-text observation per page with one-based pages and zero-based, end-exclusive UTF-16 spans; CSV emits deterministic one-based table cells. The strict sidecar protocol rejects legacy raw-content commands, mismatched bundle metadata, duplicate observations, invalid spans/coordinates/confidence, and document-ID mismatch. Rust-owned observation strings and serialized command bytes are zeroized when their buffers drop; the short-lived sidecar exits after each request, and no extraction bundle is persisted. Verified protected-PDF session passwords now feed the same native extraction/routing path, with a genuinely encrypted text-layer fixture covering unlock, non-ASCII span semantics, and routing.
- Added conditional local macOS Vision revision 3 accurate OCR for PDF pages. PDFKit native extraction always runs first; only empty/whitespace native text or NUL, U+FFFD, and non-layout control markers render the affected page in memory for Vision. Native observations remain intact and separate OCR observations carry page, confidence, Vision engine/version, and transformed top-left unit-square boxes. OCR page pixels, PNG bytes, and intermediate recognized strings use zeroizing Rust-owned buffers. OCR or rendering failure, including a missing Vision result collection, rejects the extraction bundle rather than relabeling OCR as native. Deterministic fake-engine tests prove native text skips Vision, provenance is retained, a non-symmetric lower-left box transforms correctly, and failures fail closed; a non-sensitive synthetic text image invokes the production Vision adapter.
- Added host-picked PNG/JPEG manual evidence. The Rust host verifies the matching signature and ImageIO decode before encrypted storage, keeps SHA-256 dedupe/routing/source export unchanged, and uses one EXIF-upright bounded in-memory PNG for local Vision and the viewer. Image OCR observations retain Vision engine/version and confidence but deliberately expose neither page nor bounding-box coordinates. The renderer receives only re-encoded bounded PNG pixels; no plaintext temporary file or original image bytes cross into the web UI. Focused tests cover PNG/JPEG validation and re-encoding, source-cap rejection, bounded rendering, invalid-image non-persistence, image command routing, sidecar-contract acceptance without location claims, and OCR failure rejection.
- Marked `vault-manual-import` complete against the existing #32-#35 evidence and started the separate `cloud-ocr-provider-contract` slice. Its `packages/ai` checkpoint is only a deterministic OpenAI-compatible Chat Completions request/response contract for one bounded PNG; the canonical parser, onboarding, and testing specs retain all live credential, consent, transport, UI, and smoke-test work for later slices.

### Next

- Finish independent implementation and docs-semantic review for the active mock-only `cloud-ocr-provider-contract` checkpoint. Do not add live credential transport, Keychain/settings/runtime wiring, disclosure UI, or an opt-in smoke test in this slice. The next candidate is the evidence-only `local-inbox-readiness` investigation, which remains blocked from production folder behavior by Local Inbox filesystem readiness; `review-ledger-ui` remains blocked by the component foundation and Figma role.

## 2026-07-22

### Completed

- Added the safe Money Source display/list checkpoint. A general Rust/Tauri command returns only source ID, display name, and source type, while the existing source-ID-scoped document command is connected to the renderer only for the selected source. Routed evidence appears under its configured source, unassigned evidence remains in `Needs attention`, and verified routing feedback may use the safe display name without exposing provider keys, account identity, paths, hashes, encrypted locators, credential state, or secrets. Initial and focus refreshes do not scan every source, and lock-state reconciliation invalidates in-flight document loads before clearing sensitive UI state. Desktop and 390 px browser checks passed with no console warnings or errors; Kimi Code CLI was unavailable in the current environment, so no Kimi-led designer review was claimed.
- Added protected-PDF detection and local unlock to `vault-manual-import`. Core Graphics inspects and verifies the encrypted PDF only inside Rust, `Use once` keeps a zeroizing document password only for the unlocked Vault session, and `Update saved password` can reach Keychain only after that password successfully unlocks the selected document. A security-scoped source chooser exposes only configured Money Source IDs, display names, and saved-password availability; choosing a source scopes the credential but does not assign the document. Locked rows remain visible as `Needs attention` with an `Unlock` action, automatically try the selected source's saved password, and clear password UI/session state on Vault lock without creating an unlocked duplicate. At this checkpoint an unlocked protected statement remained view-only; the later production native-observation checkpoint above superseded that limitation, while malformed PDFs continue to fail closed.
- Added the explicit `Save a copy` source-export checkpoint. A document-ID-only host command warns that the selected copy leaves CanCan's encrypted Vault, then opens the native save picker and writes the original source bytes through a private, synced, same-directory temporary file before atomic rename. Destinations inside the Vault are rejected; cancellation and pre-rename failure leave no partial destination, and the renderer receives only completion or a safe error.
- Added CSV source viewing to `vault-manual-import`. A document-ID-only `preview_source_document` command decrypts CSV evidence in memory inside Rust and returns at most the first 200 lines and 32 KiB of UTF-8 preview plaintext with total counts and a truncation flag. A small CSV file within both caps may appear in full, but the renderer never receives a raw original-file byte payload or unbounded content. The preview preserves leading blank lines, describes byte-capped final lines as potentially partial, reuses the viewer's dialog chrome, points to `Save a copy` for the full file, and clears on close and every Vault lock path.

### Next

- Continue `vault-manual-import` with production native PDF extraction. Keep protected-PDF extraction/routing separate until that in-memory extraction path exists.

## 2026-07-21

### Completed

- Added exclusive process ownership before Vault runtime construction. The desktop holds one OS-backed sibling lock for its lifetime, fails a second process before SQLCipher or file reconciliation, leaves first-run Vault detection unchanged, and has focused exclusion/release regression coverage.
- Added immediate macOS sleep/session-lock Vault locking. AppKit notifications mark the host session system-locked, reject queued unlocks through a monotonic generation, synchronously drop the Rust-only store, and only then notify the renderer. The renderer clears document names and rendered page pixels, rejects stale async completions, and rechecks host state after wake.
- Added the recovery-file save checkpoint without blocking onboarding. The Rust host writes a self-contained versioned `CCREC001` bearer secret outside the Vault, verifies its purpose-3 wrapper, stores only versioned configured state plus the file fingerprint in the Vault, and leaves cancellation or handled save failure unconfigured. The unlocked main surface keeps a responsive `To do` banner and direct save action until both writes succeed.
- Added the source-scoped statement-password storage checkpoint. Vault-unlocked host commands save, replace, and remove one password per Money Source in macOS Keychain; canonical `statement_secret_refs` stores only a deterministic device-local storage key plus recoverable transition status. Unlock and command reconciliation discard unverifiable interrupted saves and finish interrupted deletes without persisting the password or losing the reference needed for cleanup.

### Next

- Completed on 2026-07-22. The broader safe source display/list checkpoints remain separate. Recovery import remains in `backup-release`; keep automatic Inbox/Gmail tombstone suppression in their later owning slices.

## 2026-07-20

### Completed

- Hardened manual and inactivity Vault locking so the renderer invalidates its Vault session, removes document names and rendered page pixels, and shows a non-interactive gate before waiting for the privileged lock command. A failed command now reconciles the actual Vault status and restores the unlocked view only when confirmed; renderer-side reconciliation stops after component teardown.
- Added the tombstone-first `Delete source file` checkpoint with a host-owned destructive confirmation, transactional deletion audit, relationship-preserving tombstone, linked-uncommitted-record review gating, encrypted-blob removal, and startup crash convergence. Missing storage is not mislabeled as a user deletion, and exact-hash Add requires a separate host confirmation before restoring a user-deleted file.
- Added explicit `Remember on this Mac` controls backed by macOS Keychain. The Rust host stores only the 32-byte master key, checks only entry presence during startup, retrieves the secret only after explicit Keychain unlock, preserves unverified credentials after generic Vault-open failures, and keeps password unlock as the safe fallback without exposing secret values to the renderer. Forgetting does not close the current Vault.
- Split application CI into a Linux fast gate for every application pull-request revision and `main` push, plus a path-scoped macOS native gate for native desktop, sidecar/parser, migration, dependency, and toolchain changes. Draft pull requests skip native execution, ready pull requests run it, manual native verification remains available, and both workflows cancel superseded runs.
- Added real `verify:fast` and `verify:native` root commands while preserving `pnpm verify` as the default local application gate. Updated deterministic CI validation and fault injection to protect safety coverage and runner boundaries instead of requiring every application change to use one fixed macOS job.
- Replaced duplicated canonical-file and cumulative-diff copies in implementation handoffs with a shared readiness/source index and repository-inspection commands, while requiring every role to read every indexed canonical source in full from the exact head and record that evidence. Independent roles now receive compact task/evidence handoffs, retain the same source and diff coverage, and batch draft-PR review findings before the final local and CI gates.
- Reused one prepared sidecar across each combined native/local gate while keeping standalone Rust and Tauri build commands self-contained. Added an OS/toolchain/lockfile-keyed Cargo cache for registry, git, and desktop target outputs only; sidecar binaries and app artifacts remain uncached.
- Hardened compact implementation-review packets so they execute real slice validation, reject unknown slices, bind inspection to the exact head plus a content-sensitive working-tree fingerprint, and detect diff-prefixed untracked content in self-tests.

### Next

- Continue `vault-manual-import` with recovery-file, system sleep/lock, statement-password, `Save a copy`, CSV viewing, and safe source display/list checkpoints. Keep automatic Inbox/Gmail tombstone suppression in their later owning slices.

## 2026-07-19

### Completed

- Added the production in-memory PDF viewer checkpoint. A document-ID/page-number Tauri command decrypts only while the Vault is unlocked, renders a bounded page through Core Graphics, returns PNG-encoded pixels plus page position, and rejects missing, unavailable, non-PDF, invalid, and out-of-range requests without exposing original bytes, paths, hashes, or encrypted locators.
- Added responsive desktop/narrow viewer UI with bounded previous/next navigation, Escape/Close behavior, page-pixel removal on close or Vault lock, and stale-render suppression so an in-flight page cannot reopen a closed viewer. Focused Rust and renderer tests cover the command boundary and no-temporary-file invariant; desktop and 390 px visual checks passed.
- Added the fixed MVP fifteen-minute inactivity lock with keyboard, pointer, touch, and wheel activity resets. Successful automatic locking clears document names and rendered pixels through the existing Vault lock path.
- Removed Money Source input from the manual PDF/CSV import command. New evidence enters the encrypted Vault with null source and semantic identity while exact SHA-256 duplicate and restore behavior remains available immediately.
- Added the relationship-preserving migration that makes `source_documents.money_source_id` nullable, retaining its foreign key, source-list index, lifecycle triggers, parse relationships, and foreign-key validation/rollback behavior.
- Added the production single-pass mock sidecar package and a document-ID-only Tauri normalization command. The host clears inherited sidecar environment, keeps source plaintext inside the trusted process boundary, validates the synthetic provider fingerprint, and derives semantic identity without accepting renderer-supplied provider/source/account values.
- Added transactional routing for exactly one configured Money Source and stable provider account IDs. Zero/multiple source matches, unsupported evidence, fingerprint mismatch, missing stable account identity, and archived account matches return typed `Needs attention` outcomes without guessing.
- Added safe unassigned-document listing, renderer-facing TypeScript command contracts, deterministic mock-normalizer tests, migration tests, Rust repository/runtime tests, and a debug Tauri build with the generated Node single-executable sidecar.
- Wired the narrow manual-evidence UI checkpoint to the existing Tauri commands: Vault create/unlock/lock status, host-picked PDF/CSV Add file without source/account preselection, safe import outcomes, unassigned evidence, and document-ID-only routing. The responsive `Precision Vaultpunk — Obsidian Spine + Light Ledger` implementation has explicit loading, locked, unlocked, cancellation, success, `Needs attention`, error, and reduced-motion states.
- Kept routing confirmation intentionally generic. The current safe outcome exposes a `moneySourceId` but no source display/list or source-management API, so the renderer does not invent a named Money Source or source-detail Documents view.

### Next

- Continue `vault-manual-import` with the remaining Keychain, system sleep/lock, statement-password, deletion, recovery-file, `Save a copy`, and CSV viewing checkpoints. A named source display/Documents view waits for safe source display/list and source-management APIs. Keep folder automation, Gmail, mobile intake, and live AI out of this checkpoint.

## 2026-07-18

### Completed

- Reframed acquisition from Gmail-first to inbox-first. Add/Open With, a user-selected local or cloud-synced folder, Gmail send-to-self attachments, and later channels now converge on one encrypted capture, classifier, account resolver, parser, and reconciliation path.
- Accepted capture without source/account preselection. `source_documents.money_source_id` may remain null until trusted provider classification resolves one configured Money Source; the current host command's required source is now explicit remaining work in `vault-manual-import`.
- Accepted bounded canonical Gmail message envelopes for supported no-attachment transaction notifications. They require separate body consent, remain provisional by default, and reconcile with posted statement rows through existing many-to-many match edges instead of creating a second financial event.
- Defined deterministic statement-coverage prompts from provider cadence and accepted periods. Transaction emails do not satisfy coverage; locked or failed statement evidence becomes `Needs attention`, not missing.
- Added an evidence-only `local-inbox-readiness` slice plus `local-inbox-automation` after the core review UI and before Gmail, so the filesystem protocol is executable and watched-folder routing/coverage are proven without OAuth. Kept the future native iOS Share Extension as a thin intake companion behind its own transport/encryption/lifecycle/release feasibility slice.
- Added the stacked host-owned manual-import boundary: Tauri opens the native PDF/CSV picker off the command thread, imports only while the Vault is unlocked, and returns import outcomes or safe per-source document metadata without returning host paths, encrypted locators, hashes, or file bytes.
- Made semantic document identity nullable during initial file capture. The renderer command no longer accepts it; exact-hash deduplication remains immediate, while the trusted classifier/normalizer will set semantic identity and detect byte-different probable statements later.
- Added a relationship-preserving SQLite rebuild migration with explicit foreign-key-off handling, in-transaction foreign-key validation, rollback coverage, and restoration of enforcement after success or failure.

### Next

- Continue `vault-manual-import`: remove required pre-import source selection, connect deterministic classification/normalization and the Money Source/Documents UI, then complete its responsive, empty/loading/error-state, and motion inspection. Do not start watched-folder or Gmail work inside this slice.

## 2026-07-17

### Completed

- Added the first production Vault runtime boundary: Tauri now manages a Rust-only locked/unlocked session and exposes narrow status/create/unlock/lock commands. Password KDF work runs off the command thread, command responses contain only status or stable error codes, and dropping the session drops the SQLCipher store and its zeroizing master-key owner.
- Promoted the accepted `CCENV001` password-wrapper contract from the security evidence into production code, including both fixed Argon2id profiles with zeroized work memory, the exact 750 ms primary-profile selection rule, compatibility fixture hashes, authenticated header parsing, wrong-password rejection, zeroized SQLCipher key strings, and inactive-candidate creation/handled-failure rollback before the Vault directory becomes active. Abrupt pre-activation crash leftovers remain inert and deliberately uncollected until a cross-process-safe ownership/locking protocol exists.
- Kept MVP AI setup direct and user-owned: the user supplies a supported provider endpoint/model and API key. Recorded an optional paid CanCan-hosted AI relay as a post-MVP possibility behind the same provider-adapter/capability boundary, without adding speculative runtime interfaces, accounts, payments, or entitlement storage now.
- Confirmed that saving the recovery file may be deferred during setup as long as the product keeps the missing-recovery state and recovery action visible.
- Completed the design-only Source/Documents exploration without landing a redesigned implementation. Archived the functional baseline, rejected directions, selected `Precision Vaultpunk — Obsidian Spine + Light Ledger` north star, and approved palette under `resources/design/vault-archive-2026-07/`.
- Accepted version-1 visual tokens, Geologica plus limited Martian Mono roles, precision motion timing, and explicit rejection of Material Design 1/2 and generic 2024-2026 vibe-coded AI styling.
- Added the production Rust security/data-integrity suite to the macOS application workflow as authoritative CI work; it remains separate from the default local `pnpm verify` gate.
- Addressed PR review by marking `vault-manual-import` in progress, authenticating and hashing referenced Vault envelopes during startup reconciliation, and keeping disposable spike tests out of the production unit gate while preserving their isolated workflow.

### Next

- Submit the Vault-session command checkpoint without UI. After merge, add host-owned file selection plus import/list commands through this session, then connect deterministic normalization and Source/Documents with responsive and motion inspection.

## 2026-07-16

### Completed

- Started `vault-manual-import` with a rollback-safe backend checkpoint: added the production `CCENV001` file store, source-document lifecycle migration, exact-hash/semantic import outcomes, and focused compatibility/tamper tests.
- Resolved the runtime repository boundary without exposing SQLite to the renderer: `packages/db` owns canonical migrations and portable schema/query tests, while Rust/Tauri owns SQLCipher connections, transactions, typed production repositories, and file/database failure convergence. The normalizer sidecar remains database-free.
- Froze `cancan:database:v1` as the HKDF-SHA-256 database-subkey context and use SQLCipher's raw 256-bit key form, keeping the password KDF out of normal database opens.
- Closed the merged `vault-security-validation` slice after its macOS CI gate and independent reviews passed. Promoted the `CCENV001` version-1 envelope and compatibility hashes, versioned KDF selection, source-scoped Keychain behavior, memory-only PDF rendering, tombstone deletion convergence, and inactive-Vault locator switch into the canonical implementation boundary.
- Removed only the resolved Vault/security evidence blockers. Kept production job idempotency/cancellation tests, real macOS `x86_64`, backup operations, public release, Gmail consent/verification, and security-observability work in their owning implementation slices.
- Marked `vault-manual-import` ready and recorded when user-owned AI and Google credentials become necessary; neither is required for the controlled synthetic/redacted manual-import start.

### Next

- Continue `vault-manual-import` with narrow Tauri commands, mock normalization, and the first Money Source/Documents frontend review checkpoint. Keep Gmail, public OAuth, backup scheduling, and release work out of scope.

## 2026-07-15

### Completed

- Fixed the platform roadmap: Phase 1 supports macOS on both `arm64` and Intel `x86_64`; Windows desktop is Phase 2 and does not block the first macOS release. Existing end-to-end evidence remains `arm64`-only, so real Intel build/runtime/Vault/Keychain/sidecar evidence is now an explicit gate rather than an implied support claim.
- Completed the evidence-lifecycle and Vault-security design grill. In MVP, `source_documents` is both the imported evidence row and encrypted-file registry/tombstone; no separate `vault_files` table is added. Exact-hash re-import reuses or restores that row, while byte-different files under one statement identity remain separate evidence.
- Replaced ambiguous document Archive/Remove behavior with explicit `Delete source file`: remove the current encrypted Vault blob, retain the source-document tombstone and every parse/record/review/ledger/audit relationship, block future auto-commit from deleted evidence, and require explicit reversal for committed corrections.
- Accepted CanCan-only source viewing through in-memory Rust/Tauri page rendering with no plaintext temporary file. `Save a copy` is the separate warned plaintext export to a user-selected path.
- Defined automatic-add behavior: qualified records commit directly when enabled; when disabled, Review begins with no staged records selected and supports subset/all/none submission. Staged removal is an append-only decision over a rebuildable projection; committed Undo appends a reversal event.
- Scoped one optional saved statement-PDF password per Money Source in macOS Keychain, shared by Gmail and manual import, with use-once/update behavior, no password history, no unlocked duplicate, and no backup inclusion.
- Accepted versioned Argon2id wrapper profiles: prefer RFC 9106's 64 MiB, three-pass, four-lane profile inside a 750 ms supported-Mac unlock budget; otherwise use the OWASP 19 MiB, two-pass, one-lane minimum. `Remember on this Mac` bypasses the password KDF on the normal unlock path.
- Accepted restore-to-new-path validation and atomic switch plus a resumable new-device Setup Checklist. Added a bounded `vault-security-validation` evidence slice before production manual import; deterministic security tests and independent code review are required, while a third-party audit is not an MVP release blocker.

### Next

- Execute `vault-security-validation`, then remove only the blockers its reproducible evidence actually resolves before starting `vault-manual-import`.

## 2026-07-14

### Completed

- Replaced the always-full multi-agent loop with consequence-based Fast, Standard, and High-risk paths. Simple PR comments now stay in the root thread with focused checks and PR CI; standard work uses at most one independent role when useful; high-risk work keeps applicable independent review but runs the full relevant local gate once on the final stable diff instead of after every edit.
- Added project-scoped custom agents with pinned model/reasoning assignments: Sol High for complex planning, exploration, document conflicts, redesign, refactoring, performance, and architecture analysis; Terra Max for implementation; Luna Max for testing; and Sol High for independent review. Limited nesting to direct children, kept one production-code writer, and made explorer/reviewer read-only.
- Tightened the development loop around a stable diff: implementer writes code and focused tests, tester independently reports reproducible failures using the narrowest sufficient database/Tauri/UI layer, reviewer judges the stable diff read-only, and findings route back to the implementer before gates repeat.
- Added a deterministic Codex-agent configuration check and harness fault injection so role, model, reasoning, permission, concurrency, and nesting drift fail preflight/CI.
- Added a full-evidence review loop: frozen changes pass all selected-slice gates, preflight, root verification, and triggered harness/UI evidence before independent correctness and critical-cleanup review; every subsequent file change forces a full applicable rerun and cumulative-diff re-review.
- Kept main-agent permissions user/session-owned and removed the implementer's repo-local sandbox default, while retaining read-only explorer/reviewer and workspace-write tester defaults. Parent live permissions may supersede every child default, so no-edit review is also a workflow/independence contract; the repo does not publish a personal full-access policy.
- Simplified parser evidence persistence to one bounded validated `raw_json` source row/object plus `validation_json` per external record. Removed the persistent field-claim graph, dedicated evidence-reference table, and page/row/column indexes; optional location data may stay inside raw JSON as a display hint.
- Made database migrations vertical-slice-owned, reduced the initial schema to the synthetic core, restricted indexes to implemented queries and accepted uniqueness/idempotency rules, and replaced fixed global benchmark cardinalities with focused query-plan/benchmark evidence beside implemented hot paths.
- Simplified account identity to a stable provider account ID when available and one-time-confirmed candidates otherwise. Deferred a separate identifier table, keyed digests, strength levels, and key-version coupling until a real supported provider proves they are needed.
- Retained many-to-many `match_edges` in the synthetic core because linking two bank-side records through one canonical transfer event is a primary product capability. Added bidirectional record/event/sibling navigation to the slice contract and test gates.
- Completed `synthetic-core-flow` with a deterministic structured-proposal validator, bounded raw-record grounding, exact-decimal reconciliation, source-backed opening/closing balance observations, review routing, and one canonical two-account transfer.
- Added the first slice-owned production migration and repository with transaction rollback, commit idempotency, immutable committed events/legs, append-only audit, focused query-plan assertions, and a destructive-reset guard limited to marked test paths.
- Added `pnpm test:synthetic-core` as the focused script-level harness. The integration flow starts from a clean temporary database, reapplies its migration, persists raw and validation JSON, and verifies record-to-event, event-to-record, and sibling transfer navigation without live AI or network access.
- Completed the disposable `document-normalizer-runtime` comparison with one shared fixture, proposal schema, production validator, mock transcript, fixed seven-tool capability surface, adversarial assertions, step/submission budgets, and cancellation. Single-pass, ToolLoopAgent, and Pi Agent Core produced the same accepted proposal; the agent loops required four or five model steps instead of one and showed no accuracy or recovery advantage.
- Selected single-pass structured normalization for initial implementation and recorded ADR 0003. ToolLoopAgent and Pi Agent Core remain unselected until real qualification fixtures justify their extra lifecycle and dependency surface.
- Proved the selected macOS arm64 execution boundary as a pinned Node 24 single executable bundled and controlled by Tauri. The smoke gate covers frozen dependencies, ad-hoc signing, startup, inherited-environment clearing, protocol framing and secret-field rejection, clean shutdown, cancellation, crash isolation, bundle size, and a conservative comparison-workspace license inventory; exact credential delivery/redaction, an artifact-specific SBOM/license inventory, and production signing/notarization remain owning-slice gates.

### Next

- Hold the planned grouped 5-10 question grill before vault/manual-import implementation, focusing on evidence lifecycle and production vault/file security blockers.

## 2026-07-13

### Completed

- Completed `app-foundation`: added the production pnpm workspace, React/Vite desktop shell, Tauri/Rust runtime, empty core/database boundaries, reusable UI package, exact dependency pins, and frozen JavaScript/Rust lockfiles without adding product, network, vault, database, or secret behavior.
- Verified current upstream package releases on 2026-07-13 and pinned React 19.2.7, TypeScript 7.0.2, Vite 8.1.4, Vitest 4.1.10, Tauri CLI 2.11.4, Tauri crate 2.11.5, and tauri-build 2.6.3. The pnpm workspace keeps a seven-day maturity policy with exact/pattern exceptions only for the newly released pinned TypeScript, Vite, and matching Node type packages.
- Added real root `typecheck`, unit-test, Rust fmt/clippy, web-build, Tauri debug-build, and combined `verify` commands. Added a macOS application workflow that uses the pinned toolchain, frozen install, repository preflight, and the same root gate.
- Added application-CI structural gates and fault injections, bringing the harness self-test to 32 detected faults. The gate rejects removal of `pnpm verify`, either setup verification path, unlocked production Cargo resolution, or regression to the deprecated GitHub Actions runtime. Updated setup to verify both the production application and the still-relevant desktop feasibility evidence while keeping the spike's nested pnpm/Cargo lockfiles isolated and frozen.
- Ran test-first UI work from a missing `AppShell` failure to a passing deterministic render test, then passed `pnpm verify`. Playwright desktop and 390px checks showed the neutral local-first shell correctly with zero console errors or warnings after adding its favicon.
- Marked `app-foundation` complete, then resolved the focused parser-evidence contract through explicit user decisions. `synthetic-core-flow` is now ready without inferring production storage, security, or live-provider behavior.
- Added `./scripts/setup-dev.sh` as an idempotent, Homebrew-free, no-`sudo` macOS setup. Apple Silicon is verified end to end; Intel archive routing is deterministically tested but awaits a real hardware run. It checksum-verifies the pinned official Node binary, installs pinned Corepack/pnpm and rustup/Rust with clippy/rustfmt, bootstraps frozen lockfiles, and runs preflight plus production application and desktop feasibility gates.
- Verified the current supported toolchain pins on 2026-07-13: latest-LTS Node.js 24.18.0, plus latest-stable Corepack 0.35.0, pnpm 11.12.0, Rust 1.97.0, and project-local Tauri CLI 2.11.4.
- Added deterministic setup tests for architecture selection, checksum rejection, pin consistency, unsupported OS handling, missing Command Line Tools UX, foreign tool-path preservation, existing-`fnm` coexistence, rustup shell/global-default isolation, and idempotent shell-profile changes; wired them into preflight and docs CI.
- Repeated setup on the current macOS arm64 machine: the cold run completed the full Tauri/SQLCipher gate; later runs converged to the same exact versions, the login shell resolved every pin correctly even with existing `fnm`, and the profile PATH line remained exactly once.
- Repaired stale harness self-test assumptions left from the pre-feasibility slice state and added a fault injection proving setup-version drift is rejected.
- Completed a disposable macOS arm64 desktop feasibility spike: Tauri 2 built, bundled SQLCipher 4.14.0 and FTS5 worked together, wrong database keys failed, authenticated file tampering failed, password and recovery wrappers opened one master key, and macOS Keychain binary-secret write/read/delete passed.
- Accepted ADR 0001 for the Tauri/React/Rust/SQLite package boundary while keeping exact production cryptographic formats, temporary plaintext, cross-platform secret storage, backup/restore, and release signing blocked in their owning specs.
- Fixed two broad-grill checkpoints in the implementation plan: after the synthetic core flow and before vault/manual import, then after the review-ledger UI and before public OAuth/release work.
- Added the public project surface to the implementation sequence: a static Cloudflare Pages landing/privacy/security/docs/download site and GitHub-native Discussions, issue forms, PR, security, and release surfaces.
- Defined the public Gmail direction as a project-owned Desktop OAuth client with separate development/test credentials, local PKCE loopback authorization, and an explicit Google restricted-scope verification track.
- Expanded the classic open-source release plan with signed/notarized CI artifacts, checksums, SBOM/provenance evidence, immutable source tags, approved releases, updater-key custody, and developer account/setup requirements.

- Allowed every financial event type to auto-commit only when the user toggle is on, all record/provider/account gates pass, package-calibrated required fields are very-high confidence, and every affected native-unit reconciliation window closes exactly against source-backed snapshots.
- Kept mixed-statement behavior record-level: semantic-only ambiguities can remain in Review while eligible records commit after full arithmetic closure; missing or uncertain financial fields block the window.
- Finalized account identity fields and resolver order, one-time first-account confirmation, idempotent aliases, and audited rename/merge/archive behavior without rewriting committed ledger legs.
- Added cross-channel duplicate outcomes and an import completion summary that identifies files already present or already archived/removed without deciding deletion semantics.
- Expanded onboarding into a polished animated local-first/vault story with encryption/privacy explanation and a final enabled-capabilities review.
- Added `0018-app-updates-open-source-release.md` for semantic versions, public GitHub Releases, signed CI artifacts, forward compatibility, safe parser delivery, and conditional desktop updater integration.
- Clarified that CanCan has no hosted service/backend: Gmail, BYO AI, optional fixed FX rates, update checks, and future connectors are separately authorized local-client connections; MVP has no analytics or behavioral telemetry.
- Simplified user-facing security to vault password, optional `Remember on this Mac`, and one recovery file. Internal key separation remains hidden, backup adds no second password, and exact cryptographic/restore details move to feasibility validation.
- Added a machine-checked implementation sequence so coding agents load only the selected slice's specs/ADRs/blockers instead of all canonical specs.
- Added shared context and implementation-review packet generators for implementer/tester/reviewer parity, plus deterministic dependency/path/blocker/test/outcome validation.
- Defined extraction and OCR as source-observation producers and made AI normalization mandatory for the single canonical structured proposal. Added smart multimodal and advanced separate-extraction modes, field-level evidence grounding, date-only/timezone rules, and professional separation between source Debit/Credit, signed account-balance delta, and UI signs.
- Defined the optional agentic normalizer as a bounded seven-tool document agent with no coding, shell, arbitrary file/network, database, secret, or ledger capability. Product parser skills remain versioned provider packages and are separate from repo `.agents/skills`.
- Selected Vercel AI SDK ToolLoopAgent as the first runtime candidate and Pi Agent Core as the comparison candidate, while excluding the full Pi Coding Agent. Added a shared deterministic/adversarial/single-pass comparison harness and a blocked Node/Tauri execution-and-packaging evidence slice; an OS sandbox is optional and no user-installed runtime is allowed.

### Next

- Implement the ready `synthetic-core-flow` with deterministic mocked AI output. Then run the document-normalizer runtime/packaging spike and a grouped 5-10 question grill before touching real vault/manual-import behavior.
- Configure public domain/identity/accounts only when their owning slice is reached; resolve license/platform/signing/update-channel details before release automation.

## 2026-07-12

### Completed

- Accepted the AI advisor boundary: a qualified deterministic policy may commit eligible records, while AI confidence alone never grants ledger-write authority.
- Defined immutable committed events, posting versus observation classes, simple event-type invariants, reversal/replacement correction, commit idempotency, and atomic append-only audit.
- Defined source-backed initial balance anchors so first import does not assume zero or invent historical transactions.
- Defined many-to-many record/event allocations and consumer-first progressive disclosure for partial matches.
- Selected SHA-256 for exact file deduplication, separate semantic document/record identity, and versioned reparse/supersede behavior.
- Reframed the UI as a polished, future-facing personal finance and account-record product with presentation-ready views and low user mental load; sharing remains post-MVP.
- Defined Money Sources as user-configured provider roots for editable Gmail discovery, explicit imports, and future product-defined official API connectors; matching documents continue parsing when they reveal multiple child-account candidates.
- Standardized every provider/document parser as a versioned package containing classifier/prompt configuration, deterministic fingerprints and validators, account mapping, fixtures, qualification state, and implementation notes.
- Replaced global AI confidence thresholds with provider-package qualification: 100 labeled record cases, 20 confirmed shadow candidates, zero incorrect eligible outcomes, and qualification reset after parser/prompt/rule changes.
- Kept the broader `clear PDF` auto-commit direction partial until eligible financial event types and first-seen account commit behavior are decided; accepted one simple setting, quiet Recent Activity, reversal-backed Undo, and no per-record notifications.

### Next

- Continue the next unresolved P0 batch from `docs/alignment-temp/alignment-progress.md`.

## 2026-07-10

### Completed

- Changed design grill rounds to consistently ask 5-10 related questions, ordered by risk, instead of falling back to one-question-at-a-time interviews.
- Added `0017-evidence-documents-source-ux.md`: Evidence lives under Money Source detail rather than a dominant standalone Library section.
- Synchronized the spec index, Command Center navigation, current state, reading path, and active alignment register with `0017`.
- Marked unresolved Evidence Remove/reversal and encrypted original-file behavior as explicit implementation blockers instead of inventing product or security answers.
- Replaced the duplicated source-of-truth precedence list with one concern-based source contract in `docs/STRUCTURE.md`.
- Simplified `.agents/` by removing duplicate roles, product rules, placeholder plugin guidance, generic templates, brittle keyword routing, and static priority copies.
- Added a tool-neutral root `AGENTS.md`, bilingual intent routing, deterministic spec/index/link/skill/doc checks, shell syntax validation, and harness fault-injection self-tests.
- Added an independent semantic-review contract and review-packet generator for docs and harness changes.
- Added a docs-harness GitHub Action plus a contract check that protects its triggers, paths, permissions, and required commands.
- Consolidated temporary alignment material into one active unresolved decision register and deleted stale lifecycle/audit/backlog copies.

### Next

- Run the next design round from `docs/alignment-temp/alignment-progress.md`, starting with P0 implementation blockers.
- Wire real app commands into the harness only after application/package scripts exist.

## 2026-07-09

### Completed

- Pulled latest `main` and re-evaluated docs after the old numbered docs layer was removed.
- Updated temporary alignment docs so permanent homes point to canonical specs/current docs instead of deleted numbered docs.
- Reconciled remaining base-currency, default Net Worth, and future crypto/source valuation wording with `0013` and `0014`.
- Updated agent workflow/checklist references to use `docs/specs/` instead of the removed numbered docs layer.
- Created repo-local `.agents/` operating workspace with role routing, workflows, rules, skills, plugin guidance, templates, and deterministic docs/preflight scripts.
- Updated agent reading order and repo-agent workflow spec so future agents use `.agents/` without duplicating product truth from `docs/specs/`.
- Reviewed `.agents/` after creation and fixed grill workflow mismatch: CanCan design grill now asks focused batches of 5-10 questions by default, with one-question mode reserved for security, money correctness, irreversible data shape, or single blocking ambiguity.
- Updated `docs/agent/README.md` to explicitly describe the split between persistent project memory in `docs/agent/` and operating workflows in `.agents/`.
- Aligned job engine and error model in `docs/specs/0015-job-engine-error-model.md`.
- Recorded that jobs should be coarse-grained and user-meaningful, with internal step checkpoints instead of many tiny jobs.
- Added job engine to specs index and agent reading order.
- Aligned testing, fixtures, and AI agent automation gates in `docs/specs/0016-testing-fixtures-agent-gates.md`.
- Added `.gitignore` entries for `fixtures-private/`, local vault/test folders, and common generated output.
- Updated `.agents` testing workflow and testing rules for private/redacted/synthetic fixtures, deterministic LLM mocks, safe test DB reset, and UI visual inspection gates.

### Next

- Align Evidence Library detail UX.
- Decide exact design token values and whether to generate a Figma prototype.
- Align future optional estimated-total/network-valuation policy.

## 2026-07-08

### Completed

- Added implementation spec layer under `docs/specs/`.
- Recorded decisions that the app should use hand-written SQL migrations and typed repositories, not Prisma.
- Added SQL/index/benchmark policy and test database reset requirements.
- Recorded that Vercel AI SDK may be used for AI provider routing and structured generation.
- Defined Gmail rule UX as guided builder plus expert query preview/editing.
- Clarified Command Center layout: left sidebar + main body, polished asset-management/reconciliation app style.
- Added future AI Assistant direction: assistant accesses backend APIs/skills, not raw DB/files/secrets.
- Clarified Review UI should be simple and low-friction, with side-by-side detail only where needed.
- Clarified Money Flow should keep backend graph capability while first UI is chain-first.
- Added expectation that future AI coding agents can implement, test, build, inspect UI via browser/computer-use, reset database, and iterate.
- Added temporary alignment workspace under `docs/alignment-temp/` to break down full product lifecycle questions and track alignment progress.
- Added first-run/onboarding spec with product promise, fixed provider policy, bring-your-own AI direction, startup sequence, and interaction/motion requirements.
- Added design system spec with light-first, modern warm+green, technical/safe/premium direction.
- Added backup/restore/versioning spec with manifest, compatibility rules, and restore behavior.
- Added agentic development workflow spec with required tests, DB reset, build/package, UI inspection, and docs update loop.
- Aligned Gmail integration: Desktop OAuth Authorization Code Flow + PKCE + loopback redirect; local token exchange; Gmail readonly; local Keychain token storage; local encrypted mail cache; polling sync; no CanCan server.
- Recorded Google restricted scope/OAuth verification risk for public release.
- Added markdown Command Center wireframe.
- Added visual design tokens spec for Warm Off-White + green semantic direction.
- Recorded component strategy: prefer Hero UI-style foundation with CanCan wrappers and centralized tokens.
- Added repo agent workflows spec for future `.agents/` rules/workflows/skills after real commands exist.
- Added ledger/assets/valuation spec: fact-based, no market price fetch, no external FX rates, multi-currency first, source snapshots as facts, no tax/lot accounting in MVP, trade table deferred.
- Added Money Overview/source taxonomy spec: no base currency in MVP, no default Net Worth, two-level source model, source/account/instrument taxonomy.
- Added support for password-protected statement PDFs: local unlock, optional secure save, secret references only in SQLite.
- Reworked documentation structure so `docs/specs/` is the canonical implementation source of truth.
- Removed old numbered docs layer (`00-product-vision.md` through `11-open-questions.md`) to prevent duplicate and conflicting product truth.
- Added `docs/STRUCTURE.md` and `docs/specs/README.md`.
- Updated AI agent reading order and source-of-truth hierarchy after cleanup.

### Next

- Align job engine and error model.
- Align testing/fixtures and AI agent automation gates.
- Align Evidence Library detail UX.
- Continue deleting or rewriting temporary alignment files once stable decisions move into specs.

## 2026-07-07

### Completed

- Created initial CanCan docs for product vision, architecture, domain model, AI parser, reconciliation, plugins, security, UI IA, technology decisions, roadmap, open questions, and ADRs.
- Clarified that CanCan is not a budgeting app. It is a local-first financial evidence vault and reconciliation console.
- Recorded the initial Gmail-first MVP hypothesis; the 2026-07-18 inbox-first decision supersedes it after mobile-only bank delivery exposed the need for first-class local and folder intake.
- Moved AI-assisted parsing earlier in the roadmap. AI normalization, duplicate recognition, and link explanation are core capabilities.
- Confirmed first provider scope should include DBS/UOB bank and credit-card statements plus Wise PDF/CSV/export.
- Confirmed user-created Money Sources and sub-accounts are the source of truth for accounts.
- Confirmed ledger events/legs are the canonical model, including transactions, trades, balance snapshots, and valuation snapshots.
- Added `docs/agent/` as a working memory and consistency system for future AI coding agents.
