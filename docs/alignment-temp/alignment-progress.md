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
| Initial providers | partial | DBS/UOB bank/card + Wise PDF/CSV/export; sample policy unresolved | `docs/00-product-vision.md`, `docs/11-open-questions.md` |
| First-run sequence | unresolved | Needs vault/onboarding/startup flow | TBD spec |
| Visual design system | unresolved | Left sidebar + main body; detailed palette/tokens not chosen | `docs/07-ui-information-architecture.md`, future design spec |
| Command Center layout | partial | Operational queue + asset snapshot + AI entry | `docs/specs/0006-command-center-ui.md` |
| Money source/account model | partial | User-created sources/sub-accounts; mapping suggestions | `docs/02-domain-model.md` |
| Gmail automation | partial | Guided builder + expert query; API/security details need more | `docs/specs/0003-gmail-collector.md` |
| Manual import/library | partial | Test harness/fallback; detail page not fully specified | future spec |
| Extraction/OCR/AI parser | partial | Native text + OCR + AI normalization; schemas need exact detail | `docs/specs/0004-parser-contract.md` |
| AI SDK | aligned | Vercel AI SDK allowed behind CanCan adapters | `docs/09-technology-decisions.md` |
| SQL/ORM | aligned | Hand-written SQL; no Prisma; indexes/benchmarks required | `docs/specs/0002-database-schema.md` |
| Ledger events/legs | partial | Canonical model accepted; trades/gains need deeper detail | `docs/02-domain-model.md` |
| Reconciliation policy | partial | Review-first for links; auto policies need thresholds | `docs/specs/0005-review-and-commit-policy.md` |
| Review interaction | partial | Simple inbox + expandable side-by-side details | `docs/specs/0006-command-center-ui.md` |
| AI Assistant | partial | Backend APIs/skills only; exact tools need spec | `docs/01-system-architecture.md`, `docs/specs/0006-command-center-ui.md` |
| Security/privacy | partial | Encrypted vault, read-only connectors, opt-in AI | `docs/06-local-storage-security-backup.md` |
| Backup/restore | unresolved | Needs exact target, bundle, restore flow | future spec |
| Jobs/errors | unresolved | Needs job engine spec | future spec |
| Testing/fixtures | unresolved | User deferred fixture policy; integration/reset required | future spec |
| Build/package/release | unresolved | Needs app packaging spec | future spec |
| Autonomous AI coding workflow | partial | Existing protocol; needs build/package/UI inspection details as code emerges | `docs/agent/iteration-protocol.md` |

## Next Discussion Priority

1. First-run/startup sequence.
2. Visual design system and Command Center UX.
3. Database/ledger details for trades, positions, valuations, gains.
4. Backup/restore model.
5. Testing/fixtures and AI agent automation gates.
