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

## Slice-2 app shell evidence

The app shell migrated to the token foundation: `AppShell` flex chassis, obsidian Vault Spine (240px, mono metadata labels, square-cap icons, amber count pill, go-bright active dot), `LedgerHeader` (mono eyebrow + scoped Fraunces page title + quiet actions over a hairline), the 60rem left-anchored `LedgerColumn` measure with deliberate right whitespace, and the Vault gate as a porcelain panel with status-point eyebrow, Fraunces title, and the go-fill Touch ID moment. Below 760px the spine becomes a top bar whose nav stays available as a horizontal scroll strip (the legacy shell lost nav entirely there). Content below the migrated headers stays legacy until slices 3–5.

| Artifact | Width | State |
| --- | --- | --- |
| [`slice2-overview-1440.png`](slice2-overview-1440.png) | 1440 | Overview with new shell |
| [`slice2-overview-attention-1440.png`](slice2-overview-attention-1440.png) | 1440 | Overview with interim Needs-attention cards |
| [`slice2-overview-1180.png`](slice2-overview-1180.png) / [`slice2-overview-760.png`](slice2-overview-760.png) | 1180/760 | Narrow shell + horizontal nav strip |
| [`slice2-overview-reduced-motion.png`](slice2-overview-reduced-motion.png) | 1440 | `prefers-reduced-motion` render |
| [`slice2-review-1440.png`](slice2-review-1440.png) | 1440 | Review queue, active nav count + dot |
| [`slice2-sources-inbox-enabled-1440.png`](slice2-sources-inbox-enabled-1440.png) | 1440 | Sources, Inbox enabled |
| [`slice2-vault-gate-locked-1440.png`](slice2-vault-gate-locked-1440.png) / [`slice2-vault-gate-locked-760.png`](slice2-vault-gate-locked-760.png) | 1440/760 | Vault gate, Touch ID path |
| [`slice2-vault-gate-create-1440.png`](slice2-vault-gate-create-1440.png) | 1440 | Vault gate, create path |

## Slice-3 Command Center evidence

The unified Tasks surface (spec 0006): one Tasks section on Overview (host-ordered rows, group captions, consequence labels, needs-action count, `View all`), the full Tasks route with the four group filters, the Tasks nav entry with the needs-action pill, and the focused source-confirmation dialog as the `source_confirmation` deep-link target. The interim Needs-attention / Review-status / Jobs cards are gone; account-confirmation cards moved to their owning Sources attention panel (frozen-contract boundary). These captures also caught and now evidence the fix for unstyled bare `<button>` elements under a dark-UA `color-scheme` (rows/chips now set explicit `bg-transparent`).

| Artifact | Width | State |
| --- | --- | --- |
| [`slice3-overview.png`](slice3-overview.png) | 1440 | Overview: Tasks section + Money Overview + Recent activity |
| [`slice3-overview-empty.png`](slice3-overview-empty.png) | 1440 | Caught-up + first-run empty states |
| [`slice3-tasks.png`](slice3-tasks.png) | 1440 | Full Tasks route, Needs action filter |
| [`slice3-tasks-parked.png`](slice3-tasks-parked.png) | 1440 | Full Tasks route, Parked filter |
| [`slice3-source-confirm.png`](slice3-source-confirm.png) | 1440 | Focused source-confirmation dialog over inert ledger |
| [`slice3-overview-760.png`](slice3-overview-760.png) | 760 | Narrow: top-bar spine, rows truncate without overlap |
| [`slice3-tasks-reduced-motion.png`](slice3-tasks-reduced-motion.png) | 1440 | `prefers-reduced-motion` render |

## Slice-4 Sources / Evidence evidence

The Sources intake surface on the token foundation: LedgerHeader actions (Touch ID opt-in, Lock Vault, Add file), the recovery to-do Panel, the CanCan Inbox panel (SectionHeader + token controls), Money Sources with MonogramTile rows and month-grouped evidence lists (kind chip, mono metadata, StatusPoint + the 0017 five-state label, quiet/danger sm actions), the Needs-attention zone with token source/account confirmation cards, and the four Radix document modals — the 3xl-wide rendered page viewer, the bounded CSV preview, the protected-statement unlock form (token Select + Input), and the spec-0017 destructive `Delete source file` confirmation.

| Artifact | Width | State |
| --- | --- | --- |
| [`slice4-sources.png`](slice4-sources.png) | 1440 | Sources: recovery to-do, Inbox on, Money Sources + evidence rows |
| [`slice4-document-viewer.png`](slice4-document-viewer.png) | 1440 | Rendered page viewer (3xl dialog, pager footer) |
| [`slice4-document-preview.png`](slice4-document-preview.png) | 1440 | Bounded CSV preview with truncation note |
| [`slice4-document-unlock.png`](slice4-document-unlock.png) | 1440 | Protected-statement unlock: token Select + password Input |
| [`slice4-delete-confirm.png`](slice4-delete-confirm.png) | 1440 | `Delete source file` destructive confirmation (0017 consequence list) |
| [`slice4-source-confirm.png`](slice4-source-confirm.png) | 1440 | Focused source-confirmation dialog (regression check) |
| [`slice4-sources-760.png`](slice4-sources-760.png) | 760 | Narrow: top-bar spine, sections stack without overlap |
| [`slice4-delete-confirm-reduced-motion.png`](slice4-delete-confirm-reduced-motion.png) | 1440 | `prefers-reduced-motion` render |

## Implementation boundary

Do not ship these PNGs in the desktop bundle. Baselines document what the slices replace; the spike image documents toolchain feasibility only. Implementation must use semantic HTML/React, accessible states, centralized tokens, and code-native motion.
