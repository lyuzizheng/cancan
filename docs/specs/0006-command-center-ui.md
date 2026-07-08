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
- operational queue
- asset snapshot
- evidence freshness
- review/action modules
```

## Design register

CanCan should feel like a professional finance operations console with consumer-grade polish.

Avoid:

```text
giant fintech hero cards
decorative gradients
glassmorphism as default
busy card grids
cute illustrations
```

Prefer:

```text
clear navigation
dense but readable tables
source/account status chips
precise typography
compact asset summaries
side-by-side review surfaces
calm accent color
excellent empty/loading/error states
```

## Command Center zones

```text
1. Vault and sync status
2. Needs Review queue
3. New Evidence from Gmail/import
4. Failed or paused jobs
5. Suggested Links
6. Compact Asset Snapshot
7. Freshness and Backup status
8. AI Assistant entry point
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
```

The assistant can answer questions like:

```text
What changed this month?
Why did net worth move?
Which statements are missing?
Which transactions still need review?
Summarize my DBS/UOB/Wise activity.
```

It must not:

```text
read secrets
commit ledger directly
make payments
place trades
withdraw crypto
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

Example:

```text
UOB One Account -1000 SGD
-> DBS Visa credit card payment
-> DBS Visa liability -1000 SGD
```

Keep backend graph capability so node-edge visualization can be added later.

## Acceptance criteria

- First screen has left sidebar and main body.
- Review and evidence status are visible without hunting.
- Asset snapshot is present but not the only focus.
- Empty states guide the user to create sources, import files, or configure Gmail.
- AI Assistant is represented as a future-ready surface/tool entry, but cannot bypass safety boundaries.
