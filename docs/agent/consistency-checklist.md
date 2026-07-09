# Consistency Checklist

Use this before considering a docs or code change complete.

## Product consistency

- [ ] Does the change preserve the core product loop: Source -> Evidence -> Extract -> AI Normalize -> Validate -> Reconcile -> Review -> Ledger?
- [ ] Is Gmail automation still treated as MVP success and manual import as test harness/fallback?
- [ ] Are provider claims backed by fixtures or explicitly marked planned?
- [ ] Are user-created Money Sources and child containers/accounts respected?
- [ ] Does the future AI Assistant access data only through narrow backend APIs/skills?

## Data consistency

- [ ] Are native currency/instrument values preserved?
- [ ] Are source-provided equivalent values preserved without requiring a base currency?
- [ ] Are balance/valuation snapshots prevented from being counted as normal spending/income?
- [ ] Are credit card repayments prevented from double-counting spending?
- [ ] Can every committed event trace back to source evidence?
- [ ] Are hot query paths indexed and benchmarked where needed?
- [ ] Are JSON fields used only for metadata/evolving shapes, not core filters?

## AI and parser consistency

- [ ] Is AI output schema-validated?
- [ ] Is deterministic financial validation applied before staging/commit?
- [ ] Are prompt/model/parser versions recorded?
- [ ] Are LLM-dependent tests deterministic through mocked/stored outputs?
- [ ] Are AI permissions narrow and unable to read secrets or mutate committed ledger directly?
- [ ] Are ambiguous links routed to Review?
- [ ] If Vercel AI SDK is used, is provider-specific behavior still hidden behind app-level adapters?

## Fixture and testing consistency

- [ ] Are real statement fixtures kept in ignored `fixtures-private/`?
- [ ] Are committed fixtures synthetic or explicitly redacted/reviewed?
- [ ] Does DB reset target only a test database/path?
- [ ] Are expected outputs versioned when parser/prompt/extraction behavior changes?
- [ ] Are UI changes visually inspected when app code exists?

## Security consistency

- [ ] Are secrets stored outside plain SQLite?
- [ ] Is the vault encrypted at rest?
- [ ] Are connectors read-only?
- [ ] Does Gmail use official read-only OAuth/API for MVP?
- [ ] Are cloud AI provider uploads opt-in?
- [ ] Are statement PDF passwords excluded from logs, prompts, fixtures, and default backups?

## UI consistency

- [ ] Does the app use left sidebar + main body layout for core desktop UI?
- [ ] Is the first screen useful as an asset/reconciliation Command Center?
- [ ] Are review interactions simple enough for regular use while still preventing bad links?
- [ ] Are empty/loading/error states implemented?
- [ ] Has the UI been inspected visually via browser/computer-use when available?

## Documentation consistency

- [ ] Did `docs/specs/` change if implementation contracts changed?
- [ ] Did `docs/agent/current-state.md` change if the current focus changed?
- [ ] Did `docs/agent/progress-log.md` get a dated entry for meaningful progress?
- [ ] Are open questions moved to resolved decisions when answered?
