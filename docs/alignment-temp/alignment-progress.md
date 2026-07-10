# Active Alignment Register

This file contains only decisions that are still partial, unresolved, or blocked. Stable decisions belong in canonical specs or status-bearing ADRs and must not be copied here.

Status values:

```text
partial       direction exists but implementation-critical detail is missing
unresolved    user discussion is required
blocked       external evidence, feasibility work, or implementation is required first
```

## P0: blocks safe implementation

| Area | Status | Decision still required | Canonical home after decision |
| --- | --- | --- | --- |
| Ledger and reconciliation semantics | unresolved | Event/leg invariants, partial and one-to-many matches, reversal/undo, audit behavior, idempotency | `docs/specs/0002-database-schema.md`, `docs/specs/0005-review-and-commit-policy.md`, `docs/specs/0013-ledger-assets-valuation.md` |
| Evidence document lifecycle | unresolved | Meaning of Remove after commit; file deletion/archive behavior; encrypted original-file opening and temporary plaintext lifecycle | `docs/specs/0017-evidence-documents-source-ux.md` plus the ledger/security owner selected during design |
| Vault, file, backup, and restore keys | unresolved | Vault key lifecycle, file encryption boundary, backup key/KDF, atomic restore and recovery | `docs/specs/0009-backup-restore-versioning.md` and an ADR if architecture changes |
| Money Source identity model | unresolved | Institution/source/account/container/instrument hierarchy, stable account identity, archived accounts, parser-created candidates | `docs/specs/0014-money-overview-source-taxonomy.md`, `docs/specs/0002-database-schema.md` |
| Parser evidence contract | partial | OCR/native-text selection, evidence locations, locale/timezone/sign rules, stable record identity, reparse/supersede behavior | `docs/specs/0004-parser-contract.md` |
| Gmail data and cloud AI consent | unresolved | Owner-only versus public OAuth path, data sent to AI, consent granularity, provider retention and Limited Use compatibility | `docs/specs/0003-gmail-collector.md`, `docs/specs/0007-first-run-onboarding.md` |
| Restore and destructive job behavior | partial | Restore bootstrap outside the database being replaced, cancellation boundaries, state transitions and idempotency keys | `docs/specs/0015-job-engine-error-model.md`, `docs/specs/0009-backup-restore-versioning.md` |
| Desktop/storage architecture feasibility | blocked | Validate the Tauri/SQLCipher/FTS5 path, then accept, revise, or reject the proposed desktop architecture | `docs/adr/0001-local-first-tauri-react-sqlite.md` |
| AI ledger-authority ADR status | unresolved | Accept, revise, or reject the proposed AI authority boundary before implementation relies on it | `docs/adr/0002-agent-is-advisor-not-ledger-owner.md` |

## P1: required before the affected implementation slice

| Area | Status | Decision still required | Canonical home after decision |
| --- | --- | --- | --- |
| First-run optionality | unresolved | Whether AI and Gmail setup can be skipped and how incomplete setup resumes | `docs/specs/0007-first-run-onboarding.md` |
| Build and release target | unresolved | MVP operating systems, signing/notarization, packaging, update and release artifacts | future focused spec after the decision |
| Visual tokens and component version | unresolved | Exact accessible token values, HeroUI major/version contract, Figma role | `docs/specs/0008-design-system.md`, `docs/specs/0011-visual-design-tokens.md` |
| Security observability and sensitive-data lifecycle | unresolved | Log redaction, crash-report collection/consent, raw extraction retention, and deletion behavior | future focused security spec after the decision |
| Backup operations and portability | unresolved | Manual/automatic schedule, failure UX, Command Center status, export formats, and restore-migration detail | `docs/specs/0009-backup-restore-versioning.md`, `docs/specs/0015-job-engine-error-model.md` |

## P2: deliberately deferred, does not block the core MVP slices

| Area | Status | Decision still required | Canonical home after decision |
| --- | --- | --- | --- |
| Usage cadence and manual money | unresolved | Retention workflow, physical cash, and manual asset/liability scope | future focused product spec after the decision |
| AI Assistant operating scope | unresolved | Tool set, history, suggestion authority, and settings access after the core loop is stable | `docs/specs/0006-command-center-ui.md` or a future focused spec |
| Post-MVP extensions | blocked | Mobile, API connectors, valuation providers, tax reporting, parser sharing, and multi-device sync remain gated on the core loop | future roadmap after MVP evidence exists |
