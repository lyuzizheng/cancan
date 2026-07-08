# 0013. Ledger, Assets, Valuation, and Multi-Currency Spec

## Goal

Define how CanCan represents assets, liabilities, positions, snapshots, valuations, trades, and multi-currency views without overcomplicating MVP or inventing unsupported financial facts.

## Core principle: fact-based finance

CanCan is a record, reconciliation, and evidence dashboard. It should preserve and organize source evidence as user-facing financial memory.

Implementation rules:

```text
Do not fetch market prices in MVP.
Do not fetch external FX rates in MVP.
Do not calculate tax or tax-grade realized gains.
Do not create analytics that imply unsupported truth.
Do not derive values without source evidence or explicit user-approved settings.
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
manual import evidence
```

If no source evidence provides current value, CanCan should not fetch market price or estimate live value.

## No base currency in MVP

CanCan should not ask the user to choose a base currency during MVP onboarding or vault settings.

CanCan is a multi-currency factual overview. User assets may naturally exist across SGD, USD, CNY, HKD, securities, policies, and other native units. A base currency setting can incorrectly imply that CanCan will convert everything into one total.

MVP should show native values and source-provided valuations without forcing a single synthetic total.

## Multi-currency model

CanCan should be multi-currency first.

Store and show native values:

```text
SGD cash
USD cash
AAPL shares
policy value in stated currency
credit card liability in stated currency
```

If a statement/source provides a valuation in another currency, store that valuation and source it.

If no source provides conversion, show separate currency buckets rather than inventing conversion.

## Money Overview instead of default Net Worth

The Command Center should not default to a single `Net Worth` number unless the source evidence already supports compatible aggregation.

Preferred MVP language:

```text
Money Overview
Source Overview
Your Money Sources
```

Single-currency subtotal cards are allowed when the underlying values are compatible.

## Source-provided equivalent values

If a statement provides an equivalent value, CanCan may display it.

Rules:

```text
Store the provider/source valuation amount.
Store the valuation currency.
Store the source document / external record reference.
Store valuation_at or statement date when available.
Do not relabel source-provided values as app-calculated values.
```

## Optional network activity for future estimated totals

A future user setting may enable network activity for external FX rates, market prices, or estimated total views.

MVP default:

```text
network activity for valuation = off
market price fetch = off
external FX fetch = off
estimated total = unavailable unless source-backed
```

If added later, network activity must be explicitly enabled by the user, visible in Settings, reversible, versioned, and separate from source-provided facts.

## FX events

FX conversion is still a real event type when source evidence contains both sides.

Example:

```text
Wise statement:
- SGD -1000
- USD +740
- fee if present
```

CanCan records this as a source-backed FX conversion. It does not need to invent an external FX rate. If the implied rate is useful, it can be calculated as an explanation tied to the source event, not as a market price.

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

If the source statement provides cost basis, P/L, or unrealized gain, CanCan may display it as source-provided. If not provided, do not invent it.

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
single-currency totals only aggregate compatible source-backed values
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
- brokerage
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

## Product copy guardrail

User-facing copy should emphasize what CanCan helps users see and organize. Avoid defensive wording that lists product limitations.

Docs may state technical constraints clearly for implementation safety, but product UI should stay simple, confident, and user-centered.

## Acceptance criteria

- MVP does not ask for base currency.
- MVP does not fetch market prices or external FX rates.
- Native values are preserved and displayed.
- Statement/source snapshots can drive current position/value display.
- Source-provided equivalent values can be shown with evidence.
- Single totals are only shown when source evidence supports compatible aggregation.
- Future estimated totals require explicit network activity settings.
- Trades can be represented without a specialized table in MVP.
- Future specialized trade table remains possible without data loss.
- Insurance policy values are snapshots.
- Liabilities are separate and repayments do not double-count spending.
- UI uses timestamps and simple state labels rather than overcomplicated freshness scores.
