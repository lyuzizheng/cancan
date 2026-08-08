# 0011. Visual Design Tokens Spec

## Goal

Define the accepted semantic visual tokens for CanCan's `Precision Vaultpunk — Obsidian Spine + Light Ledger` direction.

These are the implementation values. The 2026-08-08 version-2 amendment adds the editorial accent family, the scoped Fraunces display role, the retirement of `radius.lg` for content, and the unified `ease.mech` curve; every other version-1 value stands. Visual inspection may tune a value only when contrast, platform rendering, or state clarity supplies concrete evidence; any role change belongs back in this spec.

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

color.accent.goBright     oklch(77.7% 0.177 153.4)  #3ED67F
color.accent.go           oklch(68.2% 0.165 153.3)  #22B566
color.accent.goDeep       oklch(46.8% 0.105 155.7)  #166B40
color.accent.goInk        oklch(21.8% 0.042 159.4)  #052013
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

accent.goBright on ledger.mineral       1.71:1
accent.go on ledger.mineral             2.42:1
accent.goDeep on ledger.mineral         5.92:1
accent.goBright on vault.obsidian      10.17:1
accent.go on vault.obsidian             7.19:1
accent.goDeep on vault.obsidian         2.93:1
accent.goInk on accent.goBright         9.10:1
accent.goInk on accent.go               6.44:1
```

`signal.amber` is an indicator/fill color, not small text. Use `signal.amberText` for labels. Status must never rely on color alone.

The `accent.go*` family is the editorial accent borrowed from the public site's brighter green. `accent.goBright` and `accent.go` measure below 3:1 on `ledger.mineral`, so they are Vault-Spine/dark-surface accents and large fills only — never Ledger text or hairline indicators. `accent.goInk` is their verified on-fill text pair. Ledger text and labels keep the `signal.emerald`/`amberText`/`dangerText` pairs; `accent.goDeep` is the verified Ledger editorial-text accent. On the Vault Spine, prefer `accent.goBright`/`accent.go` — `accent.goDeep` falls below 3:1 on `vault.obsidian`.

## Accent/status tokens

```text
color.signal.emerald     Healthy / synced / confirmed / freshness
color.signal.amber       Review / pending / needs action indicator
color.signal.amberText   Accessible attention text on the Light Ledger
color.signal.cinnabar    Restrained source identity or high-salience risk marker
color.signal.dangerText  Accessible error/risk text on the Light Ledger

color.accent.goBright    Editorial accent on dark surfaces and large fills
color.accent.go          Editorial accent on dark surfaces and large fills
color.accent.goDeep      Editorial text accent on the Light Ledger
color.accent.goInk       Text on goBright/go fills
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

Version-1 roles, with the version-2 display addition:

```text
UI hierarchy/body: Geologica variable sans
Display/editorial: Fraunces Variable, scoped per 0008 — Ledger-side display moments
  (page titles, vault gate, empty states) only; WONK 0, optical size auto,
  weights 430–560, capped at text.2xl 1.50rem
Machine metadata: Martian Mono
Numbers: tabular numerals enabled
Weights: regular, medium, semibold; bold only when a financial hierarchy requires it
```

Version-1 type scale, exposed as CSS custom properties in `packages/ui`:

```text
text.xs    0.72rem  mono metadata and dense labels
text.sm    0.80rem  secondary labels and compact controls
text.base  0.88rem  default UI body and navigation
text.md    0.92rem  emphasized row and list titles
text.lg    1.10rem  section headings
text.xl    1.30rem  panel and gate headings
text.2xl   1.50rem  page title, the largest in-app size
```

Requirements:

- Use tabular numbers for balances and transaction tables.
- Avoid giant marketing-style hero typography inside the app.
- Keep line length reasonable in explanatory panels.

## Spacing and radius

Version-2 geometry:

```text
spacing.compact for tables and dense lists
spacing.comfortable for onboarding and settings
radius.xs 3px
radius.sm 6px
radius.md 8px
radius.pill only for a true pill/tag control
```

`radius.lg` is retired for content. Controls stay at or below `radius.sm` (6px) and content panels at or below `radius.md` (8px); larger geometry is chassis-only, reserved for the Vault Spine window/application boundary. Prefer open ledgers, rules, and alignment over card containment.

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
ease.mech          cubic-bezier(0.19, 1, 0.22, 1)
```

`ease.mech` is the one signature curve for the whole product, replacing the version-1 `ease.precision`; do not introduce a second easing curve. Exit transitions should normally use roughly 75% of the entrance duration. Reduced motion must replace movement with instant state or a short crossfade. Do not use bounce, elastic, ornamental page-load staggering, or perpetual animation; an active processing status may use a bounded low-frequency optical pulse.

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

The accepted component foundation is shadcn/Radix primitives + Tailwind v4, wrapped as CanCan-branded components in `packages/ui`:

- Map every Tailwind utility through the `@theme` token layer. Arbitrary-value syntax (`w-[317px]`, `text-[13px]`, `bg-[#…]`) is banned outside `tokens.css`.
- Radix supplies behavior and accessibility only; visual identity comes from CanCan tokens. Default shadcn zinc/blue styling must never ship.
- No `dark:` variant: the app is a per-region hybrid theme (Spine/Ledger token groups), not a light/dark mode toggle.

## Acceptance criteria

- Tokens can be implemented through one primitive palette and one semantic theme layer.
- Semantic color roles are clear.
- The documented contrast checks pass and platform rendering is rechecked during implementation.
- Tables, panels, badges, charts, and review states use consistent tokens.
- UI preserves the selected Obsidian Spine + Light Ledger composition and does not resemble Material Design, a generic component-library demo, or a vibe-coded AI application.
