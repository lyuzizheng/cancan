# 0002. Database Schema Spec

## Goal

Design SQLite/SQLCipher schema for a local-first finance vault that supports evidence, parsing, review, ledger, assets, positions, snapshots, and reconciliation.

## Implementation blocker

SQLCipher plus FTS5 feasibility is verified by the [desktop spike](../../spikes/desktop-feasibility/EVIDENCE.md). Exact vault keys, encrypted-file format, temporary plaintext, recovery compatibility, and restore behavior remain open in the [active alignment register](../alignment-temp/alignment-progress.md); do not freeze those security fields by inference.

## Database policy

- Use hand-written SQL migrations.
- Do not use Prisma.
- Avoid clever, huge SQL queries that are hard to reason about.
- Prefer small indexed queries and TypeScript composition when it improves clarity.
- Use JSON fields for provider-specific metadata and evolving shapes to avoid premature table explosion.
- Add an index only for an implemented query, uniqueness rule, or idempotency rule.
- Every migration should be deterministic and reversible when practical.

## Slice-owned schema

The database grows with the implementation slices. Do not create the complete future schema in the first migration.

`synthetic-core-flow` creates only the tables needed to stage, validate, link, commit, and query synthetic records:

```text
money_sources
accounts
instruments
source_documents
parse_runs
external_records
ledger_events
ledger_legs
match_edges
review_items
audit_log
```

Later slices add their own tables only when their behavior is implemented. Examples include vault/file storage, durable jobs, Gmail rules/sync state, and other connector state. A migration must name its owning slice and have an integration test that starts from a clean test database.

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

`match_edges` is part of the synthetic core, not a deferred extension. A transfer can be represented as two source records linked through one canonical transfer event:

```text
external record from account A -> canonical transfer event <- external record from account B
```

The implemented query paths must support opening a source record and finding its canonical event plus sibling source records, and opening a canonical event and finding every linked source record.

## Account identity schema projection

`0014-money-overview-source-taxonomy.md` owns account identity and lifecycle behavior. The schema must support:

```text
candidate, confirmed, archived, and merged account states
one stable provider account ID when the provider supplies it
one user-safe masked identifier for display when available
first-seen candidate confirmation when no stable provider ID exists
merged_into_account_id without rewriting committed ledger legs
idempotent account resolution and audited merge/archive transitions
```

Do not add a separate identifier table, keyed digests, strength levels, or key-version machinery until a supported provider demonstrates that the simpler account identity cannot represent its real inputs safely.

## Document and record identity

`0004-parser-contract.md` owns document/record identity and reparse behavior. The schema keeps its SHA-256 file identity, semantic document identity, stable external-record key, and record version as separate fields.

Each external record stores the bounded original source row/object used for normalization and a validation summary:

```text
parse_runs store the complete normalization-profile and runtime/tool/model versions
external_records.raw_json stores one bounded source record, not full-document extraction
external_records.validation_json stores grounding and deterministic-validation outcomes
page, row, column, region, or provider location data may remain optional values inside raw_json
no separate evidence-reference table or location columns are required
date-only fields remain date-only rather than receiving an invented timezone
no generic assumptions_json column is added for speculative inference
```

The parser validates the raw record against job-scoped native/OCR/table observations before persistence. The raw record and validation summary are the durable audit context; optional location values are UI hints, not independently queried financial facts. Full-document extraction retention and deletion remain governed by the sensitive-data-lifecycle blocker.

## Index policy

Create indexes only when they enforce an accepted invariant or serve a real repository query. The initial required access paths are:

```text
unique source_documents(file_sha256)
unique external_records(stable_record_key, version)
unique ledger_events(commit_idempotency_key)
an access path beginning with match_edges.external_record_id for record -> event/sibling navigation
an access path beginning with match_edges.ledger_event_id for event -> source-record navigation
```

The match-edge primary/unique key may satisfy one direction; do not create a redundant index for it. Account-provider identity receives a partial unique constraint only when a stable provider account ID is present on a non-merged account. Status, date, list, and read-model indexes are added with the repository method that needs them and verified with its query plan or focused benchmark. Do not index optional values inside `raw_json` unless a later accepted feature introduces a real query for them.

## JSON field policy

Use JSON for bounded data that is displayed or validated as a whole:

```text
provider metadata
one raw source record/object per external record
validation details
AI explanation payloads
sync cursor state
source-specific account hints
```

Do not hide core query dimensions in JSON. If the UI filters by it often, make it a column.

## Benchmark policy

Benchmark implemented hot paths rather than a hypothetical complete schema. Each benchmark records the repository query, representative synthetic dataset, query plan, and threshold beside the feature that owns it. Start with record/event/sibling transfer navigation and add Command Center, Source detail, Review, asset, and event-list benchmarks only when those queries exist.

Do not freeze arbitrary global fixture cardinalities in this spec. Dataset size should be large enough to expose a regression in the owning query and may grow when real usage evidence justifies it.

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
- Every external record retains one bounded validated raw source record plus a validation summary, without a separate field-evidence graph or permanent raw full-document text.
- Account identity supports a stable provider account ID when available, first-seen candidates, archive, and merge redirects without using display names or masked suffixes as automatic identity.
- The schema supports immutable committed events, reversals, commit idempotency, many-to-many allocations, and atomic audit records.
- Two source records can link through one canonical transfer event and are navigable in both directions.
- Indexes trace to an implemented query, uniqueness rule, or idempotency rule.
- Implemented hot query paths have focused benchmark/query-plan coverage with representative synthetic data.
- Integration tests can reset and migrate only a test database without manual setup.
