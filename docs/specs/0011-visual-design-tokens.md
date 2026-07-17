# 0011. Visual Design Tokens Spec

## Goal

Define the accepted semantic visual tokens for CanCan's `Precision Vaultpunk — Obsidian Spine + Light Ledger` direction.

These are the version-1 implementation values. Visual inspection may tune a value only when contrast, platform rendering, or state clarity supplies concrete evidence; any role change belongs back in this spec.

## Palette strategy

CanCan uses an obsidian/graphite Vault Spine, a mineral-white Light Ledger, emerald healthy state, amber attention, and restrained cinnabar identity/risk.

Avoid:

```text
cream, beige, parchment, or warm AI-default canvases
purple/blue gradients, glass, glow, or AI sparkle decoration
Material Design elevation/card language
fake terminal, neon cyberpunk, grid, or scanline styling
rust, distressed metal, steampunk, or literal safe hardware
black-and-gold luxury finance
```

## Version-1 palette

Hex values are included for asset/tool interoperability. Product CSS should use the OKLCH value.

```text
color.vault.obsidian      oklch(16.7% 0.009 169.0)  #0B100E
color.vault.graphite      oklch(24.3% 0.013 160.3)  #1B221E
color.vault.seam          oklch(33.7% 0.017 159.0)  #303A34
color.vault.text          oklch(94.4% 0.009 145.5)  #E9EEE9
color.vault.textMuted     oklch(72.0% 0.017 154.3)  #9DA8A0

color.ledger.mineral      oklch(96.6% 0.004 121.6)  #F3F4F1
color.ledger.porcelain    oklch(99.1% 0.003 106.4)  #FCFCFA
color.ledger.ink          oklch(21.1% 0.010 159.2)  #151A17
color.ledger.textMuted    oklch(49.1% 0.014 153.4)  #5B635D
color.ledger.rule         oklch(87.7% 0.009 145.5)  #D3D8D3

color.signal.emerald      oklch(53.0% 0.100 157.8)  #2F7D55
color.signal.amber        oklch(62.8% 0.123 69.1)   #B87924
color.signal.amberText    oklch(48.3% 0.100 67.8)   #83520F
color.signal.cinnabar     oklch(53.9% 0.151 31.4)   #B54432
color.signal.dangerText   oklch(43.7% 0.126 29.9)   #8A2F24
```

Verified contrast examples:

```text
ledger.ink on ledger.mineral          15.96:1
ledger.textMuted on ledger.mineral     5.61:1
vault.text on vault.obsidian           16.33:1
vault.textMuted on vault.obsidian       7.80:1
signal.emerald on ledger.mineral        4.54:1
signal.amberText on ledger.mineral      5.98:1
signal.dangerText on ledger.mineral     7.57:1
```

`signal.amber` is an indicator/fill color, not small text. Use `signal.amberText` for labels. Status must never rely on color alone.

## Accent/status tokens

```text
color.signal.emerald     Healthy / synced / confirmed / freshness
color.signal.amber       Review / pending / needs attention indicator
color.signal.amberText   Accessible attention text on the Light Ledger
color.signal.cinnabar    Restrained source identity or high-salience risk marker
color.signal.dangerText  Accessible error/risk text on the Light Ledger
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

Version-1 roles:

```text
UI hierarchy/body: Geologica variable sans
Machine metadata: Martian Mono
Numbers: tabular numerals enabled
Weights: regular, medium, semibold; bold only when a financial hierarchy requires it
```

Requirements:

- Use tabular numbers for balances and transaction tables.
- Avoid giant marketing-style hero typography inside the app.
- Keep line length reasonable in explanatory panels.

## Spacing and radius

Version-1 geometry:

```text
spacing.compact for tables and dense lists
spacing.comfortable for onboarding and settings
radius.xs 3px
radius.sm 6px
radius.md 8px
radius.lg 10px
radius.pill only for a true pill/tag control
```

The Vault Spine may use larger outer chassis geometry only where it describes the window/application boundary. Content panels and controls stay at or below 10px. Prefer open ledgers, rules, and alignment over card containment.

## Material tokens

```text
material.vault.base       vault.obsidian
material.vault.raised     vault.graphite
material.vault.seam       vault.seam hairline
material.vault.depth      restrained inset tonal separation
material.ledger.base      ledger.mineral
material.ledger.surface   ledger.porcelain
material.ledger.rule      ledger.rule hairline
```

Do not use image textures in the production shell. Material is expressed with explicit solid tones, hairlines, bounded inset depth, and optical state points. Avoid alpha-heavy color construction and broad decorative shadows.

## Elevation

Use borders and subtle tonal separation before heavy shadows.

Avoid broad decorative shadows. Finance data should feel stable and grounded.

## Motion tokens

```text
motion.feedback    120ms
motion.state       180ms
motion.transition  240ms
motion.slow        320ms only for onboarding or Vault creation/unlock explanation
ease.precision     cubic-bezier(0.22, 1, 0.36, 1)
```

Exit transitions should normally use roughly 75% of the entrance duration. Reduced motion must replace movement with instant state or a short crossfade. Do not use bounce, elastic, ornamental page-load staggering, or perpetual animation; an active processing status may use a bounded low-frequency optical pulse.

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

- Tokens can be implemented through one primitive palette and one semantic theme layer.
- Semantic color roles are clear.
- The documented contrast checks pass and platform rendering is rechecked during implementation.
- Tables, panels, badges, charts, and review states use consistent tokens.
- UI preserves the selected Obsidian Spine + Light Ledger composition and does not resemble Material Design, a generic component-library demo, or a vibe-coded AI application.
