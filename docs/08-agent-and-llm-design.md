# 08. Agent and LLM Design

## Core principle

The AI agent is an advisor and parser assistant. It is not the ledger owner.

```text
LLM may propose.
Rules validate.
User or deterministic policy commits.
```

## Recommended role of LLM

LLM can help with:

```text
document classification
schema-based extraction from messy text/OCR
merchant/counterparty normalization
ambiguous account mapping suggestions
match candidate explanation
fuzzy description understanding
parser template generation
review summaries
```

LLM should not:

```text
write committed ledger events directly
delete records
ignore records permanently
read secrets
access full filesystem
send emails
make payments
place trades
withdraw crypto
bypass MFA/CAPTCHA
```

## Main engine language

Use TypeScript for the main domain engine.

Reasons:

```text
matches React stack
AI coding tools generate/review TS well
types can be shared between UI and core
future mobile React Native can reuse domain code
simpler than maintaining Python as primary ledger engine
```

## Privileged layer

Use Tauri/Rust for:

```text
file access
database opening and backup
secret storage
encryption helpers
native dialogs
sidecar execution
```

Rust should not contain most business logic unless performance/security requires it.

## Python role

Python is optional and should be a sidecar worker only.

Good uses:

```text
OCR experiments
PDF/table extraction
local ML tools
batch parser prototyping
```

Bad uses:

```text
primary ledger engine
primary reconciliation engine
secret owner
direct database mutator without validation layer
```

## Agent framework decision

MVP does not need a full autonomous agent framework.

Start with:

```text
TypeScript LLM provider adapter
structured JSON extraction
schema validation
prompt/version logging
deterministic parser pipeline
```

Candidate libraries later:

```text
Vercel AI SDK
OpenAI Agents SDK TypeScript
Mastra
LangGraph, mainly if Python sidecar workflows become useful
Pydantic AI, only for Python extraction workers
```

## Internal tool registry

Build an internal MCP-like tool registry with narrow tools.

Allowed tools:

```text
read_document_text(document_id)
read_document_page_image(document_id, page)
get_candidate_accounts(provider)
get_existing_records(account_id, date_range)
propose_parsed_records(parse_run_id, records)
propose_match_edges(candidate_edges)
```

Disallowed tools:

```text
read_secret(secret_name)
commit_ledger(event)
delete_record(id)
send_email()
make_payment()
place_trade()
withdraw_crypto()
```

## Structured extraction contract

LLM output must be strict JSON.

Example parser prompt output:

```json
{
  "document_type": "bank_statement",
  "provider": "DBS",
  "account_hint": "1234",
  "records": [
    {
      "record_type": "transaction",
      "posted_at": "2026-06-12",
      "description_raw": "WISE TOP UP",
      "amount": -1000,
      "currency": "SGD",
      "confidence": 0.94,
      "source_reference": "page 2 row 14"
    }
  ]
}
```

Then deterministic code validates:

```text
schema
dates
amount signs
currency
period boundaries
row counts
balance math
possible duplicates
```

## Prompt/version logging

Every AI call should record:

```text
provider
model
prompt template version
prompt hash
input hash
output hash
token/cost estimate
schema validation result
created_at
```

## Agent safety levels

### Level 0: no AI

Deterministic parser only.

### Level 1: AI parse proposal

LLM extracts structured records, but cannot commit.

### Level 2: AI match explanation

LLM explains and reranks candidate matches.

### Level 3: AI parser repair

LLM helps repair failed parses or generate parser configs.

### Level 4: autonomous run planner

Not MVP. Only considered after reliable audit and permission model.
