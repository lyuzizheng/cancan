# 0008. Design System Spec

## Goal

Define CanCan's visual design direction before UI implementation.

CanCan should feel like a 2026 asset-management and finance operations app: modern, warm, technical, secure, polished, and calm.

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

CanCan should not look like a generic blue fintech dashboard or a cold terminal app.

## Theme direction

MVP is light-first.

Preferred palette direction:

```text
warm off-white / soft warm surface
charcoal / graphite text
green accent for confirmation, growth, freshness, and successful sync
subtle amber/warm accent for pending/review states
red only for real risk/errors
```

The user prefers warm + green. The final palette should still pass contrast checks and avoid beige-heavy AI-default styling.

## Component/style references

Potential implementation references:

- Hero UI style quality;
- DaisyUI can be used as inspiration or utility, but should not dictate final visual identity;
- CanCan should have custom product-level polish, not default component-library appearance.

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
Top: net worth / freshness / vault status / last scan
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
Wise/wallet: balances by currency, FX conversions, top-ups, stale currency rates
Brokerage: cash, positions, valuation, estimated P/L, stale price warnings
Crypto: token balances, valuation, deposits/withdrawals, high volatility warning
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
- Warm + green direction is visible but not muddy or beige-heavy.
- Command Center balances overview, source activity, AI insight, and review status.
- Each money source type can present tailored stats.
- Motion is meaningful and has reduced-motion fallback.
