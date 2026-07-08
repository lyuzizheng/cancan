# Product Lifecycle Breakdown

This document lists the full lifecycle of questions that must be aligned before CanCan becomes an implementation-ready solo-developer + AI-coding-agent project.

The goal is not to answer everything in this file immediately. The goal is to make sure nothing important is invisible.

## 0. Product Identity and Boundaries

Questions:

- What is CanCan's one-sentence promise?
- Is the product primarily evidence vault, reconciliation console, asset dashboard, AI finance assistant, or all of these in sequence?
- What is the MVP success condition?
- Which user behavior proves retention: daily review, weekly sync, monthly close, or ad-hoc audit?
- Which features are explicitly not MVP?
- What must never be automated: payments, trades, withdrawals, email mutation, secret access?

Current direction:

- CanCan is a local-first financial evidence vault and reconciliation console.
- Gmail automation is required for MVP success.
- Manual import is a test harness/fallback.
- AI finance assistant is future-facing and must use narrow backend APIs/skills.

## 1. User, Data, and Provider Scope

Questions:

- Which exact sources are MVP: DBS bank, DBS card, UOB bank, UOB card, Wise?
- Which sample files are required before claiming support?
- How are sample files redacted, named, stored, and tested?
- Should Singapore context be assumed in defaults?
- What base currency is default?
- How should physical cash/manual assets be handled?

Current direction:

- Default base currency is SGD.
- MVP providers are DBS/UOB bank/card and Wise PDF/CSV/export.
- Fixture policy is not aligned yet.

## 2. First-Run and Startup Sequence

Questions:

- What does the user see on first launch?
- Does vault creation happen before any screen?
- Is the app usable without AI provider setup?
- Is the app usable without Gmail setup?
- What order: vault -> base currency -> AI provider -> sources -> Gmail rules -> import sample?
- How does app resume unfinished jobs on startup?
- What happens if vault unlock fails?
- What happens if migrations fail?

Needed output:

- First-run flow spec.
- Startup/resume state machine.
- Error states and recovery paths.

## 3. Information Architecture and Navigation

Questions:

- What are the persistent navigation items?
- Is Command Center the first page after unlock?
- Where does AI Assistant live: sidebar item, command palette, right panel, or chat drawer?
- Should Jobs be a first-class nav item or hidden under Settings/Developer?
- How do Library, Sources, Transactions, Assets, and Reconciliation cross-link?

Current direction:

- Left sidebar + right main body.
- Command Center is central.
- AI Assistant should exist as a future surface.

## 4. Visual Design System

Questions:

- What palette should CanCan use?
- Light, dark, or adaptive theme first?
- What typography style: dense professional, consumer polished, terminal-like, editorial?
- What accent color communicates trust without generic fintech blue/purple?
- What density: table-first, card-light, or hybrid?
- What motion is allowed?
- What chart styles are acceptable?
- What empty/loading/error states are required?

Needed output:

- Design system spec: palette, typography, spacing, layout, elevation, component principles.
- Command Center wireframe.

## 5. Money Source and Account Model

Questions:

- Does user create Money Sources and sub-accounts manually before imports?
- What account types exist from day one?
- How are account aliases/last4/provider hints stored?
- Can AI suggest account mappings?
- Can AI create unresolved account candidates?
- How are disabled/archived accounts handled?

Current direction:

- User manually creates sources/sub-accounts.
- AI/parser suggests mappings but does not silently create official accounts.

## 6. Gmail Automation

Questions:

- Guided builder fields and expert Gmail query override shape?
- How is start date stored?
- What overlap window is default?
- How are attachments deduped?
- Are email bodies stored, or only metadata and attachments?
- How are OAuth errors surfaced?
- Does each Gmail rule map to a provider/source hint?
- How does user test a rule before enabling?

Current direction:

- Guided top builder + expert query editable below.
- Official read-only Gmail API.

## 7. Manual Import and Library

Questions:

- Which file types are supported first?
- What metadata can user edit?
- Can a document be reclassified manually?
- How does the Library show raw evidence vs derived artifacts?
- How does user inspect native text, OCR text, parsed records, validation output, and ledger links?
- What is the duplicate document UX?

Needed output:

- Library detail page spec.

## 8. Extraction, OCR, and AI Parser

Questions:

- Is OCR always run, or only after text extraction?
- How are native text and OCR combined for the LLM?
- What strict schemas exist for parser output?
- What provider-specific intermediate schemas are allowed?
- What validation failures block staging?
- How are prompt versions stored?
- Which Vercel AI SDK primitives are allowed?
- What provider/model settings are visible in UI?

Current direction:

