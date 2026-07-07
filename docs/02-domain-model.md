# 02. Domain Model

## Core concepts

CanCan should model financial data across four primary dimensions:

```text
Finance Source: DBS, UOB, Wise, Moomoo, Bitget, Manulife
Account:        specific bank account, card, wallet, broker account, policy
Money Type:     cash, liability, stock, ETF, crypto, insurance, fund
Instrument:     SGD, USD, AAPL, BTC, ETH, policy identifier
```

## Why this matters

A single source can contain multiple money types.

Examples:

```text
DBS
- SGD cash account
- SGD credit card liability

Wise
- SGD cash balance
- USD cash balance
- FX conversion events

Moomoo
- USD cash
- stock positions
- trades

Bitget
- USDT balance
- BTC/ETH crypto positions
- deposits and withdrawals

Manulife
- insurance policy value
- premium payments
```

The UI should allow pivoting by source, account, money type, currency, and instrument.

## Entities

### finance_sources

Represents a provider or source family.

```text
finance_sources
- id
- name                    -- DBS, UOB, Wise, Moomoo, Bitget
- source_type             -- bank, card, payment_app, broker, crypto, insurance, email, manual
- status                  -- active, disabled, archived
- created_at
- updated_at
```

### accounts

Represents a container at a source.

```text
accounts
- id
- finance_source_id
- name
- account_type            -- deposit, credit_card, wallet, broker, crypto_wallet, insurance_policy
- base_currency
- external_hint           -- last4, masked account number, provider account id
- status
- created_at
- updated_at
```

### instruments

Represents a money/capital object.

```text
instruments
- id
- symbol                  -- SGD, USD, AAPL, BTC, MANULIFE_POLICY_123
- name
- instrument_type         -- fiat, stock, etf, crypto, fund, insurance_policy
- currency                -- valuation currency if applicable
- metadata_json
```

### source_documents

Represents raw evidence.

```text
source_documents
- id
- finance_source_id
- source_type             -- gmail, api, pdf, csv, screenshot, manual_upload, watched_folder
- external_id             -- Gmail message id, API response id, file import id
- file_hash
- file_path
- mime_type
- received_at
- statement_period_start
- statement_period_end
- document_status         -- raw, classified, extracted, parsed, staged, committed, failed
- created_at
- updated_at
```

### parse_runs

Every parse attempt is recorded.

```text
parse_runs
- id
- source_document_id
- parser_name
- parser_version
- ai_provider
- model
- prompt_hash
- status
- input_hash
- output_hash
- validation_json
- error_json
- created_at
```

### external_records

Represents extracted rows or structured records before ledger commit.

```text
external_records
- id
- source_document_id
- parse_run_id
- record_type             -- transaction, balance, position, trade, valuation, fee, interest
- row_index
- row_hash
- raw_text
- raw_json
- normalized_json
- confidence
- status                  -- staged, ignored, committed, failed
- created_at
```

### ledger_events

Represents a canonical financial event.

```text
ledger_events
- id
- event_type              -- purchase, transfer, topup, fx_conversion, trade, balance_snapshot, premium
- event_date
- source_document_id
- external_record_id
- description
- status                  -- staged, committed, reversed
- confidence
- created_at
- updated_at
```

### ledger_legs

Represents the impact of an event on an account and instrument.

```text
ledger_legs
- id
- ledger_event_id
- account_id
- instrument_id
- amount                  -- for fiat/cash value movement
- quantity                -- for stock/crypto/fund units
- currency
- direction               -- debit, credit, in, out
- balance_after
- valuation_amount
- valuation_currency
- metadata_json
```

## Why ledger_legs are needed

A normal single-bank purchase has one leg:

```text
DBS Visa purchase
- SGD -25.60
```

A Wise FX conversion has two legs:

```text
Wise conversion
- SGD -1000
- USD +740
```

A Moomoo trade has at least two legs:

```text
Buy AAPL
- USD cash -740
- AAPL quantity +1
```

A balance snapshot may have one valuation leg:

```text
Manulife policy value snapshot
- policy value SGD 20,000
```

## match_edges

Links events or records together.

```text
match_edges
- id
- left_event_id
- right_event_id
- match_type              -- duplicate, transfer, topup, cc_payment, fx_conversion, broker_deposit
- confidence
- evidence_json
- review_status           -- auto_accepted, needs_review, confirmed, rejected
- created_by              -- rule, ai, user
- created_at
- confirmed_at
```

## review_items

Represents human review tasks.

```text
review_items
- id
- review_type             -- parse_warning, possible_duplicate, possible_transfer, unmatched_record
- related_ids_json
- priority
- status                  -- open, accepted, rejected, snoozed
- title
- explanation
- created_at
- resolved_at
```

## Important distinction

Duplicate is not the same as transfer.

```text
Duplicate:
Same real-world event imported twice.
One record should be ignored or merged.

Transfer/link:
Two real-world records are both true.
They represent two sides of the same internal movement.
Both should remain in the ledger, but be linked.
```
