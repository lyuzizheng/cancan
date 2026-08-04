# 0006. Command Center UI Spec

## Goal

Design the first screen as a polished personal finance and account-record workspace, not professional accounting software, an enterprise reconciliation console, or a marketing dashboard.

## Layout

Use a left sidebar and right main body.

```text
Left sidebar:
- Command Center
- Tasks
- Sources
- Assets
- Transactions
- Review
- Money Flow
- Settings
- AI Assistant

Main body:
- top status/header
- one unified Tasks section
- money source activity overview
- compact asset snapshot
- evidence freshness/timestamps
- AI insight
- owning detail surfaces reached from Tasks deep links
```

## Markdown wireframe

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ CanCan                         Search / Command                    AI Ask     │
├───────────────┬───────────────────────────────────────────────┬──────────────┤
│ Sidebar       │ Command Center                                │ Right Rail   │
│               │                                               │              │
│ ● Overview    │ ┌───────────────────────────────────────────┐ │ AI Insight   │
│ Tasks      3  │ │ Overview: native totals, last updates     │ │ Monthly      │
│ Sources       │ │ SGD cash | USD cash | liabilities | etc.  │ │ summary      │
│ Assets        │ └───────────────────────────────────────────┘ │              │
│ Transactions  │                                               │ Backup       │
│ Review        │ ┌───────────────────────────────────────────┐ │ Encrypted    │
│ Money Flow    │ │ Tasks                         View all     │ │ Folder OK    │
│ Settings      │ │ Needs action 3                            │ │              │
│               │ │  Password needed · August statement    › │ │              │
│ AI Assistant  │ │  New source detected                   › │ │              │
│               │ │ In progress · Processing 2 statements    │ │              │
│               │ │ Recently · 3 files already in CanCan   › │ │              │
│               │ └───────────────────────────────────────────┘ │              │
│               │                                               │              │
│               │ ┌───────────────────────────────────────────┐ │              │
│               │ │ Money Source Activity                     │ │              │
│               │ │ DBS Bank      balance / latest movement   │ │              │
│               │ │ DBS Card      liability / payment status  │ │              │
│               │ │ UOB Bank      inflow / outflow / updated  │ │              │
│               │ │ Wise          SGD/USD / FX / top-ups      │ │              │
│               │ └───────────────────────────────────────────┘ │              │
│               │                                               │              │
│               │ ┌───────────────────────────────────────────┐ │              │
│               │ │ Recent Flows                              │ │              │
│               │ │ UOB -> DBS Card payment                   │ │              │
│               │ │ DBS -> Wise top-up -> FX                  │ │              │
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
1. Top overview: native totals, timestamps, vault/backup health
2. One compact Tasks section for actionable, processing, and recent work
3. Main stream: money source activity and recent flows
4. Source snapshots: tailored status per source type
5. Insight: source-backed monthly/weekly summary; missing-statement insight only after the separate statement-coverage capability is enabled
```

`Tasks` is the one user-facing work projection. It replaces separate first-screen `To do`, `Latest intake`, `Needs attention`, Review-status, and Jobs cards. It does not create a generic task authority or copy domain state: evidence, source confirmation, Review, security/setup, backup, and job owners remain authoritative, while the host derives safe task presentation rows and deep links from them.

The privileged host freezes one presentation-safe Tasks read contract before renderer integration. Each returned row contains only a deterministic opaque presentation key, group, human title and consequence, safe timestamp, destination kind, and the minimum opaque destination identity required by the owning route. The host, not the renderer, derives ordering, badges, readiness, parking, recent expiry, and batch completion. The contract returns no path, bookmark, locator, hash, raw provider identity, secret, financial payload copied from Review, or generic mutation token. User actions call the existing owner-specific command; there is no `complete_task`, `dismiss_task`, or `update_task` command.

The section groups one list model by user consequence:

```text
Needs action       user input is required; this group alone contributes to badges/counts
In progress        work continues without user input; never contributes to badges/counts
Recently completed
                   completed non-actionable receipts from the last 168 hours, including visible Add failures
```

Command Center renders at most five task rows total, ordered by group priority. Within `Needs action`, show the oldest unresolved item first so it cannot starve; within `In progress` and `Recently completed`, show newest first. The latest intake batch supplies the first eligible recent rows. `View all` opens the full `Tasks` route with `Needs action`, `In progress`, `Recently completed`, and `Parked` filters. A calm empty state reads `You're all caught up` and may show the last successful intake time.

Each row has one primary click/tap target and a human title, concise consequence, state, safe timestamp, and destination. It deep-links to the exact Password, Source confirmation, Document, Review group, Recovery/Settings, or other owning surface. Returning restores the Tasks filter and scroll position. A Review task may aggregate one statement's related records, but Review remains the decision authority and detailed destination; do not render a separate Review-status card on Command Center.

Unresolved task behavior follows the owning domain:

