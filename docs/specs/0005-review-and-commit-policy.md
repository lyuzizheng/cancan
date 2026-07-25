# 0005. Review and Commit Policy Spec

## Goal

Define when records can auto-commit, when they require review, and how reconciliation avoids double-counting.

## Implementation blocker

The auto-commit rule and document-normalizer runtime selection are accepted. The synthetic core flow may exercise the policy with deterministic mocked proposals; live normalization still waits for its upstream Vault/Gmail slices, and auto-commit qualification still requires the fixture and shadow gates in the [implementation sequence](../agent/implementation-slices.md) plus its active alignment blockers. Do not weaken the accepted gates to work around those boundaries.

## MVP user setting

Expose one setting:

```text
Automatically add qualified records = On | Off
```

Recommended default is `On`. Turning it off sends otherwise eligible records to Review.

Submission behavior:

```text
On  -> every record that passes all qualification gates commits without a pre-submit checkbox; all other records remain in Review
Off -> staged records appear in a checkbox list with none selected by default; the user may select any subset, all, or none and choose Add selected
```

Turning automatic addition off does not weaken validation. A selected record still must satisfy the normal commit invariants; selection is user intent, not permission to create an invalid ledger event.

Do not expose confidence numbers, provider allowlists, parser qualification internals, or a three-state policy selector in normal settings. [ADR 0002](../adr/0002-agent-is-advisor-not-ledger-owner.md) owns the accepted AI authority boundary.

Exact SHA-256 duplicate reuse always applies as import idempotency under `0004-parser-contract.md`; it is not an auto-commit setting and cannot be disabled through this toggle.

## Qualified document baseline

A record can be considered for auto-commit only if all are true:

```text
user auto-commit toggle is enabled
evidence arrived through a user-authorized channel or explicit import, and trusted classification resolved a configured Money Source
AI classifier selected the configured supported provider and document type
provider package deterministic fingerprints and schema checks passed
the complete provider/document normalization profile is currently qualified
schema_valid = true
every event-type-required financial field is grounded to a validated raw source record
deterministic_validation_passed = true
account/container identity is resolved for commit
not an exact/probable duplicate, partial allocation, or warning case
every required field has package-calibrated very-high confidence
the affected reconciliation window closes exactly against source-backed snapshots
all event-type-required legs and evidence are present
```

All financial event types may qualify, including repayments, transfers, FX, trades, refunds, withdrawals, interest, and fees. There is no document-type shortcut and no permanently trusted event-type list.

Do not use raw LLM self-reported confidence or one global numeric threshold. Each complete normalization profile calibrates required-field confidence against its labeled qualification fixtures. A record is `very-high confidence` only when every event-type-required field meets that profile's accepted calibration, is grounded to its validated raw source record, and has no competing parse or mapping.

## Exact reconciliation-window gate

A reconciliation window is the source-backed interval between accepted opening and closing observations for every affected account, currency, or instrument scope.

Use exact decimal/native-unit arithmetic:

```text
opening observation
+ all staged posting effects in the window
= closing observation
```

Rules:

```text
every affected native currency and instrument quantity closes independently
cross-source events require all event-type-required sides and evidence
FX and trades require both asset sides plus evidenced fees
no unexplained residual, missing row, duplicate, partial allocation, or uncertain amount/sign/account remains
a statement without sufficient source-backed opening/closing observations cannot auto-commit
the user auto-commit toggle must still be enabled
```

Snapshot closure is necessary but not sufficient: each committed record must also pass its own provider-package, field-confidence, event-invariant, identity, duplicate, and evidence gates.

A source-backed balance, position, or valuation observation may be accepted as a non-posting anchor without a prior opening observation when all of its own gates pass. It establishes a boundary; it does not make earlier postings auto-committable unless a complete opening-to-closing window exists.

## Mixed-document behavior

Auto-commit eligibility is record-level, not all-or-nothing per PDF.

A staged record with verified amount/account-balance delta/account but unresolved semantic classification may participate in arithmetic closure while remaining in Review. The other individually eligible records may auto-commit after the full window closes. A record with uncertain financial fields, ungrounded required evidence, or a missing amount creates a reconciliation gap and blocks auto-commit for that window.

Until every staged record is committed, rejected, or otherwise resolved, the window remains visibly `Needs review`. Do not present the committed subset alone as a fully reconciled/completed window.

## Review UI principle

Review must be simple enough to use, but rich enough to prevent wrong reconciliation.

Recommended interaction:

- inbox/table for batches;
- expandable item detail;
- side-by-side comparison for candidate links;
- compact evidence summary;
- AI explanation and deterministic rule explanation separated;
- one primary action: confirm/reject/edit/link manually.

Do not force every simple item into a heavy wizard.

CanCan is a polished personal finance and account-record product, not professional accounting software. Normal UI should use language such as `Looks related`, `Needs your check`, and `Review details`; allocation, match-edge, and audit terminology belongs behind progressive disclosure.

Auto-committed records appear quietly in Recent Activity with a clear one-click `Undo`. Do not send a notification for every record. A non-disruptive optional daily summary may aggregate automatic additions.

## Match and allocation policy

