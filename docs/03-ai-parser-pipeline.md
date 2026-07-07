# 03. AI Parser Pipeline

## Goal

The parser turns messy financial evidence into validated, staged records that can be reviewed and committed to the ledger.

AI-assisted parsing is core to CanCan. The LLM helps identify, normalize, deduplicate, link, and explain records. It still cannot bypass deterministic validation or review policy.

## Pipeline overview

```text
Source Discovery
-> Raw Document Ingestion
-> Fingerprint / Document Dedupe
-> Document Classification
-> Native PDF/Text Extraction
-> OCR Layer
-> Extraction Bundle Assembly
-> Parser Selection
-> AI-Assisted Structured Normalization
-> Schema Validation
-> Deterministic Financial Validation
-> Account Mapping
-> External Record Staging
-> Duplicate Candidate Generation
-> Link Candidate Generation
-> AI Rerank / Explanation
-> Review Inbox or Auto-Policy
-> Ledger Commit
```

## Extraction strategy

For PDFs, do both when practical:

```text
1. Extract native PDF text layer.
2. Run OCR layer to capture layout, tables, screenshots, or broken text extraction.
3. Keep native text and OCR output separately.
4. Assemble both into an extraction bundle for parser/LLM use.
```

This avoids losing signal. Native text is often cleaner for bank PDFs; OCR may recover visual table structure or scanned pages.

For images/screenshots:

```text
1. OCR directly.
2. Store page/image-level OCR text.
3. Store bounding boxes when available and useful.
```

## AI normalization role

The LLM is responsible for high-value ambiguous interpretation:

- document type recognition when deterministic hints are insufficient;
- row/transaction recognition from messy extracted text;
- merchant/counterparty normalization;
- account/sub-account mapping suggestions;
- duplicate candidate explanation;
- cross-source link candidate explanation;
- classifying whether a record is purchase, repayment, transfer, top-up, FX conversion, trade, fee, interest, balance snapshot, or valuation snapshot.

The LLM returns strict JSON only. It does not write the committed ledger directly.

## Structured output contract

Example:

```json
{
  "document_type": "credit_card_statement",
  "provider": "DBS",
  "account_hint": "1234",
  "statement_period": {
    "start": "2026-06-01",
    "end": "2026-06-30"
  },
  "records": [
    {
      "record_type": "transaction",
      "event_type": "purchase",
      "posted_at": "2026-06-12",
      "transaction_at": "2026-06-11",
      "description_raw": "WISE * TOP UP",
      "description_normalized": "Wise top up",
      "amount": -1000,
      "currency": "SGD",
      "direction": "outflow",
      "row_reference": "page 3 row 12",
      "confidence": 0.93
    }
  ]
}
```

## Deterministic validation

After AI extraction, code validates:

```text
schema validity
date format
amount format
currency consistency
statement period boundaries
opening balance + movements = closing balance, if possible
transaction total matches statement summary, if available
row count consistency
no duplicate row hashes
account mapping is known or explicitly unresolved
no impossible signs or values
```

If validation fails, create review/repair items instead of committing.

## Commit policy

Low-risk standalone purchases may auto-commit when:

```text
schema_valid = true
financial_validation_passed = true
account_mapping_confidence >= threshold
record_confidence >= threshold
not part of any possible transfer/top-up/repayment/FX/trade link
not a duplicate candidate above threshold
```

Transfers, credit card repayments, top-ups, FX conversions, broker deposits, crypto movements, and ambiguous duplicates should default to review unless an explicit auto-policy is later defined.

## Parser versioning

Every parse result records:

```text
parser name
parser version
AI provider
model
prompt template version
prompt hash
input hash
extraction bundle hash
output hash
validation result
token/cost estimate
```

This enables safe reparse:

```text
Reparse all DBS credit-card PDFs parsed by parser <= v3
Compare old/new output
Show changed records before commit
```

## Failure handling

Common failure states:

```text
password protected PDF
unsupported document type
low OCR confidence
ambiguous account
invalid totals
AI output schema failure
duplicate document
partial parse
possible cross-source link requiring review
```

Each failure creates a review or repair task.
