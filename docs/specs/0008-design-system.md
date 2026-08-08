# 0008. Design System Spec

## Goal

Define CanCan's visual design direction before UI implementation.

## Implementation ownership

The core visual direction, semantic palette, typography direction, and motion rhythm are accepted. Kimi Code CLI owns customer-facing renderer craft: it may select the component foundation and exact pinned versions, decide whether Figma is useful as an exploratory artifact or omit it, and structure the renderer implementation freely within the accepted visual, accessibility, privacy, and product-state contracts.

The production code and canonical specs remain authoritative. Figma is never required as an implementation input and cannot redefine finance, evidence, job, or security behavior. Component and Figma choices therefore do not block backend business-logic work or the `review-ledger-ui` slice.

CanCan should feel like a 2026 personal finance and account-record workspace: futuristic, classic, technical, secure, precise, and calm, with professional-grade detail available through progressive disclosure.

## Design personality

Keywords:

```text
futuristic but not trend-led
classic but not nostalgic
technical and precise
secure and local-first
high-definition and materially refined
quietly expressive
finance-grade
```

The selected direction is `Precision Vaultpunk — Obsidian Spine + Light Ledger`.

CanCan should not look like a generic fintech dashboard, a cold terminal app, an enterprise admin console, a Material Design application, or a vibe-coded AI product. The main product surface should carry a distinctive identity continuously; visual quality is not reserved only for onboarding or marketing moments.

## Theme direction

MVP uses a deliberate hybrid theme rather than a global light or dark skin.

Physical scene:

```text
A person sits at their Mac in focused ambient light, reviewing private financial evidence inside a precise local Vault.
The Vault Spine feels like pristine 2026 financial equipment with classic bank-vault heritage.
The ledger remains bright, calm, and readable for long sessions.
```

Structure:

```text
Obsidian/graphite Vault Spine for persistent navigation, source indexing, and system state
Mineral-white Light Ledger for financial data, source detail, review, and document work
Emerald for healthy/current/confirmed state
Amber for unresolved attention
Restrained cinnabar for source identity, destructive risk, or error
```

Material character comes from precise seams, restrained inset depth, typography, alignment, and optical status points. Do not ship raster textures, literal safe hardware, rust, distressed metal, exposed screws, or fake mechanical controls.

The approved exploration and rejected alternatives are archived under [`resources/design/vault-archive-2026-07/`](../../resources/design/vault-archive-2026-07/README.md).

## Anti-template guardrails

CanCan must explicitly avoid two families of inherited style:

```text
Material Design 1/2:
  floating actions, ripples, generic elevation stacks, raised cards, broad shadows,
  rounded color-block navigation, and component-library-default composition

2024-2026 vibe-coded AI UI:
  cream/beige canvases, purple-blue gradients, glass panels, glowing orbs,
  sparkle decoration, ubiquitous pills, repeated uppercase eyebrows,
  oversized rounded cards, generic card grids, and chat-first visual framing
```

Also avoid fake terminal/cyberpunk styling, neon outlines, decorative grids/scanlines, black-and-gold luxury banking, and generic editorial serif-plus-mono templates. The scoped Fraunces display role in the typography direction is a deliberate, bounded brand choice — not that template: it never pairs with mono body copy, never leaves its Ledger-side display moments, and never touches UI chrome. Familiar controls should remain familiar; originality belongs in the product's composition, material language, source identity, and state behavior.

## Typography direction

Use `Geologica` for interface hierarchy and body roles. Use `Martian Mono` only for dates, counts, identifiers, hashes, and machine-origin metadata where fixed-width alignment carries meaning. Use `Fraunces Variable` as the display/editorial serif, scoped to Ledger-side display moments only: page titles, the Vault gate, and empty states.

Rules:

- self-host pinned font files when implementation begins; do not introduce a runtime Google Fonts dependency;
- keep UI labels, buttons, and financial body copy in Geologica;
- Fraunces never appears in the Vault Spine, UI chrome, labels, buttons, tables, or financial body copy; it stays at or below `text.2xl` (1.5rem), at weights 430–560, WONK 0, optical size auto;
- never turn the whole product into a monospace terminal;
- use tabular numerals for financial values even outside the mono role;
- use a fixed product type scale and no giant marketing typography inside the app;
- avoid repeated small uppercase eyebrows as default hierarchy.

## Component strategy

The accepted component foundation is shadcn/Radix primitives + Tailwind v4 (decided 2026-08-08): styles-as-source with maximum brand control, and every dependency pinned. Hero UI was considered and rejected.

