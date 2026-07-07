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

Always preserve native values:

- native amount/currency for cash and liabilities;
- native quantity/instrument for stocks, ETFs, funds, and crypto;
- valuation amount and valuation currency for position values;
- exchange rate and valuation timestamp when known.

Base-currency values are derived views, not replacements for native records.

## Entities

### money_sources

```text
money_sources
- id
- name                    -- DBS, UOB, Wise, Moomoo, Bitget
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
- external_hint           -- last4, masked account number, provider account id
- status
- created_at
- updated_at
```

### instruments

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

```text
source_documents
- id
- money_source_id
- source_type             -- gmail, api, pdf, csv, screenshot, manual_upload, watched_folder
- external_id             -- Gmail message id, attachment id, API response id, file import id
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

It is used because raw financial sources are inconsistent. One provider may call something a transaction, another calls it a movement, another gives only statement rows, and another gives a valuation snapshot. CanCan needs a common model to power assets, transactions, review, reconciliation, and audit.

```text
ledger_events
- id
- event_type              -- purchase, income, transfer, card_payment, topup, fx_conversion, trade, balance_snapshot, valuation_snapshot, premium, fee, interest
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

A snapshot can be a ledger event, but it is not a transaction. It proves a balance or valuation and is used for freshness and validation.

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
- valuation_amount
- valuation_currency
- valuation_at
- metadata_json
```

Examples:

```text
Credit card purchase
- DBS Visa SGD liability +25.60

UOB pays DBS credit card
- UOB One Account SGD cash -1000
- DBS Visa SGD liability -1000

Wise FX conversion
- Wise SGD balance -1000 SGD
- Wise USD balance +740 USD

Buy AAPL in Moomoo
- Moomoo USD cash -740 USD
- Moomoo AAPL position +1 share

Moomoo daily position value
- Moomoo AAPL position valuation 1 share at 760 USD

Manulife policy value snapshot
- Manulife policy valuation 20000 SGD
```

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

`match_edges` may link external records before commit and ledger events after commit. This supports review before mutation and audit after mutation.

## review_items

```text
review_items
- id
- review_type             -- parse_warning, possible_duplicate, possible_transfer, unmatched_record, account_mapping
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
