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

## Markdown wireframe

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ CanCan                         Search / Command                    AI Ask   │
├───────────────┬───────────────────────────────────────────────┬──────────────┤
│ Sidebar       │ Command Center                                │ Right Rail   │
│               │                                               │              │
│ ● Overview    │ ┌───────────────────────────────────────────┐ │ AI Insight   │
│ Sources       │ │ Net Worth      Freshness      Last Backup │ │ ┌──────────┐ │
│ Assets        │ │ SGD xxx        92% fresh      Today       │ │ │Monthly   │ │
│ Transactions  │ └───────────────────────────────────────────┘ │ │summary   │ │
│ Reconcile     │                                               │ └──────────┘ │
│ Money Flow    │ ┌───────────────────────────────────────────┐ │              │
│ Library       │ │ Money Source Activity                     │ │ Review       │
│ Jobs          │ │                                           │ │ ┌──────────┐ │
│ Settings      │ │ DBS Bank      balance / latest movement   │ │ │Needs 8   │ │
│               │ │ DBS Card      liability / payment status  │ │ │Failed 1  │ │
│ AI Assistant  │ │ UOB Bank      inflow / outflow / freshness│ │ │New 12    │ │
│               │ │ Wise          SGD/USD / FX / top-ups      │ │ └──────────┘ │
│               │ └───────────────────────────────────────────┘ │              │
│               │                                               │ Backup       │
│               │ ┌───────────────────────────────────────────┐ │ ┌──────────┐ │
│               │ │ Recent Flows                              │ │ │Encrypted │ │
│               │ │ UOB -> DBS Card payment                   │ │ │Folder OK │ │
│               │ │ DBS -> Wise top-up -> FX                  │ │ └──────────┘ │
│               │ └───────────────────────────────────────────┘ │              │
│               │                                               │              │
│               │ ┌───────────────────────────────────────────┐ │              │
│               │ │ Lower Work Queue                          │ │              │
│               │ │ Needs Review | New Evidence | Failed Jobs │ │              │
│               │ └───────────────────────────────────────────┘ │              │
└───────────────┴───────────────────────────────────────────────┴──────────────┘
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
Claude-style retro parchment templates
```

Prefer:

```text
clear navigation
modern warm off-white + green semantic palette
dense but readable tables
source/account status chips
precise typography
compact asset summaries
animated but meaningful state transitions
side-by-side review surfaces when needed
excellent empty/loading/error states
```

## Component library strategy

Use a modern React component foundation to avoid exploding code volume.

Recommended direction:

```text
Base components: Hero UI or shadcn/Radix-style primitives
Styling: Tailwind-compatible token system
Charts: Recharts or lightweight visx-style components when needed
Tables: TanStack Table for complex tables
Icons: lucide-react
Motion: motion/react or CSS transitions for meaningful state changes
```

Preferred default: **Hero UI for application components**, with CanCan-owned design tokens and wrappers so the app does not look like an unmodified library demo.

Rules:

- Do not fork or hand-roll basic controls if a solid component exists.
- Wrap library components in `packages/ui` CanCan components.
- Keep theme tokens centralized.
- Avoid mixing multiple visual systems.
- DaisyUI can be used for prototyping inspiration, but should not define final brand identity.
- Paid UI kits are acceptable only if they do not lock the app into unmaintainable patterns; document the choice before adopting.

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

## Figma prototype option

A Figma prototype can be generated after visual tokens are accepted. The Figma output should follow this spec and `0011-visual-design-tokens.md`, then any approved Figma decisions should be copied back into docs.

## Acceptance criteria

- First screen has left sidebar and main body.
- Source activity, overview, freshness, AI insight, and review status are all represented.
- Asset snapshot is present but not the only focus.
- Different money source types can display different stats.
- Empty states guide the user to create sources, configure Gmail, or import files.
- AI Assistant is represented as a future-ready surface/tool entry, but cannot bypass safety boundaries.
