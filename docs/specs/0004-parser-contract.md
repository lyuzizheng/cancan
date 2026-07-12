# 0004. Parser Contract Spec

## Goal

Define how source evidence becomes staged records through extraction, OCR, AI normalization, schema validation, and deterministic financial validation.

## Implementation blocker

OCR/native-text selection, evidence locations, locale/timezone/sign rules, and raw extraction retention/deletion remain open in the [active alignment register](../alignment-temp/alignment-progress.md). Do not freeze those affected fields or sensitive-data lifecycle rules by inference.

## Pipeline

```text
source_document
-> native_text_extraction
-> OCR extraction
-> extraction_bundle
-> provider classifier
-> provider parser
-> AI normalization
-> schema validation
-> deterministic validation
-> external_records
-> reconciliation candidates
```

## Extraction bundle

```ts
export interface ExtractionBundle {
  sourceDocumentId: string;
  fileSha256: string;
  mimeType: string;
  nativeText?: {
    text: string;
    pageTexts: Array<{ page: number; text: string }>;
    quality: ExtractionQuality;
  };
  ocr?: {
    pages: Array<{
      page: number;
      text: string;
      confidence?: number;
      blocks?: unknown[];
    }>;
  };
  tables?: unknown[];
  metadata: Record<string, unknown>;
}
```

## Identity and reparse behavior

Exact document deduplication uses SHA-256 over the imported source bytes. Do not use MD5.

A semantic document fingerprint detects probable duplicates whose PDF metadata or encoding changed. Prefer a provider statement ID; otherwise combine Money Source/account identity, statement period, and a normalized record-set fingerprint. A semantic match with different source bytes is review evidence, not permission to discard either file automatically.

Stable external-record identity uses a provider record ID when available. Otherwise it is derived deterministically from semantic document identity, source row coordinates, and normalized stable financial fields. Confidence, parser version, prompt version, and mutable descriptions are not identity inputs.

Reparse rules:

```text
an identical request for the same document and parser/input versions is idempotent
a changed parser or prompt creates a new parse run and external-record version
new uncommitted records supersede prior uncommitted versions
committed ledger events are never rewritten automatically by reparse
changed output that conflicts with committed facts creates review work
```

## Parser result layers

Provider-specific intermediate output is allowed, but all parsers must normalize to canonical external records.

```ts
export interface CanonicalExternalRecordInput {
  recordType: 'transaction' | 'balance' | 'position' | 'trade' | 'valuation' | 'fee' | 'interest';
  eventType?: string;
  accountHint?: string;
  instrumentSymbol?: string;
  postedAt?: string;
  transactionAt?: string;
  descriptionRaw?: string;
  descriptionNormalized?: string;
  amount?: string;
  quantity?: string;
  currency?: string;
  direction?: 'inflow' | 'outflow' | 'debit' | 'credit' | 'none';
  balanceAfter?: string;
  valuationAmount?: string;
  valuationCurrency?: string;
  rowReference?: string;
  confidence: number;
  raw: unknown;
}
```

## AI provider

Vercel AI SDK may be used for structured generation/provider routing.

Requirements:

```text
strict JSON output
schema validation
prompt template version
prompt hash
input hash
output hash
model/provider metadata
cost/token estimate when available
```

## Validation gates

Schema validation checks shape.

Deterministic validation checks:

```text
date validity
amount/quantity precision
currency/instrument validity
statement period boundaries
balance math when available
row count consistency
duplicate row hash
account mapping status
impossible signs or values
```

## Fixture policy

When approved, parser fixtures should include:

```text
redacted source file or extracted text fixture
extraction bundle fixture
expected canonical external records
expected validation result
expected review items if any
```

## Acceptance criteria

- Parser output never bypasses schema validation.
- A failed parse creates a visible review/repair item.
- Exact PDF/file deduplication uses SHA-256, with semantic document identity handled separately.
- Reparse versions supersede proposals without rewriting committed ledger events.
- Provider parsers can evolve without changing ledger schema.