Radix supplies behavior and accessibility; CanCan tokens (`0011-visual-design-tokens.md`) supply identity. Library defaults such as shadcn zinc/blue must never define the shipped brand. Tailwind utilities resolve through the `@theme` token mapping only — arbitrary-value syntax and `dark:` variants are banned, because the app is a per-region hybrid theme rather than a light/dark toggle. Common components are wrapped once in `packages/ui`; avoid adding wrapper, charting, table, or motion libraries before a concrete screen needs them.

## Layout

Use persistent left sidebar + right main body.

Desktop-first structure:

```text
Left sidebar: product navigation, source shortcuts, AI Assistant entry
Main body: page header, status, content modules, tables, review surfaces
Optional right rail: asset snapshot, insight, freshness, assistant context
```

## Density

Home should be multi-dimensional but not overwhelming.

Controls are compact — 28–32px heights — and paired with deliberate whitespace: dense where data asks for it, open where the eye rests. Whitespace is part of the composition, not leftover space; do not fill it with decoration, and do not pad controls into friendliness.

Command Center should include:

- money source activity/flows;
- snapshot freshness;
- overview metrics;
- AI insight module;
- the unified Tasks projection for review/evidence/setup work;
- but not a wall of dense tables on first load.

## Geometry

No large radius. Controls stay at or below `radius.sm` (6px), content panels at or below `radius.md` (8px); larger geometry belongs to the window chassis only. `radius.pill` is reserved for a true pill/tag control.

## Command Center priority

The main subject should be money-source activity and overview, not only review queue.

Recommended zones:

```text
Top: Money Overview / source timestamps / vault status / last scan
Upper: one Tasks section with Needs action, In progress, and Recently completed rows
Main: money source streams and recent activity
Side: AI insight and backup health
Lower: source-backed supporting detail without duplicate task/review/job cards
```

Review remains important, but its Command Center entry is a deep-linked Tasks row rather than a duplicate review card. The Tasks surface follows `0006-command-center-ui.md`; technical jobs are not a visual category.

## Source-specific presentation

Different money source types should have different stats and display modes.

Examples:

```text
Bank account: balance, inflow/outflow, statement freshness, unmatched transfers
Credit card: current liability, due/payment status, statement period, spending trend
Wise/wallet: balances by currency, source-backed FX conversions, top-ups
Brokerage: cash, positions, source-provided valuation/P&L if present
Future crypto source: token balances, source-provided valuation if present, deposits/withdrawals
Insurance: policy value, premiums, valuation date, confidence/freshness
```

## Motion and interaction

Motion is a first-class system, on the same footing as palette and typography: it is tokenized in `packages/ui` and every component consumes the shared duration/easing tokens rather than ad-hoc values. Motion should feel like precision equipment responding. It must communicate state, relationship, and feedback rather than decorate the page.

Primary rhythm:

```text
120 ms  direct feedback
180 ms  state change
240 ms  page/structural transition
easing  ease.mech: cubic-bezier(0.19, 1, 0.22, 1)
```

`ease.mech` is the single signature curve shared with the public site (per `0011-visual-design-tokens.md`); do not introduce a second easing curve.

Signature opportunities:

- onboarding step transitions;
- vault unlock/create progress;
- Vault Spine lock/unlock and source-selection state;
- tab indicators that travel along a stable track;
- source sync progress;
- evidence import that inserts/reflows rows without layout jumps;
- review item confirm/reject transitions;
- chart/value updates.

Use transforms, opacity, bounded masks/clip paths, color, and FLIP-style reflow where they improve meaning. Avoid casual animation of layout-driving properties. Feedback must never block task completion.

Every motion needs a reduced-motion fallback. Avoid bounce/elastic easing, uniform page-load choreography, decorative loops, and animations that do not map to product state.

## Accessibility and quality

- Body text contrast >= 4.5:1.
- Large text contrast >= 3:1.
- Tables must remain readable.
- No text overflow in cards, chips, buttons, or table cells.
- Empty/loading/error states must be designed, not left as raw text.
- UI work must be visually inspected by browser/computer-use when available.

## Acceptance criteria

- UI looks like a custom finance product, not default component-library output.
- The Obsidian Vault Spine and Light Ledger form one coherent application rather than two unrelated themes.
- Main surfaces retain CanCan's identity without sacrificing long-session readability.
- No Material Design 1/2 or generic vibe-coded AI visual language remains.
- Command Center balances overview, the unified Tasks projection, source activity, and AI insight without duplicate Review or technical-job cards.
- Each money source type can present tailored stats.
- Motion follows the accepted precision rhythm, is visually inspected, and has a reduced-motion fallback.
