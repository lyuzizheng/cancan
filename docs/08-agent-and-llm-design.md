# 08. Agent and LLM Design

## Core principle

The AI agent is a parser, normalizer, reviewer assistant, and explanation layer. It is not the final ledger owner.

```text
LLM may propose.
Rules validate.
Policy or user commits.
```

## Recommended role of LLM

LLM can help with:

```text
document classification
schema-based extraction from native text and OCR bundles
merchant/counterparty normalization
record/event type classification
ambiguous account mapping suggestions
duplicate candidate explanation
cross-source link candidate explanation
fuzzy description understanding
parser template generation
review summaries
```

LLM should not:

```text
read secrets
access full filesystem
write committed ledger events directly
delete records
ignore records permanently
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
types can be shared between UI and core
AI coding tools generate/review TS well
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
LangGraph only if Python sidecar workflows become useful
Pydantic AI only for Python extraction workers
```

## Internal tool registry

Build narrow internal tools.

Allowed tools:

```text
read_document_text(document_id)
read_document_page_image(document_id, page)
read_ocr_output(document_id)
get_candidate_accounts(provider)
get_existing_records(account_id, date_range)
propose_parsed_records(parse_run_id, records)
propose_match_edges(candidate_edges)
explain_review_item(review_item_id)
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

## Safety levels

### Level 0: no AI

Deterministic parser only.

### Level 1: AI parse proposal

LLM extracts structured records, but cannot commit.

### Level 2: AI normalization and match explanation

LLM normalizes records and explains/reranks candidate matches.

### Level 3: AI parser repair

LLM helps repair failed parses or generate parser configs.

### Level 4: autonomous run planner

Not MVP. Only considered after reliable audit and permission model.
