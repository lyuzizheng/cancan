# UI Modernization 2026-08 — Baseline and Toolchain Evidence

## Purpose

This archive records the pre-modernization state of the desktop renderer at the start of the 2026-08 UI modernization program, plus the slice-0 compatibility-spike evidence. Later slices compare their replacements against these baselines; do not treat the old layout as a reference to preserve.

Canonical visual behavior remains in [`docs/specs/0008-design-system.md`](../../../docs/specs/0008-design-system.md) and [`docs/specs/0011-visual-design-tokens.md`](../../../docs/specs/0011-visual-design-tokens.md) (version-2 amended 2026-08-08). These images are design/toolchain evidence, not implementation specifications or runtime assets.

## Baseline — hand-rolled CSS renderer before slice 1

Captured from the dev-only `preview.html?state=…` fixture harness at the recorded widths. The Sources states render the Inbox panel only (the preview fixture never wrapped it in a Sources page header) — a pre-existing fixture simplification, replaced when the Sources view lands.

| Artifact | Width | State |
| --- | --- | --- |
| [`baseline-overview-1440.png`](baseline-overview-1440.png) | 1440 | Overview |
| [`baseline-overview-attention-1440.png`](baseline-overview-attention-1440.png) | 1440 | Overview with interim Needs-attention cards |
| [`baseline-overview-attention-1180.png`](baseline-overview-attention-1180.png) | 1180 | Same, narrow |
| [`baseline-overview-attention-760.png`](baseline-overview-attention-760.png) | 760 | Same, narrowest |
| [`baseline-overview-empty-1440.png`](baseline-overview-empty-1440.png) | 1440 | Overview empty |
| [`baseline-review-1440.png`](baseline-review-1440.png) / [`-1180`](baseline-review-1180.png) / [`-760`](baseline-review-760.png) | 1440/1180/760 | Review queue |
| [`baseline-review-detail-1440.png`](baseline-review-detail-1440.png) | 1440 | Review expanded detail |
| [`baseline-review-job-1440.png`](baseline-review-job-1440.png) | 1440 | Review batch job panel |
| [`baseline-sources-inbox-enabled-1440.png`](baseline-sources-inbox-enabled-1440.png) | 1440 | Sources, Inbox enabled |
| [`baseline-sources-inbox-reauth-1440.png`](baseline-sources-inbox-reauth-1440.png) | 1440 | Sources, Inbox re-authorization |

## Slice-0 compatibility spike evidence

[`spike-radix-tailwind-oklch.png`](spike-radix-tailwind-oklch.png) — one Radix `Dialog` themed purely by the 0011 OKLCH tokens through the Tailwind v4 `@theme` mapping (Vault-obsidian overlay, porcelain panel at 8px radius, go-bright fill with go-ink text, go-deep editorial text). Verified on Vite 8.1.4 + React 19.2.7 with `tailwindcss@4.3.3`, `@tailwindcss/vite@4.3.3`, and `@radix-ui/react-dialog@1.1.23`: production build passes, dev HMR serves hot-updated modules, and token-driven utilities compile. The spike code was disposable and reverted; slice 1 re-pins these exact versions when building the real foundation.

## Slice-1 foundation gallery evidence

Captured from the dev-only `?state=primitives` / `?state=primitives-dialog` gallery, which exercises every shipped primitive and pattern on one Ledger surface plus a Vault-dark strip for the accent usage rule.

| Artifact | What it evidences |
| --- | --- |
| [`slice1-primitives-1440.png`](slice1-primitives-1440.png) | Full gallery at desktop width: Fraunces display moment (≤1.5rem, 560), button variants (primary/strong/quiet/text/danger at 32/28px), inputs/select/tooltip, badges/status points/monogram tiles, SectionHeader + MetricRow + ActionBar rhythm, EmptyState, Skeleton pulse, Vault-dark accent pairs |
| [`slice1-primitives-1180.png`](slice1-primitives-1180.png) / [`slice1-primitives-760.png`](slice1-primitives-760.png) | Narrow compositions wrap without overflow |
| [`slice1-primitives-dialog-1440.png`](slice1-primitives-dialog-1440.png) | Radix Dialog over the obsidian scrim: radius.md porcelain panel, ease-mech entrance, focus outline in `accent.goDeep` |
| [`slice1-primitives-reduced-motion.png`](slice1-primitives-reduced-motion.png) | `prefers-reduced-motion` render — animations degrade per the motion contract |

Product views were not changed in slice 1; the legacy shell was verified pixel-identical against the baseline captures above (0 changed pixels at 1440 for overview, overview-attention, and review-detail).

## Implementation boundary

Do not ship these PNGs in the desktop bundle. Baselines document what the slices replace; the spike image documents toolchain feasibility only. Implementation must use semantic HTML/React, accessible states, centralized tokens, and code-native motion.
