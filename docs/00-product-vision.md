# 00. Product Vision

## Product thesis

CanCan is a personal, local-first finance vault for people whose financial life is spread across banks, cards, Wise/Revolut, brokerages, crypto exchanges, insurance providers, emails, PDFs, and exported statements.

The main job is **not** budgeting. The main job is to automatically collect financial evidence, parse it into canonical records, reconcile the same money movement across different sources, explain what was matched or still needs review, and show assets by source, account, money type, currency, and instrument.

## Target user

The target user is a power user who wants local control of sensitive financial data and is willing to run a desktop app manually every day or every few days, but expects the app to automate collection and parsing once sources are configured.

The user may have:

- DBS and UOB bank account statements;
- DBS and UOB credit card statements;
- Wise balances, exports, or statements;
- other Singapore bank, card, and wallet statements;
- Moomoo brokerage holdings, trades, and daily/monthly statements;
- crypto exchange assets and transactions;
- Manulife or other insurance/policy values;
- Gmail messages with statement attachments;
- manually exported PDFs, CSVs, screenshots, and reports.

## MVP success criterion

The first real product success is **Gmail-first statement automation**, not only manual import.

The MVP should prove this loop:

```text
User configures local vault
-> user creates Money Sources and sub-accounts
-> user configures Gmail read-only search rules
-> app incrementally discovers statement emails since a chosen date
-> app downloads PDF/CSV attachments into local encrypted evidence vault
-> app performs native PDF extraction and OCR layer
-> app asks AI to normalize records into strict schemas
-> deterministic validation checks the output
-> app maps records to existing source/sub-account candidates
-> app detects duplicates and cross-source links
-> low-risk standalone purchases can commit by policy
-> transfers, repayments, top-ups, and ambiguous matches enter Review
-> user confirms review items
-> committed ledger updates assets, transactions, balances, and money-flow graph
```

Manual PDF/CSV import remains important, but it is a **test harness, recovery path, and local fallback**, not the product's definition of success.

## Product shape

CanCan should feel like a **financial operations console**: calm, trustworthy, dense enough for review work, and visually polished enough to feel like a serious finance product.

It should not feel like a generic budgeting app. Its core promise is not categories and spending limits. Its core promise is:

```text
What do I own?
Where is it?
What changed?
Which source proves it?
Which records are duplicates, transfers, repayments, or still unresolved?
```

## Primary capabilities

### Source library

Every raw source is kept in a local evidence library:

- Gmail messages and metadata;
- PDF statements;
- CSV/XLSX exports;
- screenshots/images;
- API JSON snapshots;
- native PDF text extraction;
- OCR output;
- parsed tables;
- validation results;
- links to external records and ledger events.

### Asset source of truth

The app should answer:

- How much money do I have at each finance source?
- How much cash, credit card liability, stock, ETF, crypto, insurance value, and fund value do I have?
- What is the native currency/instrument and what is the base-currency value?
- Which balances are fresh, stale, unverified, or contradicted by source evidence?
- Which document/API snapshot proves the current balance?

### Reconciliation

The app should identify:

- duplicate imports;
- duplicate transactions across API/export/PDF evidence;
- bank-to-bank transfers;
- credit card repayments, including repayments from another bank;
- Wise/Revolut/payment app top-ups;
- FX conversions;
- broker deposits;
- stock/ETF buy and sell transactions;
- crypto deposits and withdrawals;
- refunds and reimbursements;
- insurance premium payments;
- money-flow chains across multiple providers.

## MVP provider scope

The first provider set should include at least:

```text
DBS bank account statement
DBS credit card statement
UOB bank account statement
UOB credit card statement
Wise PDF/CSV/export, with API later if useful
```

Each provider must be backed by real sample files before it is treated as implemented.

## Non-goals for MVP

The first version should not focus on:

- full budgeting;
- retirement planning;
- family/household collaboration;
- advisor-style recommendations;
- automated money movement;
- placing trades;
- paying bills;
- bypassing MFA or CAPTCHAs;
- real-time multi-device sync;
- server-side SaaS hosting.

## Positioning

Traditional app center:

```text
Transaction -> Category -> Budget -> Report
```

CanCan center:

```text
Source -> Evidence -> Extract -> AI Normalize -> Validate -> Reconcile -> Review -> Ledger
```
