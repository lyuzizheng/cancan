# 0008. Design System Spec

## Goal

Define CanCan's visual design direction before UI implementation.

## Implementation blocker

Exact tokens, component major/version, and Figma's role remain unresolved in the [active alignment register](../alignment-temp/alignment-progress.md). Directional styling may guide discussion but must not be treated as a final token contract.

CanCan should feel like a 2026 personal finance and account-record workspace: modern, warm, technical, secure, polished, and calm, with professional-grade detail available through progressive disclosure.

## Design personality

Keywords:

```text
modern
technical
secure
warm
precise
premium
local-first
AI-assisted
finance-grade
```

CanCan should not look like a generic blue fintech dashboard, a cold terminal app, or a Claude-style retro parchment template.

## Theme direction

MVP is light-first.

Preferred palette direction:

```text
Warm Off-White base with a small amount of warmth
charcoal / graphite text
green semantic accent for freshness, healthy sync, confirmation, and money-positive states
amber/warm accent for pending/review states
red only for real risk/errors
```

The final palette should pass contrast checks and avoid beige-heavy AI-default styling.

## Component strategy

Use a mature React component foundation to keep code volume under control and ensure consistency.

Recommended default:

```text
Hero UI for app components
Tailwind-compatible theme tokens
CanCan-owned wrappers in packages/ui
lucide-react icons
TanStack Table for complex tables
Recharts or equivalent for simple charts
motion/react or CSS transitions for purposeful motion
```

Alternatives such as shadcn/Radix are acceptable if chosen deliberately. Do not mix multiple component systems without a written reason.

DaisyUI can inspire fast prototypes, but should not define final brand identity.

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

Command Center should include:

- money source activity/flows;
- snapshot freshness;
- overview metrics;
- AI insight module;
- review/evidence status;
- but not a wall of dense tables on first load.

## Command Center priority

The main subject should be money-source activity and overview, not only review queue.

Recommended zones:

```text
Top: Money Overview / source timestamps / vault status / last scan
Main: money source streams and recent activity
Side: AI insight, review count, backup health
Lower section: Needs Review, New Evidence, Failed Jobs, Suggested Links
```

Review remains important, but it does not need to dominate the first viewport.

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

Motion should communicate state:

- onboarding step transitions;
- vault unlock/create progress;
- source sync progress;
- evidence imported state;
- review item confirm/reject transitions;
- chart/value updates.

Every motion needs reduced-motion fallback.

Avoid decorative animations that do not map to product state.

## Accessibility and quality

- Body text contrast >= 4.5:1.
- Large text contrast >= 3:1.
- Tables must remain readable.
- No text overflow in cards, chips, buttons, or table cells.
- Empty/loading/error states must be designed, not left as raw text.
- UI work must be visually inspected by browser/computer-use when available.

## Acceptance criteria

- UI looks like a custom finance product, not default component-library output.
- Warm Off-White + green direction is visible but not muddy or beige-heavy.
- Command Center balances overview, source activity, AI insight, and review status.
- Each money source type can present tailored stats.
- Motion is meaningful and has reduced-motion fallback.
