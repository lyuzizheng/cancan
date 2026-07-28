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

First-preview source/profile target:

```text
DBS bank statement
DBS credit card statement
HSBC source support through whichever real package/profile first passes review-only gates
UOB source support through whichever real package/profile first passes review-only gates
```

DBS Bank and DBS Card are different child accounts and document profiles under one DBS Money Source, not separate Money Sources. HSBC and UOB are also first-preview source targets, but neither needs a fixed Bank/Card matrix and a profile may launch Review-only. An empty Source definition is not support: each advertised source needs at least one real package/profile with deterministic fixtures and review-only runtime evidence. CPF and additional providers are desirable follow-ons but do not block the first preview unless explicitly added to the release slice.

Additional samples for password-protected PDFs are required once locked-PDF handling is implemented.

## Auto-commit confidence-calibration gate

The three-fixture baseline is enough to begin parser development, not enough to calibrate an auto-commit confidence threshold.

Each complete normalization profile calibrates independently. Its identity includes:

```text
provider + document type
+ parser/skill/prompt/schema/validator versions
+ normalizer runtime and tool-contract versions
+ input strategy and native-extraction/OCR versions
+ AI provider/model version
```

Before a profile can auto-commit, its evidence must include:

```text
representative labeled record cases across normal and edge-case statements
deterministic expected classification, account mapping, normalized fields, and eligibility outcome
zero incorrect auto-commit-eligible outcomes in the accepted calibration suite
user-confirmed shadow candidates from local use before live auto-commit
zero incorrect account mappings or financial fields among those shadow candidates
```

The owner has replaced the fixed `100 labeled + 20 shadow` rule with profile-specific evidence and explicit release approval. There is no universal minimum case count or global threshold. Each profile derives its required-field minimum from its own representative held-out and user-confirmed shadow evidence; no threshold is inherited or copied between profiles.

The calibration report must identify the complete profile and exact threshold; describe representative and held-out case composition; report eligible, Review, and error counts; show required-field confidence distributions and the weakest eligible cases; list shadow outcomes; record the count and explanation of every incorrect eligible outcome; and record explicit owner approval. An uncalibrated model self-score or any incorrect auto-commit-eligible outcome keeps the profile in shadow/Review.

Calibration cases must cover every event type that the package can emit and must include:

```text
exact opening-to-closing snapshot reconciliation when the profile/event contract supplies snapshots
one-minor-unit or one-smallest-quantity residual that must fail eligibility for a snapshot-gated profile/event
missing and duplicate rows
mixed documents where semantic-only ambiguities remain in Review
cross-account, FX, or trade cases when the package supports them
field-confidence calibration on held-out labeled cases
```

High confidence is package/document-specific and field-level. The model returns confidence and evidence per required field; the host uses the weakest required-field confidence plus event guardrails, then applies the calibrated threshold and every accepted deterministic hard gate. Optional fields cannot average away a weak required field, and one global threshold or confidence alone is insufficient.

Representative calibration cases may be distributed across synthetic, redacted, and private statement fixtures. Private cases stay local and must never be uploaded to CI or logs.

Shadow mode performs the complete eligibility decision but creates review suggestions instead of committed events. User decisions are recorded as confidence-calibration and hard-gate evidence.

Any classifier prompt, extraction prompt, parser skill, normalizer runtime, tool contract, structured schema, validator, extraction/OCR engine, model, canonical mapping, or eligibility-rule version change invalidates the affected profile's confidence calibration. The user-level auto-commit toggle remains enabled, but records from the changed profile fall back to shadow/review until its calibration and hard-gate evidence pass again.

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

The cloud-image OCR provider and transport checkpoints use only inline synthetic PNG data URLs, an injected mocked fetch boundary, and fake OpenAI-compatible Chat Completions JSON. Their deterministic tests assert complete dedicated-versus-analyser configuration selection, the exact fixed prompt/request shape, successful request execution and plain-text content parsing, network/non-2xx/malformed-response rejection, and that neither an API key nor provider response body appears in JSON bodies or errors. Default tests make no network request. A later single opt-in local smoke test may use environment variables only outside CI and must exercise this same executor with an app-produced upright bounded PNG; production credential storage belongs to a later Keychain-owned capability.

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

An agentic runtime is adopted only when it demonstrates a material accuracy or recovery advantage that justifies its additional complexity. Framework popularity or a successful happy-path demo is not evaluation evidence.

## Runtime and packaging evidence

The 2026-07-14 disposable evidence slice ran ToolLoopAgent and Pi Agent Core with the same mock model, fixed tools, fixture, structured proposal, validation feedback, cancellation, and budget limits. Both produced the same accepted proposal as single-pass, but required four or five model steps instead of one. Single-pass is therefore selected until real evaluation fixtures demonstrate a material agentic accuracy or recovery advantage.

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

The local-inbox and Gmail slices add deterministic fixtures for:

```text
picker, drag/drop, Open With, and watched-folder paths reaching the same artifact identity
folder rescan after restart, exact duplicate, cloud placeholder/provider-error deferral, and no source-file mutation
changing size/mtime/file identity across settle/read checks never reaching Vault before one stable final capture
restart/rescan after user deletion remaining suppressed until explicit restore
unassigned evidence routing to exactly one configured source/account or one compact review action
canonical no-attachment email evidence and overlapping-rule message idempotency
generic Inbox alias/label boundary excluding unrelated mailbox attachments
Spam/Trash, display-name spoof, sender-domain mismatch, and failed/missing authentication remaining untrusted
email-first and statement-first convergence to one canonical ledger event
late corroborating evidence against an already committed event, with unchanged legs/allocations and one atomic audit entry
ambiguous/different-amount notification matches remaining in Review
```

