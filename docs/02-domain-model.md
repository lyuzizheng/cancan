# 02. Domain Model

## Core concepts

CanCan models financial data across these dimensions:

```text
Money Source:   DBS, UOB, Wise, Moomoo, Bitget, Manulife
Sub-account:    bank account, credit card, wallet balance, broker account, policy
Money Type:     cash, liability, stock, ETF, crypto, insurance, fund
Instrument:     SGD, USD, AAPL, BTC, ETH, policy identifier
Evidence:       PDF, CSV, Gmail email, API snapshot, OCR/text output
Ledger Event:   canonical financial event or snapshot
Ledger Leg:     account/instrument-level effect of an event
Match Edge:     duplicate/link/transfer relationship between records/events
```

## Product data philosophy

CanCan is fact-based.

It should preserve and organize source evidence rather than invent financial truth. It should not fetch market prices or external FX rates in MVP. It should not do tax-grade realized gain calculations. It should show native values, source-provided values, timestamps, and evidence links.

## Money sources and sub-accounts

The user manually creates Money Sources and sub-accounts. Parsers may propose mappings, but should not silently create official accounts.

Example:

```text
DBS
- Multiplier Account / deposit / SGD
- DBS Visa Card / credit_card / SGD

UOB
- One Account / deposit / SGD
- UOB Credit Card / credit_card / SGD

Wise
- Wise SGD Balance / wallet / SGD
- Wise USD Balance / wallet / USD

Moomoo
- Moomoo Brokerage / broker / USD
```

When a statement is parsed, CanCan should identify the likely source, sub-account, account type, and account hint. If ambiguous, create a review item.

## Base currency and native values

Default base currency is `SGD`, configurable in vault settings.

Base currency is a UI preference, not permission to invent FX conversions.

Always preserve native values:

- native amount/currency for cash and liabilities;
- native quantity/instrument for stocks, ETFs, funds, and crypto;
- source-provided valuation amount/currency when available;
- valuation timestamp when known.

If no source provides conversion, show separate currency buckets rather than inventing conversion.

## Entities

### money_sources

```text
money_sources
- id
- name
- source_type             -- bank, card_provider, wallet, broker, crypto, insurance, email, manual
- status                  -- active, disabled, archived
- created_at
- updated_at
```

### accounts

```text
accounts
- id
- money_source_id
- name
- account_type            -- deposit, credit_card, wallet, broker, crypto_wallet, insurance_policy
- base_currency
- external_hint
- status
- created_at
- updated_at
```

### instruments

```text
instruments
- id
- symbol
- name
- instrument_type         -- fiat, stock, etf, crypto, fund, insurance_policy
- currency                -- native/valuation currency if applicable
- metadata_json
```

### source_documents

```text
source_documents
- id
- money_source_id
- source_type             -- gmail, api, pdf, csv, screenshot, manual_upload, watched_folder
- external_id
- file_hash
- file_path
- mime_type
- received_at
- statement_period_start
- statement_period_end
- document_status         -- raw, extracted, parsed, staged, committed, failed
- created_at
- updated_at
```

### parse_runs

```text
parse_runs
- id
- source_document_id
- parser_name
- parser_version
- ai_provider
- model
- prompt_hash
- extraction_bundle_hash
- status
- input_hash
- output_hash
- validation_json
- error_json
- created_at
```

### external_records

Extracted rows or structured records before ledger commit.

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

A `ledger_event` is CanCan's canonical representation of a financial event or proof point.

```text
ledger_events
- id
- event_type              -- purchase, income, transfer, card_payment, topup, fx_conversion, trade_execution, balance_snapshot, valuation_snapshot, premium, fee, interest
- event_date
- source_document_id
- external_record_id
- description
- status                  -- staged, committed, reversed
- confidence
- affects_spending        -- true/false
- affects_income          -- true/false
- affects_net_worth       -- true/false/derived
- created_at
- updated_at
```

A snapshot can be a ledger event, but it is not a transaction. It proves a balance or valuation and is used for display, timestamps, and validation.

### ledger_legs

A `ledger_leg` represents the effect of an event on one account and one instrument.

```text
ledger_legs
- id
- ledger_event_id
- account_id
- instrument_id
- amount                  -- fiat/cash movement
- quantity                -- stock/crypto/fund units
- currency
- direction               -- debit, credit, in, out
- balance_after
- valuation_amount        -- source-provided valuation only
- valuation_currency
- valuation_at
- metadata_json
```

## Trade table stance

MVP does not require a specialized `trades` table immediately.

Use:

```text
external_records.raw_json for provider-specific trade details
ledger_events.event_type = trade_execution
ledger_legs for cash/instrument impacts when confidently parsed
position/valuation snapshots for current source-provided facts
```

A specialized `trades` table can be added later referencing `ledger_event_id` and `external_record_id` if brokerage reporting, lot accounting, or richer trade analytics require it.

## match_edges

Links records or events together.

```text
match_edges
- id
- left_record_id
- right_record_id
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

```text
review_items
- id
- review_type             -- parse_warning, possible_duplicate, possible_transfer, unmatched_record, account_mapping, valuation_conflict
- related_ids_json
- priority
- status                  -- open, accepted, rejected, snoozed
- title
- explanation
- created_at
- resolved_at
```

## Key distinction

Duplicate is not the same as transfer.

```text
Duplicate:
Same real-world event imported twice.
Keep one canonical event and retain the duplicate as evidence.

Transfer/link:
Two records are both true. They represent different sides of the same internal movement.
Keep both and link them so spending/income is not double-counted.
```
