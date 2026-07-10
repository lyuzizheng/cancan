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
- App/typecheck/test/build gates wait until real app/package scripts exist.

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

Add application CI gates when the app skeleton and package scripts exist. The existing docs-harness CI is outside this spec's ownership.

Future CI should include:

```text
lint/typecheck
unit tests
migration check
integration tests with reset test DB
fixture parser tests using mocked AI output
build check
```

Do not add live Gmail, live LLM, real bank, or real statement dependencies to CI.

## Acceptance criteria

- `fixtures-private/` is ignored by Git.
- Fixture policy distinguishes private, redacted, and synthetic samples.
- Each supported document type has a target fixture minimum.
- Expected outputs are versioned and assertion-oriented.
- LLM-dependent tests are deterministic in CI.
- DB reset cannot target a real vault by default.
- UI changes require visual inspection once UI exists.
- Future application CI gates are defined without inventing scripts that do not exist.
