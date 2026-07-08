# 0011. Visual Design Tokens Spec

## Goal

Define semantic visual tokens for CanCan before UI implementation.

These are direction tokens, not final CSS values. Final values should be tuned visually and checked for contrast during implementation.

## Palette strategy

CanCan uses Warm Off-White with a small amount of warmth, graphite text, green semantic accents, and amber review states.

Avoid:

```text
retro parchment
heavy beige
purple/blue fintech gradient
cold terminal-only look
Claude-like warm document template
```

## Base tokens

Suggested semantic tokens:

```text
color.bg.app             Warm Off-White
color.bg.surface         Soft white
color.bg.subtle          Warm gray-tinted surface
color.text.primary       Deep graphite
color.text.secondary     Muted graphite
color.text.tertiary      Soft graphite
color.border.subtle      Warm gray border
color.border.strong      Graphite-tinted border
```

## Accent/status tokens

```text
color.accent.green       Healthy / synced / confirmed / freshness
color.accent.greenSoft   Background for healthy states
color.accent.amber       Review / pending / needs attention
color.accent.amberSoft   Background for review states
color.accent.red         Error / risk / failed validation
color.accent.redSoft     Background for errors
color.accent.blue        Informational only, use sparingly
```

## Semantic usage

Green means:

```text
fresh data
synced source
confirmed review item
healthy vault/backup
positive money movement when contextually appropriate
```

Green should not be blindly used for every primary button.

Amber means:

```text
pending review
needs user attention
not wrong, not dangerous, just unresolved
```

Red means:

```text
failed job
security issue
validation failure
backup failure
potential data loss
```

## Typography

Direction:

```text
UI font: modern sans-serif with excellent table readability
Numbers: tabular numerals enabled
Headings: calm, medium-weight, not oversized
Body: readable at dense dashboard sizes
```

Requirements:

- Use tabular numbers for balances and transaction tables.
- Avoid giant marketing-style hero typography inside the app.
- Keep line length reasonable in explanatory panels.

## Spacing and radius

Direction:

```text
spacing.compact for tables and dense lists
spacing.comfortable for onboarding and settings
radius.small for inputs/tables
radius.medium for panels
avoid huge rounded cards
```

Suggested radius scale:

```text
radius.xs 4px
radius.sm 6px
radius.md 10px
radius.lg 14px
```

Cards/panels should usually stay <= 14px radius.

## Elevation

Use borders and subtle tonal separation before heavy shadows.

Avoid broad decorative shadows. Finance data should feel stable and grounded.

## Motion tokens

```text
motion.fast       120-160ms
motion.normal     180-240ms
motion.slow       280-360ms only for onboarding/page transitions
ease.standard     ease-out cubic/quart
ease.emphasized   for important state transitions
```

Reduced motion must be supported.

## Chart tokens

Charts should be light, readable, and semantic.

```text
chart.cash         green family
chart.liability    amber/red-brown family, not alarm red unless overdue/risk
chart.investment   graphite/green mix
chart.crypto       distinct but restrained accent
chart.insurance    warm neutral/accent
```

Avoid rainbow chart palettes in MVP.

## Component library mapping

If using Hero UI:

- Map Hero UI theme tokens to CanCan semantic tokens.
- Wrap common components in `packages/ui`.
- Ensure default Hero UI styling is overridden enough to feel like CanCan.

## Acceptance criteria

- Tokens can be implemented in a single theme file.
- Semantic color roles are clear.
- Contrast checks pass.
- Tables, panels, badges, charts, and review states use consistent tokens.
- UI does not resemble a generic component-library demo.
