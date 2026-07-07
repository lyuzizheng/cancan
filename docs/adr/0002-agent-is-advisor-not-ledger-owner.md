# ADR 0002: AI agent is advisor/parser, not ledger owner

## Status

Proposed

## Context

The app will parse highly sensitive financial data and generate reconciliation suggestions. LLMs are useful for messy PDFs, OCR, merchant normalization, and ambiguous matches, but financial ledger mutation must be deterministic and auditable.

## Decision

The AI/agent layer may propose structured records and match candidates, but it cannot directly commit, delete, or permanently ignore ledger records.

The commit path is:

```text
AI/rule proposal
-> schema validation
-> deterministic financial validation
-> review policy
-> user confirmation or safe auto-policy
-> ledger commit
```

## Allowed AI capabilities

```text
document classification
structured extraction
merchant/counterparty normalization
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
some low-risk actions may need explicit policy design before auto-accept
```
