# Database Schema Review — Defect Report and Remediation Plan

**Reviewer**: Independent Review Agent
**Date**: 2026-07-30
**Scope**: All 9 migrations (`packages/db/migrations/`), repository layer (`packages/db/src/`), migration runner, and related specs (0002, 0005, 0013, 0014, 0015).

---

## Executive Summary

The schema design is fundamentally sound: slice-owned incremental growth, trigger-based immutability for committed financial data, conservative index policy, and JSON for bounded metadata. The issues below are primarily **drift between DB constraints and TypeScript types**, **missing indexes for already-implemented queries**, and **incomplete constraint coverage**.

| Severity | Count |
|----------|-------|
| P0 Critical | 2 |
| P1 High | 5 |
| P2 Medium | 6 |
| P3 Low | 3 |

---

## P0 — Critical

### P0-1: TypeScript types diverge from DB CHECK constraints

**Location**: `packages/db/src/repository.ts` lines 5–6

**Current code**:

```typescript
type AccountStatus = "candidate" | "confirmed" | "archived" | "merged";
type ExternalRecordStatus = "staged" | "review" | "committed" | "superseded";
```

**Actual DB constraints** (after migrations 0006 + 0009):

```sql
-- accounts (0009_post_pr41_hardening.sql)
CHECK (status IN ('candidate', 'confirmed', 'dismissed', 'archived', 'merged'))

-- external_records (0006_review_ledger.sql)
CHECK (status IN ('staged', 'review', 'committed', 'removed', 'superseded'))
```

**Impact**:

- Reading a `dismissed` account or `removed` record from DB passes through `as ExternalRecordStatus` cast silently carrying an illegal value at the type level.
- Writing code that needs to handle these states cannot construct them through the typed interface.
- `StagedRecordStatus = Extract<ExternalRecordStatus, "staged" | "review">` remains correct only by coincidence.

**Fix**:

```typescript
type AccountStatus = "candidate" | "confirmed" | "dismissed" | "archived" | "merged";
type ExternalRecordStatus = "staged" | "review" | "committed" | "removed" | "superseded";
```

**Effort**: 15 minutes
**Risk**: None. Pure type alignment.

---

### P0-2: No self-merge guard on `accounts.merged_into_account_id`

**Location**: `0001_synthetic_core.sql` line 19, `0009_post_pr41_hardening.sql` line 12

**Current state**: Self-referencing FK `REFERENCES accounts(id)` with no constraint preventing `merged_into_account_id = id`.

**Impact**: An application-layer bug could create a self-merging account row, causing infinite loops in any account-resolution or merge-chain traversal query.

**Fix**: Add triggers (avoids full table rebuild):

```sql
CREATE TRIGGER account_cannot_merge_into_self_insert
BEFORE INSERT ON accounts
WHEN NEW.merged_into_account_id IS NOT NULL AND NEW.merged_into_account_id = NEW.id
BEGIN
  SELECT RAISE(ABORT, 'account cannot merge into itself');
END;

CREATE TRIGGER account_cannot_merge_into_self_update
BEFORE UPDATE OF merged_into_account_id ON accounts
WHEN NEW.merged_into_account_id IS NOT NULL AND NEW.merged_into_account_id = NEW.id
BEGIN
  SELECT RAISE(ABORT, 'account cannot merge into itself');
END;
```

**Note**: This prevents single-node cycles. Multi-node cycles (A→B→A) require application-layer path traversal validation.

**Effort**: 30 minutes (migration + test)
**Risk**: Low. Additive trigger, no data migration.

---

## P1 — High

### P1-1: `audit_log` missing index on `(entity_type, entity_id)`

**Location**: `packages/db/src/repository.ts` `findAuditEntries()` (line 451)

**Query pattern**:

```sql
SELECT ... FROM audit_log
WHERE entity_type = ? AND entity_id = ?
ORDER BY created_at, id
```

**Impact**: `audit_log` is append-only (protected by triggers). Every commit, review decision, and deletion appends rows. Without an index, this degrades to full table scan as audit history grows.

**Fix**:

```sql
CREATE INDEX audit_log_entity_lookup
  ON audit_log(entity_type, entity_id, created_at, id);
```

**Effort**: 10 minutes
**Spec justification**: 0002 index policy states "Add an index only for an implemented query" — this query is implemented.

---

### P1-2: `review_items` missing index for open-item lookup

**Location**: `packages/db/src/repository.ts` `findOpenReviewItems()` (line 468)

**Query pattern**:

```sql
SELECT ... FROM review_items WHERE status = 'open' ORDER BY id
```

**Fix** (partial index — minimal footprint):

```sql
CREATE INDEX review_items_open ON review_items(status, id) WHERE status = 'open';
```