These are mocked local/Gmail fixtures. CI must not require a live mailbox, sync provider, iCloud account, phone, bank app, or real statement.

The Phase 2 macOS Finder Share slice adds native packaged tests for:

```text
single and bounded multi-file Share > CanCan handoff
running, launched, locked, and cancelled app states
unsupported type and over-limit rejection
shared Add/capture idempotency and Processing/Needs attention projection
bounded App Group staging while closed/locked, with private protection, no-backup/no-index behavior, atomic handoff, success/cancel/expiry cleanup, and crash recovery
no Vault key, parser, database, source registry, or durable job queue in the extension/service
signed/notarized package registration on each supported macOS architecture
```

The separate `statement-coverage` slice adds deterministic mocked-AI fixtures for:

```text
bounded source-analysis API authority and redaction
one Money Source/account/document-type scope per invocation
accepted statement-period and processing-state inputs
one bounded follow-up snippet request, at most three documents, two pages per document, and 8,000 extracted characters total
cross-scope, over-count, over-page, over-character, second-request, and full-document rejection
AI proposals that cite only supplied source/account/document/period references
host rejection of fabricated, stale, or out-of-scope references
scheduled-job retry and idempotency after its trigger policy is accepted
source/account/period/processing changes marking a scope due
no-daemon startup/unlock catch-up only when the scope is due and lacks a successful evaluation for the current local day
exact-period No statement this period suppression without suppressing future periods
Remind later disappearance and reappearance after the validated future date
```

Coverage CI uses no live model, provider account, or cloud source.

## Hosted required-check integrity

A required hosted check must actually start and pass before normal merge. A no-start result caused by billing, spending limits, runner availability, or account configuration is infrastructure failure, not application-test evidence and not an automatic waiver. Local verification may diagnose the revision and support a separately recorded maintainer exception, but it does not silently replace the normal required-check gate.

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

The production skeleton and package scripts now exist. `.github/workflows/application.yml` runs repository preflight and `pnpm verify:fast` on Linux with frozen pnpm resolution for application pull requests and `main`. `.github/workflows/application-native.yml` runs repository preflight and `pnpm verify:native` with the pinned macOS toolchain and frozen pnpm/Cargo resolution only for ready native, sidecar/parser, migration, dependency, or toolchain changes, with manual dispatch available. The native Rust gate restores bounded Cargo cache data and builds the sidecar once before executing privileged encrypted-file, SQLCipher, import rollback, tombstone assertions, clippy, and the desktop build in authoritative CI; standalone commands remain self-contained. The root TypeScript unit gate excludes `spikes/**`; disposable spike tests run only through their isolated package/workflow gates and dependencies. The production TypeScript unit-test gate includes the synthetic core's safe-reset, repeatable-migration, deterministic parser, exact reconciliation, review, commit, read-model, and relationship integration tests; `pnpm test:synthetic-core` exposes the same focused subset locally. The docs-harness CI remains outside this spec's ownership.

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
- Auto-commit requires a package/document-specific structured AI-confidence threshold plus every accepted deterministic hard gate; confidence alone is insufficient.
- Calibration covers every supported event type, snapshot closure/residual failures where the profile contract uses that gate, duplicate/missing rows, per-required-field held-out confidence, and shadow outcomes. It uses no universal labeled/shadow count or global threshold; each profile records its evidenced threshold and accepted report, shows zero incorrect eligible outcomes, and receives explicit owner approval.
- Any behavior-changing parser skill, prompt, schema, validator, agent runtime, tool contract, extraction/OCR, or model change invalidates only the affected profile's calibration and falls back to shadow mode.
- Expected outputs are versioned and assertion-oriented.
- LLM-dependent tests are deterministic in CI.
- Document-agent contract, recorded-loop, adversarial, budget, cancellation, and single-pass comparison tests are required before production adoption.
- Single-pass is selected by the shared disposable runtime/packaging evidence; any later ToolLoopAgent or Pi Agent Core adoption requires material evaluation-fixture evidence.
- DB reset cannot target a real vault by default.
- UI changes require visual inspection once UI exists.
- Every implementation slice declares deterministic test evidence and shares its generated context with testing/review.
- Local Inbox, email-notification, and cross-channel reconciliation behavior has deterministic fixtures without live cloud accounts. The later statement-coverage slice uses a mocked AI capability and capability-specific bounded input fixtures rather than a live model or cloud account.
- Phase 2 Finder Share intake proves bounded protected App Group handoff/cleanup into the existing Add/capture path without duplicating Vault/parser/source-registry/job ownership.
- Later application CI gates are added only with real scripts and implementations.
- The foundation application CI invokes real typecheck, unit-test, Rust-check, web-build, and Tauri debug-build commands.
- Required hosted checks must start and pass for normal merge; infrastructure no-start results are never represented as green code evidence.
- Presentation-safe Rust-to-TypeScript generated types have a deterministic drift gate; handwritten semantic wrappers and current React state remain outside code generation.
- Generated Rust-to-TypeScript files are committed; native CI builds the sidecar and runs the focused Rust test that compares generated output with the committed file.
- Add/capture integration covers failure before blob durability, failure or rollback of the atomic source-registration/parse-job transaction, success before asynchronous parsing completes, and later parse failure without losing the captured source.
- Migration integration upgrades an existing `0007_local_inbox.sql` database through the appended hardening migration without editing history or losing a dogfood Vault.
- Long OCR/extraction/sidecar tests prove unrelated bounded store reads remain responsive and stale post-work writes fail their version/idempotency validation.
- Per-candidate account fixtures cover mixed accept/reject, all-or-nothing stale-batch rejection, dismissed projection hiding, same-profile reparse idempotency, one-action restore of the account plus latest current projections, superseded versions remaining hidden, and preserved source/parse/record/audit evidence.
