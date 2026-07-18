# 0004. Parser Contract Spec

## Goal

Define how PDF, CSV, image, and other supported source evidence becomes staged structured records through source observation, a mandatory AI normalizer, evidence grounding, schema validation, and deterministic financial validation.

The AI normalizer may be a single structured-generation call or a small capability-limited document agent. It produces proposals only and never owns ledger mutation.

## Stable decisions and remaining evidence

The parser evidence and normalized-data contract is accepted. The product may implement it without inventing OCR, evidence, date, or sign semantics.

Single-pass structured normalization in the bundled Node sidecar is selected for initial production implementation. Production credential delivery/redaction remains an owning-slice gate. Raw full-text retention remains owned by the separate sensitive-data-lifecycle blocker; the source-file deletion behavior below is accepted.

## Pipeline

Native extraction and OCR are alternative or complementary sources of observations. They are not fixed serial stages, and neither may emit canonical financial records.

```text
source_document
-> input inspection and native extraction
-> provider/document classification
-> deterministic provider fingerprint verification
-> input planning: native text, table extraction, OCR, original pages, or a combination
-> selected provider/document parser skill
-> mandatory AI normalization
-> structured parse proposal
-> schema validation
-> evidence grounding
-> deterministic financial validation
-> external_records and source-backed ledger observations
-> reconciliation candidates
```

For PDF input, native text extraction is always attempted because it is local and cheap. OCR runs only for scanned or broken pages, weak numeric/layout coverage, or evidence-grounding gaps. CSV and other structured exports use deterministic table extraction before AI normalization. A multimodal normalizer may also receive the original PDF or page images.

## User-visible normalization modes

Normal settings expose one simple document-analysis configuration. Provider and extraction internals belong behind progressive disclosure.

### Smart document mode

This is the recommended default. The configured AI normalizer must support document or image input.

```text
AI input = original document/pages + native extracted text + conditional OCR observations
```

The app decides when local OCR is needed. A normal user does not choose an OCR provider or reason about extraction stages.

### Separate extraction mode

Advanced settings may configure text recognition separately from AI normalization. The normalizer may be text-only or multimodal.

The input planner selects per page or table:

```text
reliable native text -> native observations
scanned/broken text layer -> OCR observations
native/OCR disagreement -> both sources plus an explicit conflict
remaining ambiguity with a multimodal model -> include original page evidence
```

Native and OCR confidence values are not directly interchangeable. Selection also considers text coverage, numeric-token integrity, row/column continuity, native/OCR agreement, provider anchors, and whether statement totals can close.

## Provider parser skills

Each supported provider/document type is a versioned product parser package and document-agent skill with the same contract. DBS bank statements and DBS credit-card statements are separate document-type configurations under the DBS provider.

These product skills belong in the parser package. They are not repo-development skills and must never be loaded from `.agents/skills/` by the product runtime.

Each package defines:

```text
provider key and supported document type/MIME types
classifier prompt/config and positive/negative examples
deterministic provider/document fingerprints
multimodal and extraction-only normalization instructions
allowed document-agent tools
canonical output schema mapping
account/container detection rules
deterministic financial validators
supported capabilities and record types
parser, skill, prompt, schema, tool-contract, and validator versions
fixture/eval references and qualification profiles
human-readable supported layouts and known limitations
```

Prompt templates and provider rules are versioned artifacts, not scattered inline strings. A parse run loads only the canonical normalization instructions, the selected provider/document skill, and the relevant input profile.

Classification flow:

```text
Money Source/provider hint narrows candidates
AI classifier identifies provider and document type
selected provider package verifies deterministic fingerprints and schema
mismatch or uncertainty creates review work instead of using the parser blindly
```

For example, the DBS parser runs only after classification identifies a supported DBS statement type. AI classification is necessary but not sufficient for auto-commit eligibility.

## Source observations and extraction bundle

