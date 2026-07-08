# 0002. Database Schema Spec

## Goal

Design SQLite/SQLCipher schema for a local-first finance vault that supports evidence, parsing, review, ledger, assets, positions, snapshots, and reconciliation.

## Database policy

- Use hand-written SQL migrations.
- Do not use Prisma.
- Avoid clever, huge SQL queries that are hard to reason about.
- Prefer small indexed queries and TypeScript composition when it improves clarity.
- Use JSON fields for provider-specific metadata and evolving shapes to avoid premature table explosion.
- Add indexes deliberately and benchmark important queries.
- Every migration should be deterministic and reversible when practical.

## Core tables

Initial schema areas:

```text
vault_settings
money_sources
accounts
instruments
source_documents
file_objects
gmail_search_rules
gmail_sync_states
parse_runs
external_records
ledger_events
ledger_legs
match_edges
review_items
jobs
audit_log
```

## Index policy

Every table must have indexes for the access paths used by UI/services.

Required examples:

```text
source_documents(file_hash)
source_documents(money_source_id, received_at)
gmail_search_rules(enabled)
parse_runs(source_document_id, created_at)
external_records(source_document_id, row_hash)
external_records(status, record_type)
ledger_events(event_date)
ledger_events(status, event_type)
ledger_legs(account_id, instrument_id)
match_edges(review_status, match_type)
review_items(status, priority, created_at)
jobs(status, lease_until)
```

## JSON field policy

Use JSON for:

```text
provider metadata
raw parse metadata
validation details
AI explanation payloads
sync cursor state
source-specific account hints
```

Do not hide core query dimensions in JSON. If the UI filters by it often, make it a column.

## Benchmark policy

Before broad data import features are considered complete, create lightweight benchmark fixtures for:

```text
10k source documents
100k external records
100k ledger events
300k ledger legs
50k match edges
```

Benchmarks should cover:

```text
Command Center summary
Source detail page
Review inbox
Asset summary
Transaction/event list
Money flow chain lookup
```

## Test reset policy

Provide a deterministic test reset path:

```text
drop/recreate test database
run migrations
seed minimal sources/accounts
load fixtures
run parser/reconciliation tests
```

Acceptance requires integration tests to run from a clean database without manual setup.
