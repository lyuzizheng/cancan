# 0005. Review and Commit Policy Spec

## Goal

Define when records can auto-commit, when they require review, and how reconciliation avoids double-counting.

## Implementation blocker

The auto-commit rule is accepted. The synthetic core flow may exercise it with deterministic mocked proposals; live normalization and qualification still wait for the document-normalizer runtime evidence plus their upstream vault/Gmail slices in the [active alignment register](../alignment-temp/alignment-progress.md). Do not weaken the accepted gates to work around those boundaries.

## MVP user setting

Expose one setting:

```text
Automatically add qualified records = On | Off
```

Recommended default is `On`. Turning it off sends otherwise eligible records to Review.

Do not expose confidence numbers, provider allowlists, parser qualification internals, or a three-state policy selector in normal settings. [ADR 0002](../adr/0002-agent-is-advisor-not-ledger-owner.md) owns the accepted AI authority boundary.

Exact SHA-256 duplicate reuse always applies as import idempotency under `0004-parser-contract.md`; it is not an auto-commit setting and cannot be disabled through this toggle.

## Qualified document baseline

A record can be considered for auto-commit only if all are true:

```text
user auto-commit toggle is enabled
document arrived through a configured Money Source channel or explicit import into that source
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

## Commit, reversal, and audit policy

Uncommitted proposals may be edited or removed. Committed correction requests follow the immutable reversal/replacement lifecycle owned by `0013-ledger-assets-valuation.md`; they never directly mutate or delete an original committed event.

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

## Acceptance criteria

- MVP exposes one default-on `Automatically add qualified records` toggle.
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
- Repayments do not double-count spending.
- Committed events are corrected through reversal/replacement, never mutation or deletion.
- Financial mutations and review decisions create atomic append-only audit entries.
- Repeated commit requests are idempotent.
- Every committed event traces to source evidence and parse run.
