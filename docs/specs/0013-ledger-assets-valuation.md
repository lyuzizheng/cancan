# 0013. Ledger, Assets, Valuation, and Multi-Currency Spec

## Goal

Define how CanCan represents assets, liabilities, positions, snapshots, valuations, trades, and multi-currency views without overcomplicating MVP or inventing unsupported financial facts.

## Core principle: fact-based finance

CanCan is a record, reconciliation, and evidence dashboard. It should preserve and organize source evidence as user-facing financial memory.

Implementation rules:

```text
Do not fetch market prices in MVP.
Do not fetch external FX rates unless the user enables the one fixed read-only FX-rate source.
Do not calculate tax or tax-grade realized gains.
Do not create analytics that imply unsupported truth.
Do not derive values without source evidence or explicit user-approved settings.
Prefer statement/export/API facts over inferred calculations.
Show source, timestamp, and evidence for displayed financial facts.
```

LLM may normalize and explain source evidence. It should not invent valuation, returns, gains, or tax logic.

## Event classes and lifecycle

Ledger events have two explicit classes:

```text
posting      changes an account, liability, cash, or instrument state
observation  records a source-backed balance, position, or valuation at a point in time
```

Only posting events change ledger-derived balances. Observations validate or anchor displayed state and can reveal discrepancies without inventing a transaction.

Draft proposals may change. Once committed, an event and its legs are immutable. Corrections use a typed reversal event and, when needed, a replacement event. Reparse, staged-record removal, source-file deletion, and retry flows must not mutate or delete committed events.

## Initial balance anchor

The first import does not imply that an account started at zero.

When the earliest accepted evidence contains a balance or valuation, store it as an observation and use it as a source-backed balance anchor at that timestamp. Do not synthesize historical income, spending, transfer, or adjustment postings to explain the unknown earlier history.

If the earliest evidence contains activity but no balance, show the known activity and mark the balance unavailable. Never derive a current balance by assuming the account began at zero.

User-facing behavior should be simple:

```text
show "Balance as of <date>" from the source-backed anchor
show later known activity relative to that anchor
describe earlier history as unavailable rather than zero
if a later snapshot disagrees with known postings, create a review discrepancy
never auto-create an adjustment event to hide the difference
```

## Posting invariants

Use small event-type invariants rather than a universal cross-currency balancing engine:

```text
purchase              has an amount/currency and an affected cash or liability side
same-currency transfer has linked outgoing and incoming sides; explicit fees remain separate
credit-card repayment decreases cash and liability and is not spending
FX conversion         has both source-backed currency sides and any evidenced fee
trade execution       has the parsed cash/instrument sides and any evidenced fee
```

Do not numerically add different currencies or instruments to claim that an event balances. Do not generate artificial clearing legs merely to satisfy a generic zero-sum rule. Incomplete required sides stay in review rather than becoming a committed posting.

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

## Money Overview product policy

`0014-money-overview-source-taxonomy.md` owns the no-base-currency, Money Overview, and user-facing aggregation policy. This spec owns the underlying ledger, valuation, position, and multi-currency behavior.

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

## Optional fixed FX-rate source for estimated totals

A user setting may enable one product-defined read-only FX-rate source for estimated total views. This is the only MVP network valuation capability.

MVP default:

```text
network activity for valuation = off
market price fetch = off
external FX fetch = off
estimated total = unavailable unless source-backed
```

When enabled, the fixed FX-rate source must be visible in Settings, reversible, versioned, cached with its timestamp/source, and clearly separated from source-provided facts. It may produce labeled estimates only and must never create or mutate ledger events.

MVP does not fetch market prices or allow arbitrary valuation providers.

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
ledger_events represent canonical posting trade executions or observation snapshots
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
ledger_event.event_class = observation
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

- MVP does not fetch market prices; its only optional network valuation is one user-enabled, product-defined read-only FX-rate source.
- Native values are preserved and displayed.
- Posting and observation events are explicit; only postings change ledger-derived balances.
- The first source-backed balance is an observation anchor, not invented historical income or an adjustment transaction.
- Posting validity uses simple event-type invariants without adding different currencies or instruments together.
- Committed events are immutable and corrections use reversal/replacement events.
- Statement/source snapshots can drive current position/value display.
- Source-provided equivalent values can be shown with evidence.
- FX-based estimated totals require the explicit optional FX-rate setting and remain labeled estimates.
- Trades can be represented without a specialized table in MVP.
- Future specialized trade table remains possible without data loss.
- Insurance policy values are snapshots.
- Liabilities are separate and repayments do not double-count spending.
- UI uses timestamps and simple state labels rather than overcomplicated freshness scores.
