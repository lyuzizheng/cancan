# 07. UI Information Architecture

## Product UI principle

CanCan is a reconciliation console and financial evidence vault.

The main question is:

```text
What do I own, where is it, what changed, and which source proves it?
```

## Main navigation

```text
Home
Sources
Assets
Transactions
Reconciliation
Money Flow
Library
Jobs
Settings
```

## Home

Purpose:

```text
high-level financial status and operational health
```

Cards:

```text
Total net worth
Cash
Investments
Crypto
Insurance value
Liabilities
Unreconciled amount
Needs review count
Data freshness
Last sync
Last backup
```

Filters/toggles:

```text
By source
By money type
By currency
By account
```

## Sources

Purpose:

```text
view each finance source and its current status
```

Source card fields:

```text
source name
source type
total value in base currency
native balances
last sync
last statement date
unmatched count
parser errors
credential status
backup/document count
```

Source detail tabs:

```text
Overview
Accounts
Transactions
Holdings
Documents
Sync Runs
Reconciliation
Settings
```

## Assets

Purpose:

```text
show current assets and liabilities by money type and instrument
```

Sections:

```text
Cash
Credit card liabilities
Stocks / ETFs
Crypto
Insurance / policy values
Funds
Other assets
Other liabilities
```

Each asset row should show:

```text
instrument
quantity or balance
native currency
value in base currency
source
account
last valuation time
source document/API snapshot
confidence/freshness
```

## Transactions / Events

Purpose:

```text
show canonical ledger events, not just expenses
```

Columns:

```text
date
source
account
event type
money type
instrument
amount / quantity
currency
description
ledger impact
match status
source document
confidence
```

Event types:

```text
purchase
income
bank transfer
card payment
top-up
FX conversion
broker deposit
trade
crypto deposit/withdrawal
insurance premium
balance snapshot
valuation snapshot
fee
interest
```

## Reconciliation

Purpose:

```text
review uncertain parse and match results
```

Sections:

```text
Possible duplicates
Possible transfers
Possible top-ups
Credit card payments
Broker deposits
FX conversions
Refunds/reimbursements
Insurance premiums
Unmatched records
Parser warnings
```

Candidate card:

```text
left record
right record(s)
proposed match type
confidence
evidence
AI explanation
rule explanation
source document links
actions: confirm, reject, edit, link manually
```

## Money Flow

Purpose:

```text
show chains of linked events across sources
```

Example:

```text
DBS -1000 SGD
  -> Wise +1000 SGD
  -> Wise FX SGD/USD
  -> Moomoo +740 USD
  -> AAPL trade
```

Views:

```text
single chain view
source-to-source flow summary
unreconciled flow endpoints
monthly money movement graph
```

## Library

Purpose:

```text
source evidence and audit trail
```

Library item types:

```text
PDF statement
email
CSV/XLSX export
screenshot
API JSON snapshot
OCR output
parsed table
parse run
validation report
```

Document detail tabs:

```text
Preview
Metadata
Text/OCR
Tables
Parse Runs
Parsed Records
Linked Ledger Events
Validation
Audit Log
```

## Jobs

Purpose:

```text
operational visibility for sync, parse, import, backup
```

Shows:

```text
running jobs
paused jobs
failed jobs
completed jobs
job timeline
retry buttons
continue parsing button
```

## Settings

Sections:

```text
Vault
AI providers
Sources/plugins
Accounts
Security
Backup
iCloud
Parser versions
Developer tools
```

## MVP UI priority

Build in this order:

```text
1. Vault setup/unlock
2. Library import and document list
3. Parser run detail
4. Staged records table
5. Review inbox
6. Accounts and source detail
7. Assets dashboard
8. Money flow graph
9. Backup settings
```
