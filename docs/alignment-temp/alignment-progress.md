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
| Visual design system | partial | Light-first, modern warm+green, tech/safe feel; exact tokens unresolved | `docs/specs/0008-design-system.md` |
| Command Center layout | partial | Multi-dimensional overview; source activity main, review lower/compact | `docs/specs/0006-command-center-ui.md` |
| Money source/account model | partial | User-created sources/sub-accounts; fixed providers; mapping suggestions | `docs/02-domain-model.md` |
| Gmail automation | unresolved | Official OAuth/API recommended, but computer-use vs API still open | `docs/specs/0003-gmail-collector.md`, `docs/specs/0007-first-run-onboarding.md` |
| Manual import/library | partial | Test harness/fallback; detail page not fully specified | future spec |
| Extraction/OCR/AI parser | partial | Native text + OCR + AI normalization; schemas need exact detail | `docs/specs/0004-parser-contract.md` |
| AI SDK | aligned | Vercel AI SDK allowed behind CanCan adapters | `docs/09-technology-decisions.md` |
| SQL/ORM | aligned | Hand-written SQL; no Prisma; indexes/benchmarks required | `docs/specs/0002-database-schema.md` |
| Ledger events/legs | partial | Canonical model accepted; trades/gains need deeper detail; source-type stats differ | `docs/02-domain-model.md` |
| Reconciliation policy | partial | Review-first for links; auto policies need thresholds | `docs/specs/0005-review-and-commit-policy.md` |
| Review interaction | partial | Simple inbox + expandable side-by-side details | `docs/specs/0006-command-center-ui.md` |
| AI Assistant | partial | Backend APIs/skills only; exact tools need spec | `docs/01-system-architecture.md`, `docs/specs/0006-command-center-ui.md` |
| Security/privacy | partial | Encrypted vault, read-only connectors, opt-in AI | `docs/06-local-storage-security-backup.md` |
| Backup/restore | partial | Generic folder first; manifest/schema/app version compatibility required | `docs/specs/0009-backup-restore-versioning.md` |
| Jobs/errors | unresolved | Needs job engine spec | future spec |
| Testing/fixtures | unresolved | User deferred fixture policy; integration/reset required | future spec |
| Build/package/release | partial | Agent workflow requires build/package checks; exact commands pending code | `docs/specs/0010-agentic-development-workflow.md` |
| Autonomous AI coding workflow | partial | Tests, DB reset, build, UI inspection, docs update required | `docs/specs/0010-agentic-development-workflow.md` |

## Next Discussion Priority

1. Gmail integration approach: official OAuth/API vs AI computer-use.
2. Exact visual tokens and design-system choices.
3. Database/ledger details for trades, positions, valuations, gains.
4. Job engine and error model.
5. Testing/fixtures and AI agent automation gates.
