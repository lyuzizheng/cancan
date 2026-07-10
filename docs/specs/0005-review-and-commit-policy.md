# 0005. Review and Commit Policy Spec

## Goal

Define when records can auto-commit, when they require review, and how reconciliation avoids double-counting.

## Implementation blocker

Ledger/reconciliation semantics, undo/reversal behavior, and the AI ledger-authority ADR status remain unresolved in the [active alignment register](../alignment-temp/alignment-progress.md). Do not implement commit, destructive, or AI-authority paths beyond accepted invariants.

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
Auto-dedupe exact duplicates only; review everything else.
```

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
- Exact duplicate handling is auditable.
- Repayments do not double-count spending.
- Review decisions create audit log entries.
- Every committed event traces to source evidence and parse run.