**Effort**: 10 minutes

---

### P1-3: `ledger_events` missing observation query index

**Location**: `packages/db/src/repository.ts` `findLatestBalanceObservation()` (line 399)

**Query pattern**:

```sql
SELECT ... FROM ledger_legs
JOIN ledger_events ON ...
WHERE ledger_events.event_class = 'observation'
  AND ledger_events.status = 'committed'
ORDER BY ledger_events.event_date DESC, ledger_events.created_at DESC
LIMIT 1
```

**Fix**:

```sql
CREATE INDEX ledger_events_observation_lookup
  ON ledger_events(event_class, status, event_date DESC);
```

**Effort**: 10 minutes

---

### P1-4: Spec-required `match_edges.role` column missing

**Spec references**:

- `0002-database-schema.md` line 63: "an explicit match-edge role separating financial allocation from corroborating evidence"
- `0005-review-and-commit-policy.md` line 157: "append one confirmed `corroborating_evidence` edge without allocation value"

**Current state**: `match_edges` has no `role` column. The existing trigger uniformly rejects ALL inserts against committed events.

**Impact**: Blocks the Gmail slice's "late email evidence attachment to committed events" feature.

**Proposed fix**:

```sql
ALTER TABLE match_edges ADD COLUMN role TEXT NOT NULL DEFAULT 'financial_allocation'
  CHECK (role IN ('financial_allocation', 'corroborating_evidence'));

-- Replace blanket trigger with role-aware version
DROP TRIGGER committed_match_edge_cannot_be_inserted;
CREATE TRIGGER committed_match_edge_insert_role_aware
BEFORE INSERT ON match_edges
WHEN (SELECT status FROM ledger_events WHERE id = NEW.ledger_event_id) = 'committed'
  AND NEW.role = 'financial_allocation'
BEGIN
  SELECT RAISE(ABORT, 'financial allocation edge is immutable on committed event');
END;
```

**Decision required**: Implement now or defer to Gmail slice start?

**Effort**: 2 hours (migration + trigger rework + integration test)

---

### P1-5: Spec-required `source_documents.evidence_kind` column missing

**Spec reference**: `0002-database-schema.md` line 104: `evidence_kind: file | email_message`

**Proposed fix**:

```sql
ALTER TABLE source_documents ADD COLUMN evidence_kind TEXT NOT NULL DEFAULT 'file'
  CHECK (evidence_kind IN ('file', 'email_message'));
```

**Decision required**: Same timing question as P1-4.

**Effort**: 30 minutes

---

## P2 — Medium

### P2-1: `parse_runs.input_hash` migrated as empty string

**Location**: `0009_post_pr41_hardening.sql` line 55

**Current state**: Legacy rows receive `input_hash = ''` (empty string) with a `NOT NULL` constraint.

**Impact**: In JavaScript, `''` is falsy but the semantic confusion between "no hash available" and "empty hash" creates fragile code.

**Options**:

| Option | Pros | Cons |
|--------|------|------|
| A: Sentinel `'legacy:pre-hardening'` | NOT NULL preserved; explicit detection | Requires documentation |
| B: Make nullable + UPDATE to NULL | Clearest semantics | Another migration; audit all usage sites |
| C: Keep as-is + code convention | Zero migration | Fragile; every consumer must remember |

**Recommendation**: Option A.

**Effort**: 45 minutes

---

### P2-2: `parse_runs.status` has no CHECK constraint

**Location**: `0001_synthetic_core.sql` line 52

**Current state**: `status TEXT NOT NULL` — accepts any string.

**Contrast**: `external_records.status`, `jobs.status`, `accounts.status`, `ledger_events.status` all have CHECK constraints.

**Options**:

| Option | Pros | Cons |
|--------|------|------|
| A: Validation trigger | No table rebuild | Less declarative than CHECK |
| B: Table rebuild + CHECK | Consistent style | Another full rebuild (0009 just did one) |
| C: Application-layer only | Zero migration | Inconsistent with DB-enforced style |

**Recommendation**: Option A now; convert to CHECK in the next migration that touches `parse_runs`.

**Effort**: 1 hour

---

### P2-3: `match_edges` lacks `created_at` timestamp

**Impact**: No way to determine edge insertion order for audit trails.

**Fix**:

```sql
ALTER TABLE match_edges ADD COLUMN created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP;
```

**Note**: Existing rows receive migration timestamp, which is acceptable for synthetic-core test data.

**Effort**: 20 minutes

---

### P2-4: `ledger_legs` has no explicit ordering column

**Current state**: Leg order is encoded in the ID string (`${eventId}:leg:${index+1}`).

**Impact**: If ID generation strategy changes, leg ordering is lost. Financial legs have semantic position (debit/credit).