External records and canonical ledger events have a many-to-many relationship through explicit allocations.

Rules:

```text
partial and one-to-many allocations store amount, unit, and review status
allocations never resize ledger legs implicitly
unmatched remainder remains explicit review work
partial or one-to-many matches block auto-commit
the user may explicitly confirm a fixed canonical event while a visible remainder remains unresolved
the system never invents a synthetic remainder event
```

The default review surface presents one recommended relationship and one primary action. Exact allocations and evidence details are available on expansion, not required as the user's first mental model.

Cross-account transfers are a core relationship, not a future matching extension. The two bank-side records may both link to one canonical transfer event so either record detail can show the other side without duplicating income or spending.

## Transaction email and statement reconciliation

A transaction-notification email and a later statement row are two evidence observations, not two financial events. Preserve both source documents and both external records; never delete one merely because they appear related.

Match in this order:

```text
1. exact grounded provider transaction/reference ID within the same provider and resolved account
2. same resolved account, currency, amount, direction, provider-defined date window, and one unique merchant/reference candidate
3. otherwise Review; never guess between multiple candidates
```

A qualified posted statement row plus its source-backed snapshot window is the normal posting authority. The email remains earlier evidence and may be shown as `Pending from email` until the relationship is proven. Amount changes caused by tips, FX, reversed holds, or other provider behavior do not auto-link unless that provider package has an explicit qualified rule and fixtures.

When the relationship is accepted before commit, existing many-to-many `match_edges` link both external records to one canonical ledger event. If the statement event is already committed when the email arrives, the Gmail slice uses the role-aware schema extension owned by `0002-database-schema.md`: in one transaction, append one confirmed `corroborating_evidence` edge without allocation value, resolve the new external record/review item, and append its audit entry. It creates no new ledger event and cannot change any ledger leg, financial allocation, or prior edge. The current synthetic-core trigger does not yet permit this late insert and must not be bypassed.

With that migration in place, import order is irrelevant: email first and statement first converge to one financial result. The Activity UI folds the evidence into one item with `Email` and `Statement` provenance rather than adding the amount twice.

## Commit, reversal, and audit policy

Uncommitted staged proposals may be edited or removed. `Remove` on a staged record appends a review/domain decision and marks the rebuildable current-state projection as removed; it does not erase the parse run, raw source row, validation history, or audit entry.

Committed correction requests follow the immutable reversal/replacement lifecycle owned by `0013-ledger-assets-valuation.md`; they never directly mutate or delete an original committed event. The user-facing `Undo` action appends the typed reversal event.

Deleting a source file is a separate evidence action. It makes linked uncommitted records ineligible for future automatic commit and leaves them visibly tied to `Source file deleted`; it neither removes nor reverses committed ledger events.

Each accepted proposal version has a stable commit idempotency key. Repeating the same commit returns the existing result rather than creating another ledger event.

Financial mutations and their append-only audit entries are written in the same database transaction. Audit records identify actor, action, reason, source/proposal version, policy version, affected entity, and safe before/after references or hashes without copying unnecessary sensitive payloads.

## Ledger impact rules

Credit card purchase:

```text
spending = yes
liability increases
```

Credit card repayment:

```text
spending = no
cash decreases
liability decreases
link to card/account payment when possible
```

Balance/valuation snapshot:

```text
transaction = no
proof/validation = yes
may affect current displayed asset value
```

## Production review and ledger backend boundary

`review-ledger-backend` moves the synthetic review/ledger behavior behind the production SQLite/Tauri boundary. `packages/core` remains the single owner of pure deterministic validation, reconciliation, and event/leg/match/reversal/replacement construction. Tauri invokes that core through the protocol-separated deterministic mode of the bundled Node worker; the mode has no AI/model, filesystem, database, network, secret, or renderer capability. Rust owns privileged commands, durable-job orchestration, protocol validation, SQLCipher transactions, review projection changes, and audit persistence. The renderer receives presentation models and user actions only; it cannot submit ledger IDs, legs, event types, commit idempotency keys, or database-shaped payloads as authority.

### Uncommitted review state and concurrency

An edit creates the next external-record version and supersedes only the previous uncommitted current-state projection. It preserves the parse run, bounded raw source row, prior validation, review decisions, and audit. A committed record/event is never edited in place.

Every edit, remove, relationship, or Add request identifies the review item and expected current record version. The host compares both the version and current review status before writing. A stale request returns a safe conflict with no mutation so the renderer can reload current detail.

`commit_review_batch` is a coarse durable job under `0015-job-engine-error-model.md`. It preflights every selected current record, groups records that form one canonical event, and treats each group as one commit unit:

```text
valid current group   -> event/legs/matches + review resolution + audit in one transaction
stale or invalid group -> no financial writes; leave it in Review with a safe reason
other independent group -> may still commit
```

The result reports each selected group as committed, already committed, stale, or still needing review. Retrying the job reuses the accepted proposal-version idempotency keys and cannot duplicate ledger events, review decisions, or audit entries.

### Historical relationship discovery

Relationship discovery is an internal indexed SQLite query, not an external API or AI decision. Starting from one current record, search the provider/event-type-owned bounded date window across all imported historical periods in both directions. A statement-month boundary never limits the query.

