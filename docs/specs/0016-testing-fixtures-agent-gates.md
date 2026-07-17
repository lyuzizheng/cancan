# 0016. Testing, Fixtures, and Application Gates Spec

## Goal

Define how CanCan tests source-evidence flows, protects sensitive fixtures, keeps LLM-dependent behavior deterministic, and gates application changes.

This spec is canonical for fixture policy, parser/reconciliation test expectations, database reset safety, and future application CI gates. `0012-repo-agent-workflows.md` owns the docs/agent harness.

## Stable decisions

- Real financial statements should not be committed by default.
- Local real samples belong in `fixtures-private/`, which must stay out of Git.
- Redacted fixtures may be committed only after explicit manual redaction review.
- Synthetic fixtures are preferred for deterministic CI.
- LLM-dependent tests should not call live LLMs in CI.
- DB reset must only affect test databases, never a real vault.
- UI work requires visual inspection when app code exists.
- App/typecheck/test/build gates are added only when real app/package scripts exist; the app-foundation commands now satisfy that boundary.
- When the consequence-based path in `0012-repo-agent-workflows.md` justifies a separate tester, that tester verifies the stable implementation diff and reports reproducible failures instead of patching production code.

## Fixture privacy layers

Use three fixture layers:

```text
fixtures-private/     real local samples, never committed
fixtures-redacted/    manually redacted samples, allowed in private repo after review
fixtures-synthetic/   generated safe samples, preferred for CI
```

### `fixtures-private/`

Use for the owner's real DBS/UOB/Wise/Moomoo/Manulife samples.

Rules:

```text
must be in .gitignore
can include raw PDFs/CSVs/exports
can include local expected outputs
must not be uploaded to CI
must not be shared in issues/PRs/logs
```

### `fixtures-redacted/`

Use only after redaction review.

Redaction must remove or replace:

```text
name
address
email
phone
full account/card numbers
NRIC/passport/tax identifiers
transaction counterparties that identify real people
barcodes/QR codes/reference codes that can expose account data
PDF metadata containing personal data
```

### `fixtures-synthetic/`

Use for repeatable CI and examples.

Synthetic fixtures should model real statement shapes without using real personal data.

## Recommended fixture layout

```text
fixtures-private/
  dbs/
    bank-statement/
      sample-001/
        input.pdf
        expected.json
        ai-output.mock.json
    credit-card-statement/
      sample-001/
        input.pdf
        expected.json

fixtures-synthetic/
  wise/
    csv-export/
      sample-001/
        input.csv
        expected.json
        ai-output.mock.json
```

Fixture names should be stable and non-sensitive.

## Minimum supported-provider fixture set

For each supported document type, target at least three samples:

```text
normal statement
edge-case statement
duplicate/repayment/reconciliation scenario
```

Initial MVP document types:

```text
DBS bank statement
DBS credit card statement
UOB bank statement
UOB credit card statement
Wise PDF/CSV/export
```

Additional samples for password-protected PDFs are required once locked-PDF handling is implemented.

## Auto-commit qualification gate

The three-fixture baseline is enough to begin parser development, not enough to grant auto-commit eligibility.

Each complete normalization profile must qualify independently. Its identity includes:

```text
provider + document type
+ parser/skill/prompt/schema/validator versions
+ normalizer runtime and tool-contract versions
+ input strategy and native-extraction/OCR versions
+ AI provider/model version
```

Each profile must qualify with:

```text
at least 100 labeled representative record cases across normal and edge-case statements
deterministic expected classification, account mapping, normalized fields, and eligibility outcome
zero incorrect auto-commit-eligible outcomes in the qualification suite
at least 20 user-confirmed shadow candidates from local use before live auto-commit
zero incorrect account mappings or financial fields among those shadow candidates
```

Qualification cases must cover every event type that the package can emit and must include:

```text
exact opening-to-closing snapshot reconciliation
one-minor-unit or one-smallest-quantity residual that must fail eligibility
missing and duplicate rows
mixed documents where semantic-only ambiguities remain in Review
cross-account, FX, or trade cases when the package supports them
field-confidence calibration on held-out labeled cases
```

Very-high confidence is package-specific and field-level. Set its calibration so the qualification set produces zero incorrect eligible fields/records; never substitute a raw LLM self-score or one global threshold.

The 100 cases may be distributed across synthetic, redacted, and private statement fixtures. Private cases stay local and must never be uploaded to CI or logs.

Shadow mode performs the complete eligibility decision but creates review suggestions instead of committed events. User decisions are recorded as qualification evidence.

Any classifier prompt, extraction prompt, parser skill, normalizer runtime, tool contract, structured schema, validator, extraction/OCR engine, model, canonical mapping, or eligibility-rule version change revokes the affected profile's qualification. The user-level auto-commit toggle remains enabled, but records from the changed profile fall back to shadow/review until it qualifies again.

