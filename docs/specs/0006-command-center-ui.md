# 0006. Command Center UI Spec

## Goal

Design the first screen as a polished asset-management and reconciliation app, not a marketing dashboard.

## Layout

Use a left sidebar and right main body.

```text
Left sidebar:
- Command Center
- Sources
- Assets
- Transactions
- Reconciliation
- Money Flow
- Library
- Jobs
- Settings
- AI Assistant

Main body:
- top status/header
- money source activity overview
- compact asset snapshot
- evidence freshness
- AI insight
- review/action modules
```

## Design register

CanCan should feel like a professional finance operations console with consumer-grade polish and 2026-level interaction quality.

Avoid:

```text
giant fintech hero cards
decorative gradients
glassmorphism as default
busy card grids
cute illustrations
default component-library appearance
```

Prefer:

```text
clear navigation
modern warm + green palette
dense but readable tables
source/account status chips
precise typography
compact asset summaries
animated but meaningful state transitions
side-by-side review surfaces when needed
excellent empty/loading/error states
```

## Command Center zones

The first viewport should be multi-dimensional but not too dense.

Recommended priority:

```text
1. Top overview: net worth, freshness, last scan, vault/backup health
2. Main stream: money source activity and recent flows
3. Source snapshots: tailored status per source type
4. AI insight: monthly/weekly summary or missing statement prompt
5. Compact review status: needs review, failed jobs, suggested links
6. Lower section: detailed Needs Review / New Evidence / Failed Jobs
```

## Source-specific modules

Different source types should not all render the same generic card.

Examples:

```text
Bank account: balance, inflow/outflow, statement freshness
Credit card: liability, statement period, repayment status, spending trend
Wise/wallet: balances by currency, top-ups, FX conversions
Brokerage: cash, positions, valuation, estimated P/L
Crypto: token balances, valuation, deposits/withdrawals
Insurance: policy value, premium history, valuation date
```

## AI Assistant direction

Future assistant should access data through backend APIs/skills, not direct database or filesystem access.

Assistant tools should be narrow:

```text
get_asset_summary()
get_monthly_summary(month)
list_review_items(status)
explain_money_flow(chain_id)
search_transactions(query)
get_source_freshness()
list_missing_statements()
```

## Review surface

Keep review lightweight:

- list/table for scanning;
- expand row for details;
- side-by-side comparison only when matching/linking is involved;
- show AI explanation and rule evidence separately;
- keep confirm/reject/edit actions visible.

## Money Flow UI

First version is chain-first, not a complex graph canvas.

Backend should still model graph relationships so node-edge visualization can be added later.

## Acceptance criteria

- First screen has left sidebar and main body.
- Source activity, overview, freshness, AI insight, and review status are all represented.
- Asset snapshot is present but not the only focus.
- Different money source types can display different stats.
- Empty states guide the user to create sources, configure Gmail, or import files.
- AI Assistant is represented as a future-ready surface/tool entry, but cannot bypass safety boundaries.