A relationship recommendation requires compatible native unit, distinct resolved accounts, one unique qualified candidate, and an event-specific signed-effect predicate:

```text
same-currency transfer -> exact equal magnitude; outgoing account decreases and incoming account increases
credit-card repayment  -> exact equal magnitude; cash decreases and card liability decreases
```

These effects come from the canonical signed `accountBalanceDelta`, never directly from a source Debit/Credit label. For the first production rule set, same-currency transfers allow at most three calendar days and credit-card repayments allow at most seven calendar days. The canonical event date is the cash/outgoing account record's `postedOn`. These windows only produce Review recommendations; they never authorize auto-commit. Multiple candidates, partial allocations, unmatched remainders, or a missing qualified rule remain in Review.

This permits an outgoing HSBC payment to find the corresponding DBS credit-card side even when the two rows arrive in different statement months or import order. Two uncommitted sides can form one canonical event before commit. If a candidate is already represented by a committed financial event, the system never inserts or resizes an allocation on that event; an accepted correction appends the event-type-specific reversal and replacement under `0013-ledger-assets-valuation.md`.

### Presentation-safe host contract

Freeze the Rust/TypeScript request and response types before renderer integration. Exact source names follow repository conventions, but the command purposes are:

```text
read:
  list review items
  get review detail
  list recent activity
  get money overview
  list relationship candidates

mutate:
  edit one current review record
  remove one current review record
  accept one relationship with explicit allocations
  enqueue one selected review batch
  undo one committed event

job:
  get safe job status/result
```

Read models may return stable IDs, consumer labels, native-unit amounts, dates/periods, source/provider/account labels, attention summaries, bounded evidence snippets already allowed by `0017-evidence-documents-source-ux.md`, relationship explanations, and allowed actions. They must not return raw stored JSON, validation internals, audit payloads, paths, hashes, locators, secrets, unrestricted document text, or database authority.

### `review-ledger-backend` implementation checkpoints

1. Add only the migration/indexes exercised by production review, recent-activity, overview, and relationship-candidate repository queries. Freeze presentation-safe TypeScript/Tauri contracts and deterministic Kimi fixtures.
2. Add version-checked edit/remove/relationship actions plus the minimal persisted `jobs`/lease/recovery path for `commit_review_batch`. Reuse the canonical validator and host-derived identities.
3. Add historical candidate queries and event-type-specific reversal/replacement. Cover DBS-card/HSBC-bank repayment across statement months and import order, ambiguous/partial review, repayment excluded from spending, balanced legs, and immutable originals.

`review-ledger-backend` completion requires persisted reload/restart tests, locked-Vault behavior, migration-from-prior-schema coverage, and local native/desktop gates.

### `review-ledger-ui` integration checkpoint

Kimi integrates the renderer against the frozen host contracts. Completion requires deterministic host fixtures, browser behavior, accessibility, reduced motion, and final designer-level visual review.

## Acceptance criteria

- MVP exposes one default-on `Automatically add qualified records` toggle.
- With automatic addition on, qualified records commit without a pre-submit checkbox; with it off, Review starts with no staged records selected and supports subset/all/none selection through `Add selected`.
- Exact duplicate reuse is mandatory ingestion idempotency, not a user-selectable auto-commit level.
- Any financial event type may auto-commit only after every accepted record-level and reconciliation-window gate passes.
- Normal settings do not expose numeric confidence, parser controls, or a three-state policy selector.
- Provider/package calibration and deterministic checks, not raw LLM confidence or a global threshold, decide very-high confidence.
- Exact source-backed snapshot closure is required for every affected native-unit scope, without residuals or missing financial fields.
- A qualified non-posting observation may establish the first anchor, but cannot retroactively validate an incomplete earlier posting window.
- A mixed document may auto-commit individually eligible records while semantic-only ambiguities remain in Review; financial gaps block the window.
- Auto-committed records appear quietly in Recent Activity with one-click reversal-backed Undo.
- Exact duplicate handling is auditable.
- Partial and one-to-many matches remain simple in normal UI and block auto-commit.
- Two bank-side records can link to one canonical transfer event and be discovered from either side.
- Historical relationship search crosses statement-month boundaries through a bounded deterministic provider/event-type rule and never guesses between multiple candidates.
- A transaction email and posted statement row remain separate evidence but can link to one canonical event through deterministic provider ID or one unique qualified fallback match.
- Late email evidence attaches to a committed event only through an append-only audited corroboration edge; it never changes committed allocations or ledger legs.
- Transaction notifications never satisfy statement snapshot closure or create duplicate income, spending, or balance impact.
- Repayments do not double-count spending.
- Committed events are corrected through reversal/replacement, never mutation or deletion.
- Removing a staged record is an append-only decision over a mutable projection, while deleting source evidence never implicitly corrects the ledger.
- Financial mutations and review decisions create atomic append-only audit entries.
- A manual review batch commits each independent valid canonical event group atomically, leaves stale/invalid groups in Review, and reports every selected outcome.
- Repeated commit requests are idempotent.
- Every committed event traces to source evidence and parse run.