Extraction tools emit immutable, job-scoped observations rather than records.

```ts
export interface SourceObservation {
  id: string;
  kind: 'native_text' | 'ocr_text' | 'table_cell' | 'document_region';
  page?: number;
  row?: number;
  column?: number;
  text: string;
  textSpan?: { start: number; end: number };
  boundingBox?: { x: number; y: number; width: number; height: number };
  engine: string;
  engineVersion: string;
  confidence?: number;
}

export interface ExtractionBundle {
  sourceDocumentId: string;
  fileSha256: string;
  mimeType: string;
  observations: SourceObservation[];
  metadata: Record<string, unknown>;
}
```

Coordinates are normalized to one documented origin and unit. CSV evidence uses stable row/column coordinates. PDF evidence uses page plus native-text span, OCR block, or normalized page region.

These observations are job-scoped validation inputs. They are not persisted as a field-claim graph or dedicated page/row/column schema. Durable record-level source context is the bounded `raw` object defined below. This spec does not authorize permanent raw full-document text retention.

## Document-agent contract

AI normalization is mandatory and is the only component allowed to emit a canonical structured parse proposal. Native extraction, OCR, rules, and validators may supply evidence or reject a proposal, but they do not independently create canonical records.

A document agent is a bounded implementation of the AI normalizer, not a general assistant or coding agent.

Allowed tool contract:

```text
inspect_input
extract_native_text
extract_table
ocr_pages
read_page_region
validate_proposal
submit_structured_proposal
```

Rules:

```text
the runtime injects the current parse job and document scope
the model never supplies arbitrary file paths or switches to another document
tools use strict input/output schemas and bounded page/row/result sizes
there is no shell, generic file read/write, arbitrary HTTP, database, secret, or ledger tool
document content is untrusted evidence, never system instructions
free-text assistant output is not a parse result
only a schema-valid submit_structured_proposal call can complete normalization
the default budget is at most eight model steps and two proposal submissions
validation failures may be returned once for bounded repair; exhaustion creates visible review/failure state
```

The agent may choose native extraction, OCR, table extraction, or page inspection within the selected skill's allowlist. It cannot bypass schema, evidence, financial, qualification, or review gates.

## Structured parse proposal

All single-pass and agentic normalizers emit the same envelope.

```ts
export type ExactDecimalString = string;

export interface ExactMoneyInput {
  value: ExactDecimalString;
  currency: string;
}

export interface CanonicalExternalRecordInput {
  proposalRecordId: string;
  providerRecordId?: string;
  recordType: 'transaction' | 'balance' | 'position' | 'trade' | 'valuation' | 'fee' | 'interest';
  eventType?: string;
  proposalAccountId?: string;
  instrumentSymbol?: string;
  postedOn?: string;
  transactionOn?: string;
  postedAt?: string;
  descriptionRaw?: string;
  descriptionNormalized?: string;
  amount?: ExactMoneyInput;
  quantity?: ExactDecimalString;
  statementEntrySide?: 'debit' | 'credit';
  accountBalanceDelta?: ExactMoneyInput;
  balanceAfter?: ExactMoneyInput;
  valuation?: ExactMoneyInput;
  raw: Record<string, unknown>;
}

export interface StructuredDocumentIdentity {
  providerKey: string;
  documentType: string;
  statementId?: string;
  statementPeriod?: { from?: string; to?: string };
}

export interface StructuredAccountCandidate {
  proposalAccountId: string;
  accountType:
    | 'deposit_account'
    | 'credit_card'
    | 'currency_balance'
    | 'brokerage_account'
    | 'cash_balance'
    | 'position_group'
    | 'insurance_policy'
    | 'manual_asset'
    | 'manual_liability';
  providerAccountId?: string;
  maskedIdentifier?: string;
  currency?: string;
}

export interface StructuredParseProposal {
  document: StructuredDocumentIdentity;
  accounts: StructuredAccountCandidate[];
  openingSnapshots: CanonicalExternalRecordInput[];
  records: CanonicalExternalRecordInput[];
  closingSnapshots: CanonicalExternalRecordInput[];
}
```

