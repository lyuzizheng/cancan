# 0005. Review and Commit Policy Spec

## Goal

Define when records can auto-commit, when they require review, and how reconciliation avoids double-counting.

## Implementation blocker

The desired default-enabled auto-commit direction and provider-package qualification flow are accepted. The exact meaning of a sufficiently clear record and which financial event types may auto-commit remain unresolved in the [active alignment register](../alignment-temp/alignment-progress.md). Do not enable live auto-commit until that boundary and first-seen account commit eligibility are accepted.

## Policy levels

User-configurable settings:

```text
Never auto-commit
Auto-dedupe exact duplicates only
Auto-commit qualified clear records
Future: custom advanced policies
```

Recommended default:

```text
Auto-commit qualified clear records; review all excluded or ambiguous cases.
```

Expose one simple user toggle for this policy; do not expose confidence numbers, provider allowlists, or parser qualification internals in normal settings. The default can be disabled. [ADR 0002](../adr/0002-agent-is-advisor-not-ledger-owner.md) owns the accepted AI authority boundary.

## Qualified document baseline

A record can be considered for auto-commit only if all are true:

```text
user auto-commit toggle is enabled
document arrived through a configured Money Source channel or explicit import into that source
AI classifier selected the configured supported provider and document type
provider package deterministic fingerprints and schema checks passed
provider/document/parser package version is currently qualified
schema_valid = true
deterministic_validation_passed = true
account/container identity is resolved for commit
not an exact/probable duplicate, partial allocation, or warning case
record/event type is inside the accepted auto-commit boundary
```

Do not use one global numeric AI confidence threshold as the trust boundary. Confidence may help rank review items, but provider-package qualification plus deterministic checks decide eligibility.

Classification and parsing continue when a configured source reveals new account candidates. Those records remain staged until the Money Source identity contract permits commit to that account.

## Event-type boundary still requiring design

```text
credit card repayments
bank transfers
Wise/Revolut/payment app top-ups
FX conversions
broker deposits
crypto deposits/withdrawals
stock/ETF/fund trades
insurance premiums if policy classification is ambiguous
refunds
cash withdrawals
interest and standalone fees
```

Clear provider documents may eventually allow some of these types to auto-commit, but document readability alone is not yet an executable financial rule. Until the next decision is accepted, these types remain review-only.

The following always require review under the accepted matching rules:

```text
partial matches
one-to-many matches
ambiguous account mappings
validation warnings
```

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

- User can disable auto-commit.
- Qualified clear-record auto-commit is the desired default, but live execution stays blocked until its event-type and first-seen-account boundaries are accepted.
- Normal settings expose one simple toggle rather than numeric confidence or parser controls.
- Provider/package qualification and deterministic checks, not a global AI confidence threshold, decide eligibility.
- Auto-committed records appear quietly in Recent Activity with one-click reversal-backed Undo.
- Exact duplicate handling is auditable.
- Partial and one-to-many matches remain simple in normal UI and block auto-commit.
- Repayments do not double-count spending.
- Committed events are corrected through reversal/replacement, never mutation or deletion.
- Financial mutations and review decisions create atomic append-only audit entries.
- Repeated commit requests are idempotent.
- Every committed event traces to source evidence and parse run.
