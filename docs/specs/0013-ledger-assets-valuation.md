# 0013. Ledger, Assets, Valuation, and Multi-Currency Spec

## Goal

Define how CanCan represents assets, liabilities, positions, snapshots, valuations, trades, and multi-currency views without overcomplicating MVP or inventing unsupported financial facts.

## Core principle: fact-based finance

CanCan is a record, reconciliation, and evidence dashboard. It should not invent financial data.

Rules:

```text
Do not fetch market prices in MVP.
Do not calculate tax or tax-grade realized gains.
Do not create analytics that imply unsupported truth.
Do not derive values without source evidence or explicit user-approved rules.
Prefer statement/export/API facts over inferred calculations.
Show source, timestamp, and evidence for displayed financial facts.
```

LLM may normalize and explain source evidence. It should not invent valuation, returns, gains, or tax logic.

## Current valuation

MVP valuation is based on source evidence only:

```text
bank/card statements
Wise export/statement
broker statement/API/export snapshot
insurance policy statement
crypto exchange export/API snapshot
manual import evidence
```

If no source evidence provides current value, CanCan should not fetch market price or estimate live value.

## Multi-currency model

CanCan should be multi-currency first.

Do not force all values into SGD using app-generated FX rates.

Store and show native values:

```text
SGD cash
USD cash
AAPL shares
BTC quantity
policy value in stated currency
credit card liability in stated currency
```

If a statement/source provides a valuation in another currency, store that valuation and source it.

If no source provides conversion, show separate currency buckets rather than inventing conversion.

## Base currency meaning

Base currency is a UI preference, not permission to invent FX conversions.

Use base currency when:

- the source evidence already provides base-currency valuation;
- the user explicitly configures an approved valuation/conversion source later;
- a future feature intentionally adds market/FX data with clear consent.

Until then, Command Center should be comfortable showing multi-currency totals and source-native values.

## FX events

FX conversion is still a real event type when source evidence contains both sides.

Example:

```text
Wise statement:
- SGD -1000
- USD +740
- fee if present
```

CanCan records this as a source-backed FX conversion. It does not need to invent an external FX rate. If the implied rate is useful, it can be calculated as an explanatory derived field tied to the source event, not as a market price.

## Trades table decision

MVP should not start with a specialized `trades` table unless the implementation clearly needs it.

Recommended MVP model:

```text
external_records preserve provider-specific trade rows
ledger_events represent canonical trade_execution or position/valuation snapshot events
ledger_legs represent cash and instrument impacts when confidently parsed
source_documents and parse_runs preserve evidence/audit
```

Why defer specialized trades table:

- it adds schema and migration complexity early;
- broker formats differ;
- CanCan is not doing tax/lot accounting in MVP;
- provider snapshots can carry current position facts without full trade accounting.

Why it should remain easy to add later:

- `external_records.raw_json` preserves provider-specific fields;
- `ledger_events.event_type` can distinguish `trade_execution`;
- `ledger_legs` can represent quantity/cash impacts;
- a future `trades` table can reference `ledger_event_id` and `external_record_id`.

## Positions

Use both source snapshots and parsed events where available.

MVP display priority:

```text
1. Source/provider position snapshot as current fact.
2. Parsed trades/events as explanation of changes.
3. If snapshot and event-derived position conflict, show review/conflict rather than silently choosing.
```

## Unrealized gains

Do not calculate complex unrealized gains in MVP.

If the source statement provides cost basis, P/L, or unrealized gain, CanCan may display it as source-provided.

If not provided, do not invent it.

## Realized gains and tax

Do not implement realized gain, lot accounting, or tax reporting in MVP.

Sell trades can be recorded as events, but tax-grade realized gain is a later feature.

## Insurance policy value

Insurance policy value is a valuation snapshot.

Recommended representation:

```text
instrument_type = insurance_policy
ledger_event.event_type = valuation_snapshot
ledger_leg.valuation_amount = source-provided policy value
source_document proves value
```

Premium payment classification remains configurable later, but policy value itself is snapshot evidence.

## Liabilities

Liabilities should be displayed separately from assets.

Rules:

```text
credit card purchases increase liability
credit card repayment reduces liability and cash
repayment is not spending
liabilities are shown in a Liabilities section
net worth only aggregates values that have compatible source-backed valuation
```

## Updated-at and freshness

Prefer clear timestamps over vague scores.

Show:

```text
last statement date
last imported at
last parsed at
last source update at
last valuation at
```

Use simple labels only when helpful:

```text
updated recently
stale
missing statement
conflict
unverified
```

Avoid confusing dashboard metrics like `92% fresh` unless later backed by a clear user-understandable definition.

## Source type stats registry

Implement source-specific stats through a registry.

```text
sourceTypeStatsRegistry
- bank_account
- credit_card
- wallet
- broker
- crypto
- insurance
```

Each source type defines:

```text
Command Center module stats
Source Detail stats
primary native values
secondary timestamps
review/freshness indicators
supported event types
```

This keeps frontend/backend behavior consistent and avoids scattered if/else logic.

## Acceptance criteria

- MVP does not fetch market prices or external FX rates.
- Native values are preserved and displayed.
- Base currency does not imply automatic conversion.
- Statement/source snapshots can drive current position/value display.
- Trades can be represented without a specialized table in MVP.
- Future specialized trade table remains possible without data loss.
- Insurance policy values are snapshots.
- Liabilities are separate and repayments do not double-count spending.
- UI uses timestamps and simple state labels rather than overcomplicated freshness scores.