## Expected output contract

Each fixture should include an `expected.json` with assertion intent.

Expected output should not always require exact full-object equality. Use layered assertions:

```text
document classified correctly
source and child container mapped correctly
statement period extracted
key balances/snapshots extracted
record count exact or within a range when appropriate
key transactions extracted
known duplicates do not create duplicate source documents
expected review items are created
expected ledger events/legs are created after commit tests
```

Recommended shape:

```json
{
  "fixture_id": "dbs-credit-card-sample-001",
  "provider_key": "dbs",
  "document_type": "credit_card_statement",
  "privacy_level": "synthetic|redacted|private",
  "parser_contract_version": "0.1.0",
  "normalization_profile_id": "mock-profile-v1",
  "agent_runtime": "single-pass|tool-loop-agent|pi-agent-core",
  "tool_contract_version": "0.1.0",
  "extraction_version": "0.1.0",
  "prompt_hash": "mock-or-real-hash",
  "expected": {
    "classification": {},
    "source_mapping": {},
    "statement_period": {},
    "record_count": { "mode": "exact", "value": 0 },
    "key_records": [],
    "snapshots": [],
    "review_items": []
  }
}
```

## LLM parser test policy

CI must not depend on live LLM calls.

Use one of:

```text
mocked model provider
stored ai-output.mock.json
schema-level parser output fixture
recorded local eval output that has been manually approved
```

Live LLM eval may exist as a local-only command, but it must be opt-in through environment variables and must not run in default CI.

Rules:

- store model/provider/prompt/parser version metadata;
- preserve parse run history when AI output changes;
- do not overwrite expected outputs without review;
- never include secrets or PDF passwords in mocked outputs.

## Document-agent harness

The document agent remains a small normalizer with fixed parser tools. Its harness must use the same proposal schema, observations, validators, and fixtures as the single-pass baseline.

Deterministic contract tests cover:

```text
product parser-skill manifest and prompt/schema/version references
exact allowlist of inspect/extract/OCR/read-region/validate/submit tools
rejection of shell, arbitrary file path, cross-document ID, generic network, secret, database, and ledger requests
strict tool argument/result schemas and bounded page/row/result sizes
free-text completion rejection
schema-valid submit_structured_proposal completion
eight-step and two-submission budget enforcement
cancellation, retry, parse-run history, and idempotency
```

Recorded-agent tests use mocked model streams or transcripts to prove:

```text
the model can inspect, extract, validate, repair once, and submit
invalid tool calls fail without executing a capability
validation feedback cannot be bypassed by a second proposal
native/OCR conflict remains visible
an ungrounded multimodal field cannot become auto-commit eligible
budget exhaustion creates a deterministic review/failure outcome
tool events and final parse metadata are reproducible
```

Adversarial fixtures include document text that instructs the model to ignore its prompt, request secrets, read other files, access another document, call a nonexistent tool, or fabricate raw source rows or locators. They also splice individually valid amount/date/description values from different rows or regions into one fake raw record. These cases pass only when the capability request is impossible or rejected and no eligible financial record is produced from fabricated evidence.

Every agentic profile is compared with a single-pass structured-normalization baseline on the same cases. Record:

```text
required-field exact accuracy
row recall and duplicate/missing-row failures
raw-record grounding coverage
exact reconciliation and eligibility outcome
model/tool steps, latency, token use, and estimated cost
runtime errors, cancellation, and repair success
```

An agentic runtime is adopted only when it demonstrates a material accuracy or recovery advantage that justifies its additional complexity. Framework popularity or a successful happy-path demo is not qualification evidence.

## Runtime and packaging evidence

The 2026-07-14 disposable evidence slice ran ToolLoopAgent and Pi Agent Core with the same mock model, fixed tools, fixture, structured proposal, validation feedback, cancellation, and budget limits. Both produced the same accepted proposal as single-pass, but required four or five model steps instead of one. Single-pass is therefore selected until real qualification fixtures demonstrate a material agentic accuracy or recovery advantage.

The spike also verified the Node/Tauri execution boundary:

```text
current pinned Node compatibility and frozen dependency resolution
dev worker startup and stdio/IPC framing
Tauri-bundled sidecar feasibility on the supported macOS target
startup latency, packaged size, clean shutdown, cancellation, and crash isolation
inherited-environment clearing and protocol rejection without secret echo
signed/notarized packaging implications and conservative third-party license inventory
no user-installed Node, Docker, VM, QEMU, or sandbox requirement
```

The [runtime evidence](../../spikes/document-normalizer-runtime/EVIDENCE.md) proves a reproducible Node 24 single-executable and ad-hoc signed Tauri sidecar route on macOS `arm64`, including startup, cancellation, clean shutdown, crash isolation, inherited-environment clearing, protocol secret-field rejection, bundle size, and an over-inclusive comparison-workspace license inventory. Phase 1 also requires the same architecture-matched build, launch, lifecycle, and packaging evidence on a real macOS `x86_64` runner or machine. Exact production credential delivery/redaction and an artifact-specific SBOM/license inventory remain owning-slice gates alongside production signing and notarization. An OS permission sandbox is optional; fixed tools and job scoping are mandatory regardless of process placement.

