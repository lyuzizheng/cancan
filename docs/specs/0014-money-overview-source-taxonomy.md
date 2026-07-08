# 0014. Money Overview and Source Taxonomy Spec

## Goal

This spec aligns CanCan's top-level overview and source taxonomy.

It supersedes older MVP wording that asks the user to choose a base currency during onboarding or vault setup.

## No base currency in MVP

CanCan should not ask the user to configure a base currency in MVP.

The app organizes the assets, liabilities, currencies, instruments, and platform records that already exist in the user's evidence.

Implementation implication:

```text
Remove base currency from first-run onboarding.
Do not add a required base_currency vault setting for MVP.
Do not depend on base currency for Command Center rendering.
Do not auto-convert native values into one total.
```

## Money Overview

The Command Center should use Money Overview, Source Overview, or Your Money Sources instead of defaulting to one large Net Worth number.

The overview should display source-native facts such as SGD cash, USD cash, card liabilities, Wise balances, brokerage cash, positions from statement, insurance policy value, and other source-backed balances or valuations.

Single-currency subtotal cards are allowed when the underlying values are compatible.

## Source-provided equivalent values

If a statement provides an equivalent value in another currency, CanCan may show it as part of source evidence.

Store valuation amount, valuation currency, valuation date, and the source document or external record reference.

## Optional network activity for future estimated totals

A future setting may enable network activity for estimated totals, market prices, or external FX.

MVP default:

```text
network activity for valuation = off
market price fetch = off
external FX fetch = off
estimated total = not shown unless source-backed
```

If added later, it must be explicitly enabled by the user, visible in Settings, reversible, versioned, and separate from source-provided facts.

## Product copy guardrail

User-facing copy should not over-explain limitations or use defensive wording.

Preferred copy style:

```text
Your money sources, organized.
A private vault for statements, balances, movements, and evidence.
See accounts, cards, wallets, investments, and policies in one local view.
```

## Two-level source model

CanCan should use a two-level user model:

```text
Money Source
  -> Account / Balance / Instrument Container / Policy / Card / Wallet / Portfolio section
```

Examples:

```text
DBS
  -> Multiplier Account
  -> DBS Visa Card

Wise
  -> SGD Balance
  -> USD Balance

Moomoo
  -> Brokerage Account
     -> USD Cash
     -> Positions from statement

Manulife
  -> Policy
     -> Policy value snapshots
```

## Source taxonomy

`source_type` describes the kind of external financial source or platform.

Recommended source types:

```text
bank
card_provider
wallet
brokerage
insurance
manual
```

Future source types may include more platform categories, such as `crypto_exchange`, after provider specs, fixtures, parser contracts, and source stats are defined.

`account_type` or child container type describes the lower-level object:

```text
deposit_account
credit_card
currency_balance
brokerage_account
cash_balance
position_group
insurance_policy
manual_asset
manual_liability
```

Future account/container types may be added when a new supported provider requires them.

`instrument_type` describes what is being held or valued:

```text
fiat_currency
stock
etf
fund
insurance_policy
liability
```

Future instrument types may include provider-backed token or alternative asset units when needed.

## Display principle

UI should be user-facing, not schema-facing.

Render human language such as DBS Multiplier Account, DBS Visa Card, Wise USD Balance, Moomoo Positions, and Manulife Policy Value.

## Parser mapping principle

Parser output should map evidence to the most specific known user container.

Examples:

```text
DBS bank statement -> DBS / Multiplier Account
DBS credit card statement -> DBS / DBS Visa Card
Wise export -> Wise / currency balance or source-backed FX event
Moomoo statement -> Moomoo / Brokerage Account plus sections from the statement
Manulife statement -> Manulife / Policy
```

If the parser can identify the source but not the child container, create a review item rather than silently committing.

## Acceptance criteria

- MVP onboarding does not ask for base currency.
- Command Center uses Money Overview or Source Overview language, not default Net Worth.
- Multi-currency values are shown in native buckets.
- Source-provided equivalent values can be displayed with evidence.
- Estimated totals require explicit future network activity settings.
- Source model remains two-level from the user's perspective.
- Source type, account or container type, and instrument type are distinct.
- Product copy avoids defensive limitation messaging.
- Parser mapping targets the child container/account whenever possible.