**Fix**:

```sql
ALTER TABLE ledger_legs ADD COLUMN leg_index INTEGER NOT NULL DEFAULT 1;
CREATE UNIQUE INDEX ledger_legs_event_order ON ledger_legs(ledger_event_id, leg_index);
```

**Effort**: 30 minutes + repository insert update

---

### P2-5: `jobs` table rebuilt 3 times for CHECK constraint changes

**History**: Created in 0006, rebuilt in 0007 (add `source_document_ingest`), rebuilt in 0009 (remove it).

**Options**:

| Option | Pros | Cons |
|--------|------|------|
| A: Relax to `CHECK (length(job_type) > 0)` + app enum | Never rebuild for new job types | Loses DB-level typo protection |
| B: Keep strict CHECK | DB-enforced | Full table rebuild per new job type |
| C: Registry table + trigger | Extensible + enforced | Over-engineered for current scale |

**Recommendation**: Option A. Job type enumeration changes far more often than data integrity is at risk.

**Effort**: 1 hour (next time jobs table needs migration)

---

### P2-6: `review_relationships` ordering CHECK assumes lexicographic IDs

**Location**: `0006_review_ledger.sql` line 90: `CHECK (first_external_record_id < second_external_record_id)`

**Impact**: If ID generation changes from UUIDv4 to a non-lexicographic scheme, the deduplication semantics break.

**Recommendation**: Document the ID generation requirement. No schema change needed if UUIDv4 remains standard.

**Effort**: 15 minutes (documentation)

---

## P3 — Low

### P3-1: `local_inbox_entry_observations` timestamp style inconsistency

Uses 4 INTEGER columns (`creation_nanoseconds`, `creation_seconds`, etc.) while the rest of the schema uses TEXT ISO-8601 timestamps.

**Recommendation**: Add SQL comment explaining these map directly to macOS `struct stat` fields (`st_birthtimespec`, `st_mtimespec`). No structural change needed.

---

### P3-2: No WAL journal mode configured

**Current state**: No `PRAGMA journal_mode = WAL` anywhere. Default rollback journal blocks readers during writes.

**Impact**: When the Rust job engine writes during `parse_document`, UI read queries may stall.

**Recommendation**: Introduce WAL when `review-ledger-backend` slice lands (Rust owns SQLCipher transactions). Requires coordination with backup/checkpoint logic.

**Effort**: 1 hour + Tauri/Rust coordination

---

### P3-3: Migration runner has no rollback support

**Current state**: `applyMigrations()` is forward-only.

**Recommendation**: Acceptable for a local-first app where backup/restore (spec 0009) is the recovery path. Document this decision explicitly.

---

## Implementation Plan

### Phase 1 — Immediate (this PR)

Non-controversial fixes requiring no product decisions:

1. Fix TypeScript type drift (P0-1)
2. Add self-merge guard triggers (P0-2)
3. Add `audit_log` covering index (P1-1)
4. Add `review_items` partial index (P1-2)
5. Add `ledger_events` observation index (P1-3)
6. Add `match_edges.created_at` (P2-3)

All delivered as one new migration: `0010_schema_review_hardening.sql`

### Phase 2 — Next sprint (requires decision)

7. `match_edges.role` + trigger rework (P1-4) — blocked on Gmail slice timeline
8. `source_documents.evidence_kind` (P1-5) — same dependency
9. `parse_runs.input_hash` sentinel fix (P2-1)
10. `parse_runs.status` trigger validation (P2-2)
11. `ledger_legs.leg_index` (P2-4)

### Phase 3 — Deferred

12. `jobs` CHECK relaxation (P2-5) — next time jobs needs migration
13. WAL mode (P3-2) — with review-ledger-backend slice
14. Documentation items (P2-6, P3-1, P3-3)

---

## Test Plan

- Existing `synthetic-core-flow.test.ts` must pass unchanged (backward compatibility).
- New trigger tests: attempt self-merge INSERT and UPDATE, expect ABORT.
- Index verification: `EXPLAIN QUERY PLAN` for `findAuditEntries`, `findOpenReviewItems`, `findLatestBalanceObservation` confirms index usage.
- Migration idempotency: run `applyMigrations` twice, second run is no-op.

---

## Open Discussion Points

1. **`match_edges.role` + `evidence_kind` timing**: Add now to unblock Gmail slice parallel development, or defer per slice-owned schema principle?
2. **`jobs.job_type` CHECK strategy**: Keep strict (rebuild per type) or relax to app-layer enum?
3. **WAL mode introduction**: Now or with `review-ledger-backend`?
4. **`parse_runs.input_hash` empty string**: Sentinel value vs nullable?
