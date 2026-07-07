# 07. UI Information Architecture

## Product UI principle

CanCan is a reconciliation console and financial evidence vault.

The main question is:

```text
What do I own, where is it, what changed, which source proves it, and what needs review?
```

## Visual direction

CanCan should feel like a polished finance operations console, not a generic fintech landing dashboard.

Design attributes:

```text
calm
precise
dense but readable
auditable
trustworthy
fast to scan
beautiful without decorative noise
```

Avoid relying on oversized hero metrics, glossy cards, decorative gradients, or consumer-fintech fluff. Use strong table design, clear hierarchy, crisp source/status indicators, and restrained accent color.

## Main navigation

```text
Command Center
Sources
Assets
Transactions
Reconciliation
Money Flow
Library
Jobs
Settings
```

## Command Center

The MVP home screen should be an operational command center with asset summary embedded, not a pure dashboard.

Purpose:

```text
show financial status, evidence pipeline health, and next actions
```

Primary zones:

```text
left: source rail with freshness/status
center: needs review, new evidence, failed parses, suggested links
right: compact asset snapshot and backup/vault health
```

Key metrics:

```text
Total net worth in base currency
Cash
Investments
Crypto
Insurance value
Liabilities
Unreconciled amount
Needs review count
New evidence count
Data freshness
Last Gmail scan
Last backup
```

## Sources

Purpose:

```text
view each money source and its sub-accounts
```

Source fields:

```text
source name
source type
total value in base currency
native balances
last sync/statement date
unmatched count
parser errors
credential status
document count
```

Source detail tabs:

```text
Overview
Sub-accounts
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

Each row should show:

```text
instrument
quantity or balance
native currency
value in base currency
source
sub-account
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
sub-account
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
Account mapping suggestions
```

Candidate card/table detail:

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
native text extraction
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

## Settings

Sections:

```text
Vault
Base currency
AI providers
Gmail rules
Sources/plugins
Money sources and sub-accounts
Security
Backup
iCloud / folder backup
Parser versions
Developer tools
```

## MVP UI priority

Build in this order:

```text
1. Vault setup/unlock
2. Money source and sub-account setup
3. Manual import test harness
4. Gmail rule setup and scan status
5. Library document list/detail
6. Parser run detail
7. Staged records table
8. Review inbox
9. Source detail and asset summary
10. Money Flow graph
11. Backup settings
```
