# Product Rules

These rules apply to every CanCan task.

- CanCan is a local-first financial evidence vault and reconciliation console, not a budgeting app.
- The product loop is `Source -> Evidence -> Extract -> AI Normalize -> Validate -> Reconcile -> Review -> Ledger`.
- Gmail automation is part of MVP success; manual import is a test harness and fallback.
- Every committed financial record must trace back to source evidence.
- MVP does not ask for base currency and does not default to one large Net Worth number.
- Do not fetch market prices or external FX rates in MVP.
- AI may extract, normalize, propose, rank, and explain. Deterministic validation and user/policy gates own committed ledger state.
- Do not build payments, trading, bill pay, mobile sync, tax reporting, autonomous financial advice, or external market-data fetching before the core evidence/reconciliation loop works.
