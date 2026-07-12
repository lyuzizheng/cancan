# 0005. Review and Commit Policy Spec

## Goal

Define when records can auto-commit, when they require review, and how reconciliation avoids double-counting.

## Implementation blocker

The desired product default is accepted, but exact record/account confidence thresholds, trusted-source eligibility, and their fixture-backed calibration remain unresolved in the [active alignment register](../alignment-temp/alignment-progress.md). Do not enable the safe standalone-purchase auto-commit execution path until those criteria are accepted and tested.

## Policy levels

User-configurable settings:

```text
Never auto-commit
Auto-dedupe exact duplicates only
Auto-commit safe standalone purchases
Future: custom advanced policies
```

Recommended default:

```text
Auto-commit safe standalone purchases; review all excluded or ambiguous cases.
```

The default can be disabled. AI confidence is an input to the policy, not permission to write the ledger. [ADR 0002](../adr/0002-agent-is-advisor-not-ledger-owner.md) owns the accepted AI authority boundary.

## Auto-commit standalone purchase criteria

A record may auto-commit as a standalone purchase only if all are true:

```text
schema_valid = true
deterministic_validation_passed = true
record_confidence >= configured_threshold
account_mapping_confidence >= configured_threshold
not a transfer/top-up/repayment/FX/trade/broker/crypto candidate
not a duplicate candidate above threshold
source document is trusted enough for policy
```

## Always review by default

```text
credit card repayments
bank transfers
Wise/Revolut/payment app top-ups
FX conversions
broker deposits
crypto deposits/withdrawals
stock/ETF/fund trades
insurance premiums if policy classification is ambiguous
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
- Safe standalone-purchase auto-commit is enabled by default but never relies on AI confidence alone.
- Exact duplicate handling is auditable.
- Partial and one-to-many matches remain simple in normal UI and block auto-commit.
- Repayments do not double-count spending.
- Committed events are corrected through reversal/replacement, never mutation or deletion.
- Financial mutations and review decisions create atomic append-only audit entries.
- Repeated commit requests are idempotent.
- Every committed event traces to source evidence and parse run.