`proposalRecordId` and `proposalAccountId` are unique only within one proposal and let records refer to detected accounts without array-position coupling. They are validated for uniqueness and are not stable database identity inputs. `providerRecordId`, when grounded, is the preferred stable external-record identity described below.

`raw` is the bounded original row/object used to normalize that record. It preserves the source values needed to understand the result, not the complete extracted document. Provider-specific page, table, row, column, region, or other locator data may be included inside this JSON object when available, but no locator shape is required and no persistent field-to-location graph is created.

`ExactDecimalString` uses a canonical plain-decimal representation with no exponent and is validated before exact decimal/native-unit arithmetic. `amount.value` is a non-negative magnitude. `accountBalanceDelta.value` is signed: positive means the source account's reported balance increases, including an increase in a credit-card liability.

`statementEntrySide` preserves an evidenced Debit/Credit label from the source. Debit/Credit is not an alias for increase/decrease: a debit normally decreases a bank balance but increases a credit-card balance. Provider skills define that mapping explicitly. When the mapping cannot be proven, `accountBalanceDelta` remains unresolved and the record requires review. Observation records such as balance snapshots do not invent a `none` posting direction.

UI plus/minus signs are derived from the view's meaning. They are not permanent parser facts.

## Date, locale, and inference rules

Statement dates remain date-only unless the source contains enough information for a real timestamp.

```text
date only -> postedOn or transactionOn as YYYY-MM-DD
explicit timestamp with offset/timezone -> postedAt as an offset-bearing ISO timestamp
time without a justified timezone -> preserve raw evidence and leave absolute time unresolved
```

Parsing precedence is:

```text
provider/document rule
explicit Money Source setting
explicit user setting
otherwise unresolved review
```

The operating-system locale is never silently used to reinterpret financial evidence.

Do not add a generic `assumptions` JSON field. If a future repeated use case genuinely requires inference, it must introduce a typed field-level contract and review policy rather than an unbounded assumption list.

## Evidence grounding

Every event-type-required financial field must be grounded to the record's raw source object before auto-commit eligibility, and that raw object must first be validated against the current job's source observations.

Rules:

```text
proposal record IDs, proposal account IDs, and observation IDs are unique in their own scope
every record contains one bounded raw JSON object copied from the current document's extracted row/region
the validator proves that the raw object corresponds as a whole to one coherent deterministic table row or one bounded source record/region returned by a current-job tool
individual values occurring somewhere in the document is insufficient; values from different rows/regions must not be spliced into one raw object
required amount/date/currency/balance fields must reproduce or deterministically map from values in that raw object
provider-defined transformations and validation outcomes are summarized in external_records.validation_json
native/OCR disagreement remains a validation conflict
multimodal-only output that cannot be grounded may enter Review but cannot auto-commit
optional page/row/region data inside raw JSON is a display hint, not grounding authority or a required query dimension
every committed event remains traceable to source document, parse run, record version, raw source object, and validation summary
```

Grounding happens during parsing; the database does not persist an evidence-claim graph. A missing, fabricated, oversized, or ungrounded raw object rejects auto-commit eligibility.

## Identity and reparse behavior

Exact document deduplication uses SHA-256 over the imported source bytes. Do not use MD5.

The trusted host may encrypt and register a selected file before provider/document classification. At that point `semantic_document_key` remains null: the renderer and user never supply it. Exact-hash duplicate and restore outcomes are available immediately. The trusted classification/normalization path sets semantic identity after grounding the provider statement ID or accepted fallback inputs; only then can a byte-different file receive a probable-existing-statement outcome.

