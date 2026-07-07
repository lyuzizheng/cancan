# 03. AI Parser Pipeline

## Goal

The AI parser turns messy financial evidence into validated, staged records that can be reviewed and committed to the ledger.

It must be auditable, resumable, and safe. The LLM should never directly mutate the committed ledger.

## Pipeline overview

```text
Source Discovery
-> Raw Document Ingestion
-> Fingerprint / Dedupe
-> Document Classification
-> Text Extraction
-> OCR if needed
-> Layout / Table Segmentation
-> Parser Selection
-> Structured Extraction
-> Schema Validation
-> Deterministic Validation
-> Account Mapping
-> Canonical Record Staging
-> Dedupe
-> Reconciliation Candidate Generation
-> AI Rerank / Explanation
-> Review Inbox
-> Commit to Ledger
```

## App start behavior

### First run

```text
Create Vault
-> configure encryption
-> configure AI provider keys
-> enable plugins
-> create/import accounts
-> choose first scan/import scope
-> run initial parser jobs
```

### Subsequent run

```text
Unlock Vault
-> check unfinished jobs
-> resume failed/paused parse runs
-> traverse enabled plugins
-> plan incremental scan
-> show Continue Parsing / Run Sync
```

## Source discovery

Sources can come from:

```text
API plugins
- Wise
- Moomoo
- Bitget
- future bank APIs

Gmail plugins
- finance emails
- attachments
- statement notifications

Manual import
- PDF
- CSV
- XLSX
- screenshot/image

Watched folder
- user-selected local folders
```

## Raw ingestion

Every raw document or API response must be stored before parsing.

Persist:

```text
file hash
source metadata
email metadata
API request/response metadata
statement period if detected
original file path or vault path
created source_document row
```

## Fingerprint and document dedupe

Before parsing, compute:

```text
sha256(raw bytes)
normalized file name
provider hint
statement date range
email message id / attachment id if available
```

If a document already exists, skip or attach it as a duplicate source reference.

## Document classification

Classify into:

```text
bank deposit statement
credit card statement
Wise statement
Revolut statement
broker statement
crypto exchange statement
insurance policy statement
unknown financial document
non-financial document
```

Classification should use deterministic hints first:

```text
sender domain
file name
PDF text header
known provider keywords
statement period patterns
```

Use LLM classification only if deterministic hints are insufficient.

## Text extraction

For PDFs:

```text
1. Try native PDF text layer extraction.
2. Measure text density and table quality.
3. If low quality, perform OCR on affected pages.
4. Store text layer and OCR output separately.
```

For screenshots/images:

```text
1. OCR directly.
2. Store page/image-level OCR text.
3. Store bounding boxes if available.
```

Do not throw away raw evidence.

## OCR strategy

Recommended order:

```text
local OCR first, if quality is acceptable
cloud/model OCR only when local extraction fails or user explicitly allows
```

Sensitive documents should not be sent to external AI providers unless the user has configured and enabled that behavior.

## Layout and table segmentation

Parse documents by sections:

```text
account summary
transaction table
fees/interest section
payment summary
positions table
trades table
balance/valuation snapshot
```

The output of this stage is a set of candidate sections and rows.

## Parser selection

Parser selection should be provider-aware.

Examples:

```text
DBS credit card statement parser
UOB deposit statement parser
Wise balance statement parser
Moomoo daily statement parser
Bitget transaction history parser
Manulife policy statement parser
Generic bank parser fallback
```

Preferred order:

```text
provider-specific deterministic parser
provider-specific AI-assisted parser
generic table parser
generic document agent parser
manual mapping UI
```

## Structured extraction

The AI parser should output strict JSON only.

Example target shape:

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
      "posted_at": "2026-06-12",
      "description_raw": "WISE * TOP UP",
      "amount": -1000,
      "currency": "SGD",
      "direction": "outflow",
      "row_reference": "page 3 row 12",
      "confidence": 0.93
    }
  ]
}
```

All output must pass schema validation.

## Deterministic validation

After AI extraction, validate with code:

```text
date format
amount format
currency consistency
statement period boundaries
opening balance + movements = closing balance, if possible
transaction total matches summary total, if available
row count consistency
no duplicate row hashes
no impossible signs or values
```

If validation fails, create review items instead of committing.

## Account mapping

Map parsed records to known accounts using:

```text
provider
account last4 / masked number
currency
statement type
email sender
existing account aliases
user confirmation history
```

If ambiguous, stage records under an unresolved account and ask the user.

## Staging before commit

Parsed records become `external_records` first.

Only after validation and review do they become committed `ledger_events` and `ledger_legs`.

## Parser versioning

Every parse result must record:

```text
parser name
parser version
model provider
model name
prompt hash
input hash
output hash
validation result
```

This enables safe reparse:

```text
Reparse all DBS PDFs parsed by parser <= v3
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
```

Each failure creates a review or repair task.
