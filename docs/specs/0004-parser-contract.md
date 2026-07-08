# 0004. Parser Contract Spec

## Goal

Define how source evidence becomes staged records through extraction, OCR, AI normalization, schema validation, and deterministic financial validation.

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
  fileHash: string;
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
- AI output cannot directly commit ledger events.
- A failed parse creates a visible review/repair item.
- Provider parsers can evolve without changing ledger schema.
