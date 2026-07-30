-- 0010_schema_review_hardening.sql
-- Owning slice: schema-review-hardening
-- Addresses: P0-2 (self-merge guard), P1-1/P1-2/P1-3 (missing indexes), P2-3 (match_edges.created_at)
-- See docs/alignment-temp/db-schema-review.md for full review report.

-- P0-2: Prevent accounts from merging into themselves.
-- Multi-node cycle prevention (A->B->A) remains an application-layer responsibility.
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

-- P1-1: Covering index for findAuditEntries(entity_type, entity_id) ordered by created_at, id.
-- audit_log is append-only and grows with every financial mutation.
CREATE INDEX audit_log_entity_lookup
  ON audit_log(entity_type, entity_id, created_at, id);

-- P1-2: Partial index for findOpenReviewItems() which filters status = 'open'.
-- Open items are typically a small subset; partial index stays compact.
CREATE INDEX review_items_open ON review_items(status, id) WHERE status = 'open';

-- P1-3: Index for findLatestBalanceObservation() which filters event_class + status
-- and orders by event_date DESC.
CREATE INDEX ledger_events_observation_lookup
  ON ledger_events(event_class, status, event_date DESC);

-- P2-3: Add created_at to match_edges for audit traceability.
-- Existing rows receive migration timestamp (acceptable for synthetic-core data).
ALTER TABLE match_edges ADD COLUMN created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP;
