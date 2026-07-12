# ADR 0002: AI agent is advisor/parser, not ledger owner

## Status

Accepted

## Context

The app will parse highly sensitive financial data and generate reconciliation suggestions. LLMs are useful for messy PDFs, OCR output, merchant normalization, duplicate detection, and ambiguous matches, but financial ledger mutation must be deterministic and auditable.

## Decision

The AI/agent layer may propose structured records and match candidates, but it cannot directly commit, delete, or permanently ignore ledger records.

The commit path is:

```text
AI/rule proposal
-> schema validation
-> deterministic financial validation
-> review or auto-policy
-> user confirmation when required
-> ledger commit
```

Low-risk standalone purchases may auto-commit only after schema validation, deterministic validation, confidence threshold checks, account mapping, and duplicate/link checks.

An enabled deterministic auto-policy may commit an eligible proposal. AI confidence is only one input to that policy; it never grants direct ledger-write authority by itself. `0005-review-and-commit-policy.md` owns which policy is enabled by default.

Transfers, credit card repayments, top-ups, FX conversions, broker deposits, crypto movements, trades, and ambiguous duplicates default to review unless a future explicit auto-policy is approved.

## Allowed AI capabilities

```text
document classification
structured extraction
record normalization
merchant/counterparty normalization
account mapping suggestions
match explanation
candidate reranking
parse repair suggestions
parser template generation
```

## Disallowed AI capabilities

```text
read secrets
write committed ledger directly
delete records
send emails
make payments
place trades
withdraw crypto
bypass authentication
```

## Consequences

Positive:

```text
ledger remains auditable
AI mistakes are contained
review UI becomes central
safer handling of finance data
repeatable deterministic validation
```

Negative:

```text
slower than fully autonomous workflow
more engineering around review and validation
deterministic eligibility rules and conservative thresholds require fixture-backed calibration and maintenance
```