A semantic document fingerprint detects probable duplicates whose PDF metadata or encoding changed. Prefer a provider statement ID; otherwise combine Money Source/account identity, statement period, and a normalized record-set fingerprint. A semantic match with different source bytes is review evidence, not permission to discard either file automatically.

Cross-channel import behavior:

```text
same SHA-256 -> reuse the existing source document and do not create duplicate records
same SHA-256 after current file deletion/loss -> reuse the source-document tombstone and restore the encrypted current file
same semantic document identity with different bytes -> retain the additional file evidence under the same statement identity
same stable external-record keys -> reuse/version records rather than duplicate them
semantic conflict or changed financial content -> retain both files and create review work
```

Each exact byte sequence has one `source_documents` row, which also owns its encrypted-file state and tombstone. Byte-different files under one semantic statement identity remain separate source-document rows.

The import flow returns per-file outcomes as they become grounded: file capture distinguishes newly imported, already-present, restored, and failed files; trusted classification may add the probable-existing-statement outcome for byte-different evidence.

Stable external-record identity uses a provider record ID when available. Otherwise it is derived deterministically from semantic document identity and a provider-owned canonical identity projection of the validated raw record. That projection contains only stable source values: it excludes optional locators, OCR/model confidence, observation IDs, extraction/runtime metadata, and mutable normalized descriptions. If the source contains literally identical projected rows, an occurrence ordinal within that identical-row group distinguishes them.

Reparse rules:

```text
an identical request for the same document and complete normalization profile is idempotent
a changed profile creates a new parse run and external-record version
new uncommitted records supersede prior uncommitted versions
committed ledger events are never rewritten automatically by reparse
changed output that conflicts with committed facts creates review work
deleting the current source file makes its uncommitted records ineligible for future automatic commit but does not delete record history or alter committed events
```

## Normalization profiles and qualification

Qualification attaches to the complete behavior-changing profile, not only a parser name:

```text
provider + document type
+ parser/skill/prompt/schema/validator versions
+ normalizer runtime and tool-contract versions
+ input strategy and native-extraction/OCR versions
+ AI provider/model version
```

Changing any behavior-relevant component creates a new profile. A new profile does not inherit auto-commit qualification; it returns to fixture evaluation and shadow mode.

## Normalizer runtime and execution environment

The full Pi coding agent is excluded because its coding/session/resource behavior and default general-purpose tools are outside this product's scope.

The 2026-07-14 bounded evidence spike compared:

| Candidate | Evidence result | Selection |
| --- | --- | --- |
| Single-pass structured normalization | Accepted the shared six-record proposal in one model step through the production proposal validator | Selected for initial production implementation |
| Vercel AI SDK `ToolLoopAgent` | Accepted the same proposal in five model steps and enforced the fixed tools, cancellation, and budgets | Not selected; no accuracy or recovery advantage in current evidence |
| `@earendil-works/pi-agent-core` | Accepted the same proposal in four model steps and enforced the fixed tools, cancellation, and budgets | Not selected; no accuracy or recovery advantage in current evidence |

The selected execution boundary is a trusted, bundled Node worker/sidecar controlled by Tauri, not a user-installed runtime. The Tauri host owns source-file selection and OS-secret retrieval; the worker receives only the current parse job, runtime configuration, and fixed parser capability surface. No Docker, VM, QEMU, or separate sandbox installation is required for users.

An OS sandbox is not an MVP requirement because the model has no arbitrary execution, filesystem, or network tool. The Tauri sidecar provides packaging and crash/process separation but is not itself a permission sandbox.

The disposable [runtime spike](../../spikes/document-normalizer-runtime/EVIDENCE.md) proves dependency bundling, deterministic mock execution, startup, cancellation, crash isolation, inherited-environment clearing, protocol secret-field rejection, ad-hoc signed Tauri packaging, debug size, and a conservative comparison-workspace license inventory on macOS `arm64`. Phase 1 still requires the equivalent architecture-matched sidecar evidence on macOS `x86_64`. Exact production credential delivery/redaction, an artifact-specific SBOM/license inventory, Developer ID signing, and notarization remain owning-slice gates rather than runtime-selection evidence. ADR 0003 records the accepted package/runtime boundary.

