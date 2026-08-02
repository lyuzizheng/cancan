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

`0013-ledger-assets-valuation.md` owns storage and evidence rules for source-provided equivalent values. Money Overview may display only values permitted by that valuation contract.

## Optional fixed FX-rate estimates

`0013-ledger-assets-valuation.md` owns the optional fixed FX-rate policy. Money Overview may present a converted estimate only when that user setting is enabled and must label the result as estimated with source/timestamp context.

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

## User-configured Money Sources

A Money Source is a user-created provider root and the home for its ingestion configuration. Depending on product-supported capabilities, its channels may include:

```text
provider-specific Gmail search rules
manual PDF/CSV import
a user-selected local or synced CanCan Inbox folder
supported transaction-notification email rules
a product-defined official provider API connector when one is implemented
```

CanCan may ship useful default search rules or official connector setup for supported providers. Users configure and control those channels; they do not create arbitrary live provider implementations in MVP.

Documents discovered through a source channel still pass provider/document classification. If a document matches the configured provider and exposes multiple child accounts, parsing and staging continue and the detected accounts become candidates under that Money Source. Discovery must not stop merely because the child accounts were not entered manually first.

First-seen accounts continue through parsing and staging without interruption. Before the first commit, show one compact review such as `We found 3 accounts`. The user can accept or reject each presentation-safe candidate in that review instead of accepting the exact set as one all-or-nothing decision. Accepted candidates become confirmed accounts and later high-confidence records may reuse those identities without repeating setup. Rejected candidates follow the dismissed lifecycle below.

## Evidence assignment

Capture must not force the user to choose a Money Source or account before CanCan has inspected the evidence. Manual import, drag/drop, Open With, a watched folder, and a generic Gmail Inbox rule may enter with no source assignment or with a non-authoritative source hint.

Resolve in this order:

```text
1. trusted provider/document classification identifies a supported provider
2. exactly one configured Money Source for that provider -> assign it
3. multiple configured Money Sources or conflicting hints -> ask one compact source question
4. no configured source -> offer to create the detected supported source
5. resolve child accounts through the account identity algorithm below
```

The source hint narrows candidates but cannot override a provider fingerprint mismatch. Unassigned or ambiguous evidence projects into Tasks `Needs action` unless the user keeps it parked; it does not require a standalone Evidence Library or generic task authority.

When step 4 detects no configured source, the trusted classifier resolves the versioned composite `money_source_candidates` identity defined by `0002`: provider key plus a tagged exact stable provider root/source ID when supplied, otherwise the tagged provider-scoped singleton. It never concatenates identity strings. Concurrent documents attach to the same candidate. `Keep unassigned` parks that candidate and its linked evidence. `Create source and continue` performs the expected-version recheck, existing-source recheck, at-most-one source creation, candidate confirmation, linked-document routing, and audit append atomically; retries reuse the confirmed source and stale requests reload. Provider fingerprints remain evidence about the parser package/version, not the long-lived source identity by themselves.

If a protected file cannot reveal provider identity, do not ask the user to guess the Money Source before unlocking. The privileged Rust host tries each distinct saved statement password at most once for that parse attempt. Password values and per-password results stay outside renderer, AI, logs, and job state; a successful unlock grants document access only and is never source/account evidence. If none works, project `Password needed` into Tasks `Needs action`; only the user's explicit `Leave parked` decision suppresses that current reason. After user unlock and trusted classification, offer to save or update the password only for the confirmed Money Source.

## Account identity data contract

`accounts` owns the user-visible child account:

```text
id
money_source_id
provider_key
provider_account_id nullable
account_type
display_name
masked_identifier nullable
currency nullable
status = candidate | confirmed | dismissed | archived | merged
merged_into_account_id nullable
raw_identity_json nullable
first_seen_at
last_seen_at
created_at
updated_at
```

When a provider supplies a stable account ID, enforce uniqueness for the non-merged account under:

```text
money_source_id + provider_key + provider_account_id
```

The stable provider ID is private vault data and is never used as display copy. `masked_identifier` is presentation only and must not auto-resolve identity by itself.