- Preserve native extraction and OCR layer.
- AI-assisted parsing is core.
- Vercel AI SDK is allowed behind CanCan adapters.

## 9. Database, SQL, and Performance

Questions:

- Exact migration order?
- Which fields are columns vs JSON?
- Which indexes are mandatory?
- What benchmark sizes are required?
- Which queries are hot paths?
- How does test database reset work?
- How does schema migration work for existing vaults?
- How are backups schema-versioned?

Current direction:

- Hand-written SQL, no Prisma.
- Indexes and benchmarks required.
- JSON allowed for evolving metadata, not core filters.

## 10. Ledger, Assets, Positions, and Valuation

Questions:

- Are all events represented by ledger_events + ledger_legs?
- Should trades also get specialized trade tables?
- How are stock positions represented?
- How are current values and historical valuations stored?
- How are realized/unrealized gains represented?
- How are FX rates stored?
- How are insurance policy values represented?
- How are liabilities represented?
- How are balance snapshots prevented from double-counting income/spending?

Current direction:

- Ledger events/legs are canonical.
- Snapshots can enter ledger model but are not normal transactions.
- Trades and gains still need deeper alignment.

## 11. Reconciliation and Review

Questions:

- What can auto-commit?
- What must always be reviewed?
- What confidence thresholds are used?
- How are one-to-many and partial matches represented?
- How are credit card repayments linked?
- How are bank transfers, top-ups, FX conversions, and broker deposits linked?
- How does user correct a wrong match?
- How does user undo a commit?
- What audit log is required?

Current direction:

- Exact duplicate can auto-dedupe.
- Safe standalone purchases may auto-commit by policy.
- Transfers/repayments/top-ups/FX/broker links default to review.

## 12. AI Assistant and Backend Skill/API Layer

Questions:

- What assistant tools exist first?
- Does assistant have chat history?
- Can assistant summarize monthly financial state?
- Can assistant call read-only asset and transaction APIs?
- How are assistant outputs grounded in source evidence?
- Can assistant create review suggestions?
- Can assistant modify settings?
- How is assistant prevented from committing ledger changes?

Current direction:

- Assistant accesses narrow backend APIs/skills, not raw DB/files/secrets.

## 13. Security, Privacy, and Consent

Questions:

- What vault password/key model is used?
- Which secret backend first?
- Is file vault individually encrypted or covered by encrypted backup/DB?
- What is the cloud AI consent model?
- Are raw PDFs sent to cloud models by default? no.
- Are logs redacted?
- How are crash reports handled?
- How does restore handle secrets?

Current direction:

- SQLCipher/equivalent required from v1.
- AI providers opt-in.
- Secrets not included in backups by default.

## 14. Backup, Restore, and Portability

Questions:

- Is first backup target iCloud, generic folder, or both?
- What exact bundle format?
- How are checksums stored?
- How does restore verify integrity?
- How does restore handle schema migration?
- Are exports to CSV/JSON supported?
- Can a user inspect their data outside CanCan?

Needed output:

- Backup/restore spec.

## 15. Jobs, Errors, Observability

Questions:

- What job types exist?
- How are paused/failed jobs resumed?
- How much internal logging is visible to user?
- What is the difference between review item, parser error, and job failure?
- How are retries configured?
- How does user cancel a job?

Needed output:

- Job engine spec.

## 16. Testing and Fixtures

Questions:

- Which fixtures are allowed in repo?
- How are real statements redacted?
- What golden outputs are checked?
- What integration paths must be covered?
- How does test DB reset work?
- How does AI output get deterministic tests?
- What tests run before build/package?

Needed output:

- Fixtures and testing spec.

## 17. Build, Packaging, and Release

Questions:

- What OS is MVP: macOS only or cross-platform?
- What build commands are required?
- How does AI coding agent package app locally?
- Is code signing required in early builds?
- Is auto-update needed?
- What artifacts are produced?
- How does release note/progress update work?

Needed output:

- Build/package spec.

## 18. Autonomous AI Coding Agent Workflow

Questions:

- What docs must agent read before coding?
- What planning format should agent use?
- What validation must agent run?
- How should agent inspect UI with browser/computer-use?
- When should agent ask user instead of guessing?
- How are docs updated after implementation?
- How are integration tests and DB reset required?

Current direction:

- Agent should read `docs/agent/` and `docs/specs/` first.
- Agent should implement small slices, test, build, inspect UI, update docs/progress.

## 19. Long-Term Product Extensions

Questions:

- When does mobile read-only viewer matter?
- When do API connectors matter?
- When do valuation providers matter?
- When do tax/capital-gains reports matter?
- When does parser marketplace/config sharing matter?
- When does multi-device sync become necessary?

Needed output:

- Later roadmap gates.