```text
protected document       Enter password / Leave parked
unknown supported source Create source and continue / Choose an existing source / View document / Keep unassigned
deleted exact re-import  Restore source file / Leave deleted
ordinary setup           complete now / Remind me later for seven calendar days
financial Review         resolve in Review; no dismiss or fake completion
```

Parking removes the item from Command Center and the actionable badge but preserves it under the full Tasks `Parked` filter and its Source/Documents owner. A setup reminder becomes actionable again when due. Neither behavior is implemented as a generic task-row mutation; the owning domain persists the parked, unassigned, or reminder decision.

Time behavior is deterministic. When the user selects `Remind me later`, the host uses the current macOS calendar/time zone to add seven calendar days while preserving the local wall-clock time, then stores the resulting UTC due instant; a later time-zone change does not rewrite that decision. `Recently completed` instead uses elapsed time and includes outcomes whose UTC completion instant is within the last 168 hours. Renderer copy formats both instants in the current local time zone.

Successful, updated, exact-duplicate, same-content, no-new-record, and explicit visible `File not added` outcomes appear only under `Recently completed`, not as actionable work. Full Tasks keeps them for 168 hours; Command Center shows only rows from the latest eligible batch. Exact-duplicate-only batches remain silent under the notification policy.

The sidebar `Tasks` item replaces the inert `Jobs` marker and shows only the `Needs action` count. The durable job engine remains internal reliability infrastructure. Advanced job history, retry/cancel controls, and redacted technical detail may be reached from Settings or a diagnostic route when implemented; normal users never need to choose a technical job to complete work.

At narrow widths the same information architecture becomes one column without alternate states or mobile-only labels. Rows remain fully tappable, do not rely on hover, preserve a clear focus indicator, expose state in accessible text rather than color alone, and replace nonessential progress motion under reduced motion.

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

The global `Add` action accepts files without asking for source/account first. Its processing, recent, password, source-confirmation, folder-capture, and missing-period projections appear in the unified Tasks section and deep-link to the owning evidence/source surface; they do not create a separate Inbox navigation module.

## AI Assistant direction

The Phase 2 renderer, assistant, app-internal AI features, internal scheduled jobs, full first-party AI CLI, and external-agent tools share the host-owned backend capability interface in `0019-ai-capability-platform-and-cli.md`. Its target catalog covers every existing user-facing read plus host-validated edit/link/commit/delete/restore outcomes. Tauri/in-process/socket/tool layers are adapters over that interface. AI accesses data through registered purpose-specific product capabilities, not direct database or filesystem access.

Assistant tools should be narrow:

```text
get_asset_summary()
get_monthly_summary(month)
list_review_items(status)
explain_money_flow(chain_id)
search_transactions(query)
get_source_updated_at()
analyse_statement_coverage(scope)
```

Each feature adds one bounded capability contract instead of a generic source-data dump. The AI returns an advisory result with source references; it cannot mutate sources, review decisions, relationships, jobs, or ledger state except through an explicitly registered, host-validated, user-approved mutation capability.

## Data loading and failure isolation

Core unlocked views must remain usable when an optional capability fails. Overview, Review, Activity, and Sources load independently from Local Inbox status, statement coverage, future Gmail, backup, and AI insight. An optional capability error stays in its owning module with a retry/action and does not reject or clear an otherwise successful core finance refresh. Do not silently swallow the failure or turn every view into one all-or-nothing request.

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
- Source activity, overview, timestamps, AI insight, and relevant Review work are all represented without a separate Review-status card.
- Different money source types can display different stats.
- UI avoids overcomplicated freshness scores or noisy tags.
- Default review and relationship UI uses a low-complexity personal-finance mental model with detail available on demand.
- User-facing presentation models remain separate from internal ledger and audit structures.
- Empty states guide the user to add files, choose an Inbox folder, or optionally configure Gmail after creating sources.
- Evidence documents are reached through Source detail rather than a standalone Library sidebar item.
- AI Assistant is represented as a future-ready surface/tool entry, but cannot bypass safety boundaries.
- Background AI features use narrow read APIs and advisory outputs rather than direct source/database/filesystem authority.
- Failure in an optional capability does not blank or stale an otherwise successful core finance view.
- One unified Tasks projection replaces separate `To do`, `Latest intake`, `Needs attention`, Review-status, and Jobs modules on Command Center.
- Command Center shows at most five rows; `View all` opens the full Tasks route, and only `Needs action` contributes to its badge.
- Every row deep-links to its exact owning surface and returns to the prior Tasks context; the projection owns no domain state or generic task table.
- In-progress work is non-actionable, recent success/no-op outcomes expire after 168 hours, and exact-duplicate-only batches remain silent.
- Parking and setup reminders are persisted by their owning domains, while unresolved financial Review work cannot be dismissed through Tasks.
- Desktop and narrow layouts share the same task states, ordering, labels, deep links, accessible focus, and reduced-motion behavior.
