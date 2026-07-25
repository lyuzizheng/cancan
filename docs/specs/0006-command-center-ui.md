# 0006. Command Center UI Spec

## Goal

Design the first screen as a polished personal finance and account-record workspace, not professional accounting software, an enterprise reconciliation console, or a marketing dashboard.

## Layout

Use a left sidebar and right main body.

```text
Left sidebar:
- Command Center
- Sources
- Assets
- Transactions
- Review
- Money Flow
- Jobs
- Settings
- AI Assistant

Main body:
- top status/header
- money source activity overview
- compact asset snapshot
- evidence freshness/timestamps
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
│ Sources       │ │ Overview: native totals, last updates     │ │ ┌──────────┐ │
│ Assets        │ │ SGD cash | USD cash | liabilities | etc.  │ │ │Monthly   │ │
│ Transactions  │ └───────────────────────────────────────────┘ │ │summary   │ │
│ Review        │                                               │ └──────────┘ │
│ Money Flow    │ ┌───────────────────────────────────────────┐ │              │
│ Jobs          │ │ Money Source Activity                     │ │ Review       │
│ Settings      │ │                                           │ │ ┌──────────┐ │
│               │ │ DBS Bank      balance / latest movement   │ │ │Needs 8   │ │
│               │ │ DBS Card      liability / payment status  │ │ │Failed 1  │ │
│ AI Assistant  │ │ UOB Bank      inflow / outflow / updated  │ │ │New 12    │ │
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

## Dashboard philosophy

Do not overcomplicate the dashboard with too many generated metrics, tags, or AI-invented labels.

Show user-facing facts:

- source-native balances;
- source-provided valuations;
- last updated/imported/parsed timestamps;
- clear unresolved/review states;
- recent money flows;
- AI insight only when grounded in source data.

Avoid confusing aggregate scores like `92% fresh` unless later backed by a clear definition. Prefer `Updated 2h ago`, `Statement through Jun 30`, `Missing July statement`, or `Needs review`.

## Design register

CanCan should feel like a refined, future-facing personal finance workspace with consumer-grade simplicity and 2026-level interaction quality. Financial detail remains available on demand without becoming the default mental model.

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
polished cards and lists with progressive disclosure
subtle source/account states
precise typography
compact asset summaries
animated but meaningful state transitions
side-by-side review surfaces when needed
excellent empty/loading/error states
```

## Component library strategy

Kimi Code CLI owns the renderer implementation choice. It may use Hero UI, shadcn/Radix-style primitives, or a lighter code-native approach, but there is no preferred library before a concrete screen needs it.

Choose the smallest pinned dependency set that satisfies the accepted design and accessibility contract. Reuse solid basic controls instead of hand-rolling them, but do not add wrappers, tables, charts, icons, motion libraries, or styling systems speculatively. Shared CanCan components belong in `packages/ui` only after real repeated use exists.
- Keep theme tokens centralized.
- Avoid mixing multiple visual systems.
- DaisyUI can be used for prototyping inspiration, but should not define final brand identity.
- Paid UI kits are acceptable only if they do not lock the app into unmaintainable patterns; document the choice before adopting.

## Command Center zones

The first viewport should be multi-dimensional but not too dense.

Recommended priority:

```text
1. Compact To do banner list for incomplete safety/setup actions
2. Top overview: native totals, timestamps, vault/backup health
3. Main stream: money source activity and recent flows
4. Source snapshots: tailored status per source type
5. Insight: monthly/weekly summary or deterministic missing-statement prompt
6. Compact review status: needs review, failed jobs, suggested links
7. Lower section: detailed Needs Review / New Evidence / Failed Jobs
```

The `To do` list is action-oriented rather than a notification feed. A deferred recovery-file save appears here with a direct `Save recovery file` action and remains until the host confirms both the external file and Vault-side configured state were saved. It must not block entry into Command Center.

## Source-specific modules

Different source types should not all render the same generic card.

Examples:

```text
Bank account: balance, inflow/outflow, statement updated at
Credit card: liability, statement period, repayment status, spending trend if source-backed
Wise/wallet: balances by currency, top-ups, FX conversions from source evidence
Brokerage: cash, positions, source-provided valuation/P&L if present
Future crypto source: token balances, source-provided valuation if present
Insurance: policy value, premium history, valuation date
```

## Evidence navigation

Evidence documents live under each Money Source detail view. Do not add a dominant standalone Library navigation item in MVP. The canonical document UX is `0017-evidence-documents-source-ux.md`.

The global `Add` action accepts files without asking for source/account first. Unassigned imports, folder-capture problems, and missing-period prompts appear as compact `Needs attention` cards; they do not create a separate Inbox navigation module.

## AI Assistant direction

Future assistant should access data through backend APIs/skills, not direct database or filesystem access.

Assistant tools should be narrow:

```text
get_asset_summary()
get_monthly_summary(month)
list_review_items(status)
explain_money_flow(chain_id)
search_transactions(query)
get_source_updated_at()
list_missing_statements()
```

## Review surface

Keep review lightweight:

- list/table for scanning;
- expand row for details;
- side-by-side comparison only when matching/linking is involved;
- show AI explanation and rule evidence separately;
- keep confirm/reject/edit actions visible.

Use personal, action-oriented language such as `Looks related` and `Needs your check`. Do not expose match edges, allocations, posting classes, or reconciliation jargon in the default view.

## Presentation and future sharing

Design user-facing summaries, source cards, timelines, and money-flow views as presentation-ready surfaces rather than thin renderings of internal audit tables.

Keep presentation models separate from internal ledger/reconciliation structures so a future explicit sharing or export feature can reuse clear human-readable views without exposing raw evidence, secrets, hidden technical metadata, or internal matching complexity. Sharing itself is not an MVP feature.

## Money Flow UI

First version is chain-first, not a complex graph canvas.

Backend should still model graph relationships so node-edge visualization can be added later.

## Figma prototype option

Kimi may generate a Figma prototype when it materially helps renderer craft, or omit it. Figma is exploratory rather than authoritative: it follows this spec and `0011-visual-design-tokens.md`, and accepted product/design changes must be copied into canonical docs and production code.

## Acceptance criteria

- First screen has left sidebar and main body.
- Source activity, overview, timestamps, AI insight, and review status are all represented.
- Different money source types can display different stats.
- UI avoids overcomplicated freshness scores or noisy tags.
- Default review and relationship UI uses a low-complexity personal-finance mental model with detail available on demand.
- User-facing presentation models remain separate from internal ledger and audit structures.
- Empty states guide the user to add files, choose an Inbox folder, or optionally configure Gmail after creating sources.
- Evidence documents are reached through Source detail rather than a standalone Library sidebar item.
- AI Assistant is represented as a future-ready surface/tool entry, but cannot bypass safety boundaries.
- Deferred safety/setup work appears as a compact actionable `To do` banner list rather than blocking onboarding.
