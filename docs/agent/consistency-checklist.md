# Consistency Checklist

Use this before considering a docs or code change complete.

## Product consistency

- [ ] Does the change preserve the core product loop: Source -> Evidence -> Extract -> AI Normalize -> Validate -> Reconcile -> Review -> Ledger?
- [ ] Is Gmail automation still treated as MVP success and manual import as test harness/fallback?
- [ ] Are provider claims backed by fixtures or explicitly marked planned?
- [ ] Are user-created Money Sources and sub-accounts respected?

## Data consistency

- [ ] Are native currency/instrument values preserved?
- [ ] Is base-currency value derived and timestamped when needed?
- [ ] Are balance/valuation snapshots prevented from being counted as normal spending/income?
- [ ] Are credit card repayments prevented from double-counting spending?
- [ ] Can every committed event trace back to source evidence?

## AI and parser consistency

- [ ] Is AI output schema-validated?
- [ ] Is deterministic financial validation applied before staging/commit?
- [ ] Are prompt/model/parser versions recorded?
- [ ] Are AI permissions narrow and unable to read secrets or mutate committed ledger directly?
- [ ] Are ambiguous links routed to Review?

## Security consistency

- [ ] Are secrets stored outside plain SQLite?
- [ ] Is the vault encrypted at rest?
- [ ] Are connectors read-only?
- [ ] Does Gmail use official read-only OAuth/API for MVP?
- [ ] Are cloud AI provider uploads opt-in?

## Documentation consistency

- [ ] Did the relevant numbered docs change if behavior changed?
- [ ] Did `docs/agent/current-state.md` change if the current focus changed?
- [ ] Did `docs/agent/progress-log.md` get a dated entry for meaningful progress?
- [ ] Are open questions moved to resolved decisions when answered?
