# 0002. Database Schema Spec

## Goal

Design SQLite/SQLCipher schema for a local-first finance vault that supports evidence, parsing, review, ledger, assets, positions, snapshots, and reconciliation.

## Implementation blocker

Vault/storage feasibility remains open in the [active alignment register](../alignment-temp/alignment-progress.md). Do not finalize affected storage constraints until that entry is resolved.

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
account_identifiers
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

## Ledger schema projection

`0013-ledger-assets-valuation.md` owns event and leg behavior. The schema must support it without duplicating that policy here:

```text
ledger_events distinguish posting from observation events
committed ledger events and legs are immutable
reversal events reference the event they reverse
replacement events may reference the reversal/correction chain
each accepted proposal version has a unique commit idempotency key
```

`0005-review-and-commit-policy.md` owns matching and audit behavior. The schema must support:

```text
many-to-many external-record/event links
explicit allocation amount and unit for partial links
review status and unmatched remainder
append-only audit entries written atomically with financial mutations
```

## Account identity schema projection

`0014-money-overview-source-taxonomy.md` owns account identity and lifecycle behavior. The schema must support:

```text
candidate, confirmed, archived, and merged account states
multiple versioned provider identifiers per account
unique versioned keyed identifier digests within a Money Source/provider/type
masked display values separated from identity digests
merged_into_account_id without rewriting committed ledger legs
idempotent account resolution and audited merge/archive transitions
```

## Document and record identity

`0004-parser-contract.md` owns document/record identity and reparse behavior. The schema keeps its SHA-256 file identity, semantic document identity, stable external-record key, and record version as separate fields.

## Index policy

Every table must have indexes for the access paths used by UI/services.

Required examples:

```text
source_documents(file_sha256)
source_documents(money_source_id, semantic_document_key)
source_documents(money_source_id, received_at)
accounts(money_source_id, status)
accounts(merged_into_account_id)
account_identifiers(money_source_id, provider_key, identifier_type, identifier_key_version, identifier_digest) WHERE verified exact/strong
account_identifiers(account_id, strength)
gmail_search_rules(enabled)
parse_runs(source_document_id, created_at)
external_records(stable_record_key, version)
external_records(status, record_type)
ledger_events(event_date)
ledger_events(status, event_type)
ledger_events(commit_idempotency_key)
ledger_legs(account_id, instrument_id)
match_edges(review_status, match_type)
match_edges(external_record_id, ledger_event_id)
review_items(status, priority, created_at)
jobs(status, lease_until)
audit_log(entity_type, entity_id, created_at)
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

## Acceptance criteria

- Schema changes use hand-written, versioned migrations.
- Core query dimensions are columns rather than hidden in JSON.
- Exact source-file identity uses SHA-256 and remains separate from semantic document identity.
- Account identity supports unique keyed provider aliases, first-seen candidates, archive, and merge redirects without using display names or bare hashes as identity.
- The schema supports immutable committed events, reversals, commit idempotency, many-to-many allocations, and atomic audit records.
- Required UI/service access paths have deliberate indexes.
- Important query paths have benchmark coverage at the documented fixture sizes.
- Integration tests can reset and migrate only a test database without manual setup.
