# Vault Archive Design Exploration

## Purpose

This archive records the July 2026 visual exploration for CanCan's Source/Documents surface. It preserves the rejected directions as well as the selected north star so later implementation does not flatten the design back into a generic dashboard.

Canonical visual behavior remains in [`docs/specs/0008-design-system.md`](../../../docs/specs/0008-design-system.md) and [`docs/specs/0011-visual-design-tokens.md`](../../../docs/specs/0011-visual-design-tokens.md). These images are design evidence, not implementation specifications or runtime assets.

## Decision trail

| Artifact | Result | What it taught us |
| --- | --- | --- |
| [`00-baseline-desktop.png`](00-baseline-desktop.png) and [`00-baseline-narrow.png`](00-baseline-narrow.png) | Baseline | The functional information architecture was clear, but the warm-neutral card treatment felt like a 2020-era SaaS dashboard |
| [`01-vault-archive-light-ledger.png`](01-vault-archive-light-ledger.png) | Partial reference | The open light ledger was legible and useful; the composition felt too formal and institutional |
| [`02-industrial-dark-vault.png`](02-industrial-dark-vault.png) | Rejected as rendered | The physical Vault Spine was memorable, but rust, exposed hardware, and military-industrial texture felt heavy rather than elegant |
| [`03-modernist-bank-archive.png`](03-modernist-bank-archive.png) | Rejected | The light modernist direction was clean but too plain and close to generic fintech |
| [`04-precision-vaultpunk-obsidian.png`](04-precision-vaultpunk-obsidian.png) | **Selected north star** | A pristine dark Vault Spine paired with an open light ledger creates CanCan's own futuristic/classic identity |
| [`05-precision-vaultpunk-porcelain.png`](05-precision-vaultpunk-porcelain.png) | Material comparison | The white precision material was elegant, but it lost the strong Vault boundary the product needs |
| [`06-approved-palette.png`](06-approved-palette.png) | **Selected palette reference** | Obsidian, mineral white, emerald, amber, and restrained cinnabar provide the accepted semantic palette |

The palette board still prints the exploratory easing curve `(0.23, 1, 0.32, 1)`. That value is superseded. The canonical motion token was `cubic-bezier(0.22, 1, 0.36, 1)` at version 1 and is `ease.mech cubic-bezier(0.19, 1, 0.22, 1)` since the 2026-08-08 version-2 amendment in the design specs; archived image annotations never override canonical text.

## Selected direction

`Precision Vaultpunk — Obsidian Spine + Light Ledger`

- The persistent navigation and system state live in a pristine obsidian/graphite Vault Spine.
- The main workspace is a high-legibility mineral-white ledger rather than a card dashboard.
- Material character comes from precise seams, restrained inset depth, typography, and optical status points—not raster texture, rust, fake controls, or decoration.
- Familiar product interactions remain familiar. Distinctiveness comes from composition, material language, source identity, and purposeful motion.
- Motion should feel like precision equipment responding: fast feedback, clear state transitions, no bounce, no ornamental loops, and a reduced-motion alternative.

## Explicit anti-references

Do not regress toward:

- Material Design 1/2 elevation, ripples, floating actions, or raised card stacks;
- 2024–2026 vibe-coded AI styling such as cream/beige canvases, purple-blue gradients, glass panels, glowing orbs, sparkle decoration, ubiquitous pills, repeated uppercase eyebrows, and generic card grids;
- fake terminal/cyberpunk interfaces, neon outlines, decorative grids, or scanlines;
- steampunk, rust, distressed metal, exposed screws, safe dials, or non-functional mechanical controls;
- black-and-gold luxury banking or editorial serif-plus-mono templates.

## Implementation boundary

The selected mock is a composition and material north star. Do not ship these PNGs in the desktop bundle, rasterize UI text, or trace non-functional hardware literally. Implementation must use semantic HTML/React, accessible states, centralized tokens, and code-native motion.