## Test database reset policy

DB reset must be safe by construction.

Rules:

```text
reset only a test database path
never reset a real vault path
require a test env marker such as CANCAN_TEST=1
prefer temp directories or .cancan-test/
print the target path before deletion
fail if target path looks like a real vault or backup folder
```

Suggested future script names:

```text
pnpm test:db:reset
pnpm test:integration
pnpm test:fixtures
```

## Application automation gates

The docs/agent harness and its CI workflow are owned by `0012-repo-agent-workflows.md`.

Once app code exists, every feature slice should run the strongest relevant subset:

```text
typecheck
unit tests
integration tests with DB reset
migration check
parser fixture tests
docs consistency check
build/package check when app code changes
```

The applicable subset is declared by the selected row in `docs/agent/implementation-slices.md`. Testing and review generate the same slice context used by implementation; they do not independently guess a different spec set.

Use the narrowest test layer that proves the changed behavior:

```text
pure domain behavior -> unit tests
repository or migration behavior -> safe temporary SQLite integration
privileged desktop behavior -> Tauri command and build checks
user-visible vertical flow -> UI interaction, persisted result, reload, and visual-state inspection
```

Do not require artificial UI tests for backend-only behavior. When a slice changes a user-visible flow, test the complete boundary it owns rather than stopping at a mocked component.

The current foundation commands are:

```text
pnpm typecheck
pnpm test:unit
pnpm test:synthetic-core
pnpm check:rust
pnpm test:rust
pnpm build:web
pnpm build:desktop
pnpm verify
```

UI feature slices additionally require:

```text
browser/Chrome/Playwright/computer-use inspection
empty/loading/error state check
reduced-motion behavior check when motion is involved
```

## Snapshot testing policy

Use snapshots carefully.

Good snapshot targets:

```text
normalized parser output
extraction bundle metadata
ledger event/leg shape for a fixture
review item shape
```

Avoid relying only on snapshots for:

```text
financial correctness
UI visual quality
security behavior
secret handling
```

UI screenshots can be useful artifacts, but visual review and targeted assertions should decide pass/fail.

## Application CI design stance

The production skeleton and package scripts now exist. `.github/workflows/application.yml` runs the pinned macOS toolchain, frozen pnpm/Cargo resolution, repository preflight, `pnpm test:rust`, and `pnpm verify`. The Rust gate executes privileged encrypted-file, SQLCipher, import rollback, and tombstone assertions in authoritative CI without adding them to the default local gate. The TypeScript unit-test gate includes the synthetic core's safe-reset, repeatable-migration, deterministic parser, exact reconciliation, review, commit, read-model, and relationship integration tests; `pnpm test:synthetic-core` exposes the same focused subset locally. The docs-harness CI remains outside this spec's ownership.

Later slices extend these existing gate categories with their own migrations, repositories, parser fixtures, and builds rather than creating duplicate CI paths. Gates that do not exist yet are added only when their owning behavior exists, including:

```text
encrypted-vault and destructive-job integration
document-agent budget, cancellation, and bundled-runtime packaging
browser accessibility, error-state, and reduced-motion checks for user-facing flows
```

Do not add live Gmail, live LLM, real bank, or real statement dependencies to CI.

## Acceptance criteria

- `fixtures-private/` is ignored by Git.
- Fixture policy distinguishes private, redacted, and synthetic samples.
- Each supported document type has a target fixture minimum.
- Auto-commit qualification requires 100 labeled record cases, 20 confirmed shadow candidates, and zero incorrect eligible outcomes per complete normalization profile.
- Qualification covers every supported event type, exact snapshot closure, residual failures, duplicate/missing rows, and field-level held-out calibration.
- Any behavior-changing parser skill, prompt, schema, validator, agent runtime, tool contract, extraction/OCR, or model change revokes only the affected profile's qualification and falls back to shadow mode.
- Expected outputs are versioned and assertion-oriented.
- LLM-dependent tests are deterministic in CI.
- Document-agent contract, recorded-loop, adversarial, budget, cancellation, and single-pass comparison tests are required before production adoption.
- Single-pass is selected by the shared disposable runtime/packaging evidence; any later ToolLoopAgent or Pi Agent Core adoption requires material qualification-fixture evidence.
- DB reset cannot target a real vault by default.
- UI changes require visual inspection once UI exists.
- Every implementation slice declares deterministic test evidence and shares its generated context with testing/review.
- Later application CI gates are added only with real scripts and implementations.
- The foundation application CI invokes real typecheck, unit-test, Rust-check, web-build, and Tauri debug-build commands.