Primary references:

- [Vercel AI SDK ToolLoopAgent](https://ai-sdk.dev/docs/agents/building-agents)
- [Pi Agent Core](https://github.com/earendil-works/pi/tree/main/packages/agent)
- [Pi containerization and permission boundary](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/containerization.md)
- [Tauri external binaries / sidecars](https://v2.tauri.app/develop/sidecar/)
- [Node.js single executable applications](https://nodejs.org/docs/latest-v24.x/api/single-executable-applications.html)

## Parse-run metadata

Every run records:

```text
strict schema/output version
parser skill and normalization profile ID
prompt template version and prompt hash
input and output hashes
normalizer runtime, tool-contract, model, and provider versions
native extraction and OCR engine versions actually used
model steps, tool calls, validation attempts, latency, token usage, and cost estimate when available
final schema/evidence/financial validation outcome
```

Do not copy secrets, PDF passwords, or unnecessary raw sensitive text into parse metadata or logs.

## Validation gates

Schema validation checks shape and canonical decimal/date forms.

Evidence grounding checks required fields against source observations.

Deterministic financial validation checks:

```text
date validity
amount/quantity precision
currency/instrument validity
statement period boundaries
explicit Debit/Credit-to-account-delta mapping
balance math and exact opening-to-closing reconciliation when available
row count and statement-total consistency
duplicate row hash
account mapping status
impossible signs or values
```

Raw model confidence never grants eligibility. Package-calibrated field outcomes, grounded evidence, deterministic validators, and the review/commit policy decide eligibility.

## Fixture and harness policy

Parser fixtures include, when approved:

```text
synthetic/redacted source file or private local source reference
source-observation/extraction fixture
mocked agent transcript or stored structured model output
expected structured parse proposal
expected raw-record grounding and financial-validation results
expected review items and eligibility outcome
```

Any future agentic candidate must be tested against the selected single-pass structured-normalization baseline. It enters production only if the qualification suite shows a material accuracy or recovery advantage that justifies its extra calls, latency, cost, and dependency surface.

## Acceptance criteria

- Native extraction and OCR produce source observations, never canonical financial records.
- A mandatory AI normalizer is the sole producer of structured parse proposals.
- Single-pass and bounded-agent implementations share one proposal schema and every downstream gate.
- Smart document mode stays simple; separate OCR/normalizer configuration is advanced-only.
- Product parser skills are versioned provider packages and never load repo `.agents/skills/`.
- The document agent exposes only the seven fixed job-scoped parser tools and has no shell, generic file, arbitrary network, database, secret, or ledger capability.
- Free text cannot complete a parse; completion requires a schema-valid structured proposal within the accepted step/submission budget.
- Every required financial field is grounded to a bounded raw source object that was validated against current-job observations before auto-commit eligibility.
- Dates remain date-only unless a justified timezone exists; the OS locale and generic assumption JSON are not used as silent inference mechanisms.
- Debit/Credit source labels, signed source-account balance deltas, and UI plus/minus presentation remain separate concepts.
- Exact PDF/file deduplication uses SHA-256, with semantic document identity handled separately.
- Exact-hash re-import may restore a deleted current file without duplicating its source-document row; byte-different evidence remains separate under semantic identity.
- Re-import, reparse, and source-file deletion preserve record identity without rewriting committed ledger events; deleted evidence cannot drive future automatic commit.
- Complete normalization profiles qualify independently and reset to shadow mode after behavior-changing updates.
- Single-pass structured normalization is the initial runtime; ToolLoopAgent or Pi Agent Core requires new qualification evidence showing a material accuracy or recovery advantage.
- No sandbox installation is required for users; any process isolation is evidence-driven rather than permission theater.
