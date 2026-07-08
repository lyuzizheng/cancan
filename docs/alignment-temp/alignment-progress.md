# Alignment Progress

Status values:

```text
aligned       decision is stable and reflected in permanent docs/specs
partial       direction exists but needs more detail
unresolved    needs user discussion
blocked       cannot decide until external info/sample/code exists
```

## Progress Table

| Area | Status | Current decision | Permanent home |
| --- | --- | --- | --- |
| Product identity | aligned | Local-first financial evidence vault + reconciliation console | `docs/00-product-vision.md` |
| MVP success | aligned | Gmail automation required; manual import is test harness | `docs/00-product-vision.md`, `docs/10-roadmap.md` |
| Initial providers | partial | Fixed supported provider list; sample policy unresolved | `docs/00-product-vision.md`, `docs/specs/0007-first-run-onboarding.md` |
| First-run sequence | partial | Welcome -> vault -> currency -> AI -> source -> Gmail/import -> Command Center | `docs/specs/0007-first-run-onboarding.md` |
| Visual design system | partial | Warm Off-White + small warmth + green semantic; exact values unresolved | `docs/specs/0008-design-system.md`, `docs/specs/0011-visual-design-tokens.md` |
| Component strategy | partial | Prefer Hero UI-style app foundation with CanCan wrappers; alternatives possible if documented | `docs/specs/0006-command-center-ui.md`, `docs/specs/0008-design-system.md` |
| Command Center layout | partial | Multi-dimensional overview; source activity main, review lower/compact; markdown wireframe added | `docs/specs/0006-command-center-ui.md` |
| Money source/account model | partial | User-created sources/sub-accounts; fixed providers; mapping suggestions | `docs/02-domain-model.md` |
| Gmail automation | aligned | Desktop OAuth + PKCE + loopback, Gmail readonly, local token/cache, polling sync | `docs/specs/0003-gmail-collector.md`, `docs/specs/0007-first-run-onboarding.md` |
| Manual import/library | partial | Test harness/fallback; detail page not fully specified | future spec |
| Extraction/OCR/AI parser | partial | Native text + OCR + AI normalization; schemas need exact detail | `docs/specs/0004-parser-contract.md` |
| AI SDK | aligned | Vercel AI SDK allowed behind CanCan adapters | `docs/09-technology-decisions.md` |
| SQL/ORM | aligned | Hand-written SQL; no Prisma; indexes/benchmarks required | `docs/specs/0002-database-schema.md` |
| Ledger/assets/valuation | partial | Fact-based; no market price fetch; multi-currency; source snapshots first; no tax; trade table deferred | `docs/02-domain-model.md`, `docs/specs/0013-ledger-assets-valuation.md` |
| Reconciliation policy | partial | Review-first for links; auto policies need thresholds | `docs/specs/0005-review-and-commit-policy.md` |
| Review interaction | partial | Simple inbox + expandable side-by-side details | `docs/specs/0006-command-center-ui.md` |
| AI Assistant | partial | Backend APIs/skills only; exact tools need spec | `docs/01-system-architecture.md`, `docs/specs/0006-command-center-ui.md` |
| Security/privacy | partial | Encrypted vault, read-only connectors, opt-in AI; Google restricted scope review risk recorded | `docs/06-local-storage-security-backup.md`, `docs/specs/0003-gmail-collector.md` |
| Backup/restore | partial | Generic folder first; manifest/schema/app version compatibility required | `docs/specs/0009-backup-restore-versioning.md` |
| Jobs/errors | unresolved | Needs job engine spec | future spec |
| Testing/fixtures | unresolved | User deferred fixture policy; integration/reset required | future spec |
| Build/package/release | partial | Agent workflow requires build/package checks; exact commands pending code | `docs/specs/0010-agentic-development-workflow.md` |
| Autonomous AI coding workflow | partial | Tests, DB reset, build, UI inspection, docs update required; `.agents/` workflows planned | `docs/specs/0010-agentic-development-workflow.md`, `docs/specs/0012-repo-agent-workflows.md` |

## Next Discussion Priority

1. Clarify base-currency/net-worth behavior under no-derived-FX principle.
2. Job engine and error model.
3. Testing/fixtures and AI agent automation gates.
4. Evidence Library detail UX.
5. Exact design token values and Figma prototype decision.
