# ADR 0004: Audit BRAWUKA-302 retention decisions (Gmail, statement passwords, cloud OCR, synthetic core, test APIs)

## Status

Accepted

## Context

Follow-up audit BRAWUKA-302 (parent BRAWUKA-297, dimension 1) flagged the Gmail
chain, `remove_statement_password`, `cloud-ocr-provider.ts`, test-only database
APIs, and `SyntheticCoreRepository` as dead or duplicated code. Each item was
checked against the canonical specs before deciding retain, relocate, or remove.

## Decision

- Retain the Gmail chain (`runtime/gmail.rs`, `runtime/gmail_connector.rs`,
  `database/gmail.rs`, migration `0010_gmail_accounts.sql`,
  `packages/connectors/src/gmail-oauth*.ts`, the `cancan-gmail-connector`
  `externalBin` entry). Spec 0003 keeps Gmail as an optional evidence channel
  with a blocked-but-planned onboarding checkpoint (development identity and
  public verification in `docs/alignment-temp/alignment-progress.md`); the
  chain is the mocked-contract stage of that plan, not abandoned code. Files
  now carry a test-only header until the checkpoint wires a renderer command.
- Retain `remove_statement_password` as a registered Tauri command. Spec 0003
  requires that users can delete saved statement passwords from Settings; the
  command is the host half of that contract and the renderer contract is
  already typed (`RemoveStatementPasswordArgs`, `vault-api.removeStatementPassword`).
- Retain `packages/ai/src/cloud-ocr-provider.ts`. Spec 0004 explicitly approves
  the cloud-image OCR adapter plus injected-fetch executor for a manually
  imported image, while excluding credential storage, app/Tauri/sidecar/UI
  wiring, logging, live calls, and provider-specific parsing. The file header
  now records that boundary.
- Retain `SyntheticCoreRepository` (`packages/db`) with no drift gate. Spec
  0016 owns the synthetic fixture/testing gate; `@cancan/db` is imported by no
  production or app surface (only its own tests), so there is no production
  drift vector. A drift gate is deferred until a production consumer appears.
- Relocate test-only database APIs from `database/mod.rs` into the
  `*_test_support` extension pattern (`database_test_support.rs`,
  `imports.rs` test helpers, `intake.rs` keeps only the production path):
  `register_import`, `register_captured_import`, `register_prepared_import`
  (now `test_support_*`), `persist_captured_import` (now
  `test_support_persist_captured_import`), `apply_trusted_classification`
  (test shorthand; production uses `apply_trusted_classification_for_parse_job`),
  `StructuredParseTestState` readers, `expire_*_lease_for_test`,
  `seed_money_source`, `confirm_candidate_accounts`. `runtime/review.rs`
  `seed_money_source` moved to `runtime/tasks_test_support.rs`. Behavior is
  unchanged; `cargo check --locked --tests` passes.

## Consequences

- No production behavior changes; no migration changes.
- The next Gmail onboarding checkpoint must wire the renderer command and
  remove the test-only headers; the Settings surface must expose password
  deletion through the retained command.
