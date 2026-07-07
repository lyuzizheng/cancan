# 00. Product Vision

## Product thesis

CanCan is a personal, local-first finance vault for people whose financial life is spread across banks, cards, Wise/Revolut, brokerages, crypto exchanges, insurance providers, emails, PDFs, and exported statements.

The main job is **not** budgeting. The main job is:

1. collect source evidence;
2. parse it into canonical records;
3. reconcile the same money movement across different sources;
4. explain what was matched, duplicated, or still needs review;
5. show the user's assets by source, money type, currency, and instrument.

## Target user

A power user who wants local control of sensitive financial data and is willing to run a desktop app manually every day or every few days.

The user may have:

- DBS, UOB, HSBC, MariBank, Trust Bank accounts and cards;
- Wise and Revolut balances;
- payment app statements;
- Moomoo brokerage holdings and daily statements;
- Bitget crypto assets and transactions;
- Manulife insurance/policy values;
- Gmail emails with PDF/CSV statements;
- manually exported PDFs, CSVs, screenshots, and reports.

## Product shape

CanCan should feel like a **financial operations console**, not a generic personal finance dashboard.

The main user loop is:

```text
Open app
-> unlock local vault
-> resume unfinished jobs
-> scan enabled sources/plugins
-> import new raw evidence
-> parse documents/API responses
-> stage canonical records
-> generate reconciliation candidates
-> review ambiguous items
-> commit confirmed records
-> create encrypted iCloud backup snapshot
```

## Primary capabilities

### Source library

Every raw source is kept in a local library:

- Gmail messages;
- PDF statements;
- CSV/XLSX exports;
- screenshots/images;
- API JSON snapshots;
- parser text/OCR output;
- parsed tables;
- validation results;
- links to ledger events.

### Asset source of truth

The app should answer:

- How much money do I have at each finance source?
- How much cash, stock, crypto, insurance value, and liabilities do I have?
- Which balances are fresh, stale, or unverified?
- Which source documents prove the current balance?

### Reconciliation

The app should identify:

- duplicate imports;
- bank-to-bank transfers;
- credit card repayments;
- Wise/Revolut/payment app top-ups;
- FX conversions;
- broker deposits;
- crypto deposits/withdrawals;
- refunds and reimbursements;
- insurance premium payments;
- money-flow chains across multiple providers.

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

CanCan is closer to a local financial ETL/reconciliation vault than a simple expense tracker.

Traditional app center:

```text
Transaction -> Category -> Budget -> Report
```

CanCan center:

```text
Source -> Evidence -> Parse -> Normalize -> Reconcile -> Review -> Ledger
```