When a supported document does not expose a stable provider account ID, preserve its small provider-defined normalized identity projection in `raw_identity_json`. Reuse requires exact equality of that projection under the same Money Source/provider and exactly one matching account. Account type, currency, or masked identifier may be inputs to the projection, but that visible composite alone is not sufficient. If the provider cannot define a projection that reliably distinguishes its supported accounts, the result remains a candidate/review rather than resolving a confirmed account automatically. The user decides each new candidate once before its first commit. Multiple candidates or conflicts require review; never guess, auto-confirm, or auto-merge.

The host validates the current candidate-set version before applying one submitted batch of per-candidate decisions. A stale batch writes nothing. Records tied only to accepted candidates may continue through the normal commit policy.

Rejecting a candidate appends one audited decision against that candidate and proposal version, projects the account to `dismissed`, and hides that candidate plus each uncommitted current record projection whose resolved account is that candidate from ordinary Review and automatic commit. A matched peer owned by an accepted account is not dismissed merely because it referenced the rejected record. The source document, parse run, bounded raw record versions, validation evidence, and audit history remain intact; rejection is not evidence deletion. Re-running the same evidence or the same normalization profile reuses the dismissed identity and does not recreate an active candidate. An explicit user restore returns the account candidate and all of its latest uncommitted current record projections to Review together; superseded versions remain history and are not reactivated. A materially different provider identity projection may create a new candidate. Accepted candidates in the same batch are independent of rejected ones. The renderer never receives the stable provider account ID or raw private identity projection.

Do not introduce a separate identifier table, keyed digests, identifier strength levels, or encryption-key-version coupling until a real supported provider requires more than this contract. That extension must arrive with provider fixtures and resolver tests, not as speculative schema.

## Account resolution algorithm

Resolve in this order:

```text
1. exact verified provider account ID -> existing account
2. exact provider-defined normalized identity projection with exactly one match -> existing account or candidate requiring one confirmation
3. exact match to a dismissed identity -> remain dismissed unless the user explicitly restores it
4. no exact match -> create a new candidate
5. multiple exact matches or conflicting provider IDs -> review; never guess or auto-merge
```

The provider parser emits the stable provider ID or bounded identity inputs; a single resolver owns matching and candidate creation. Re-running the same source evidence is idempotent.

## Rename, merge, and archive

- Renaming changes only `display_name`; identifiers and ledger links remain stable.
- Merging requires confirmation and an audit entry. The secondary account becomes `merged` with `merged_into_account_id`; committed ledger legs are not rewritten, and read models resolve the canonical account without allowing merge cycles.
- Archiving retains provider identity, evidence, and history.
- New evidence for an archived account prompts a compact restore confirmation instead of silently restoring or creating a duplicate.

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

If it identifies multiple child containers, create or update distinct account candidates under the configured Money Source and preserve the source identifiers used for each candidate.

## Acceptance criteria

- MVP onboarding does not ask for base currency.
- Command Center uses Money Overview or Source Overview language, not default Net Worth.
- Multi-currency values are shown in native buckets.
- Source-provided equivalent values can be displayed with evidence.
- FX-converted estimated totals require the explicit optional fixed-rate setting and remain visually distinct from source-backed facts.
- Source model remains two-level from the user's perspective.
- Money Sources are user-configured roots for provider-specific discovery/import channels.
- Capture channels may defer source/account selection; trusted classification routes to one configured Money Source and the existing account resolver handles child accounts.
- Protected unclassified evidence receives one bounded privileged-host saved-password pass before a user prompt; password matching never assigns source/account identity.
- A supported provider can ship default Gmail rules or an official API connector without allowing arbitrary providers.
- Matching documents continue through parsing when they reveal multiple child-account candidates.
- First-seen accounts require one compact per-candidate accept/reject review before their first commit, not before parsing.
- Rejecting a candidate dismisses only that candidate and its uncommitted current projections, preserves source/parse/record/audit evidence, remains idempotent across reparse, and restores the account plus its latest current projections to Review in one explicit action.
- Account identity prefers a stable provider account ID; bounded candidate inputs require confirmation and never use display names or masked suffixes alone as automatic identity.
- Rename preserves identity; merge and archive behavior are explicit, audited, and do not rewrite committed ledger legs.
- Source type, account or container type, and instrument type are distinct.
- Product copy avoids defensive limitation messaging.
- Parser mapping targets the child container/account whenever possible.
