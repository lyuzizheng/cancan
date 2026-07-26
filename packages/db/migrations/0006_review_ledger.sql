CREATE TABLE external_records_next (
  id TEXT PRIMARY KEY,
  parse_run_id TEXT NOT NULL REFERENCES parse_runs(id),
  source_document_id TEXT NOT NULL REFERENCES source_documents(id),
  account_id TEXT REFERENCES accounts(id),
  stable_record_key TEXT NOT NULL,
  version INTEGER NOT NULL CHECK (version > 0),
  status TEXT NOT NULL CHECK (status IN ('staged', 'review', 'committed', 'removed', 'superseded')),
  record_type TEXT NOT NULL,
  event_type TEXT,
  posted_on TEXT,
  amount_value TEXT,
  currency TEXT,
  account_balance_delta TEXT,
  raw_json TEXT NOT NULL,
  validation_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(stable_record_key, version)
);

INSERT INTO external_records_next(
  id, parse_run_id, source_document_id, account_id, stable_record_key, version,
  status, record_type, event_type, posted_on, amount_value, currency,
  account_balance_delta, raw_json, validation_json, created_at
)
SELECT
  id, parse_run_id, source_document_id, account_id, stable_record_key, version,
  status, record_type, event_type, posted_on, amount_value, currency,
  account_balance_delta, raw_json, validation_json, created_at
FROM external_records;

DROP TRIGGER committed_match_edge_cannot_be_inserted;
DROP TRIGGER committed_match_edge_cannot_be_updated;
DROP TRIGGER committed_match_edge_cannot_be_deleted;
DROP TABLE external_records;
ALTER TABLE external_records_next RENAME TO external_records;

CREATE TRIGGER committed_external_record_cannot_be_updated
BEFORE UPDATE ON external_records
WHEN OLD.status = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed external record is immutable');
END;

CREATE TRIGGER committed_external_record_cannot_be_deleted
BEFORE DELETE ON external_records
WHEN OLD.status = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed external record is immutable');
END;

CREATE TRIGGER committed_match_edge_cannot_be_inserted
BEFORE INSERT ON match_edges
WHEN (SELECT status FROM ledger_events WHERE id = NEW.ledger_event_id) = 'committed'
  OR (SELECT status FROM external_records WHERE id = NEW.external_record_id) = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed match edge is immutable');
END;

CREATE TRIGGER committed_match_edge_cannot_be_updated
BEFORE UPDATE ON match_edges
WHEN (SELECT status FROM ledger_events WHERE id = OLD.ledger_event_id) = 'committed'
  OR (SELECT status FROM external_records WHERE id = OLD.external_record_id) = 'committed'
  OR (SELECT status FROM ledger_events WHERE id = NEW.ledger_event_id) = 'committed'
  OR (SELECT status FROM external_records WHERE id = NEW.external_record_id) = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed match edge is immutable');
END;

CREATE TRIGGER committed_match_edge_cannot_be_deleted
BEFORE DELETE ON match_edges
WHEN (SELECT status FROM ledger_events WHERE id = OLD.ledger_event_id) = 'committed'
  OR (SELECT status FROM external_records WHERE id = OLD.external_record_id) = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed match edge is immutable');
END;

CREATE TABLE review_relationships (
  id TEXT PRIMARY KEY,
  event_type TEXT NOT NULL
    CHECK (event_type IN ('same_currency_transfer', 'credit_card_repayment')),
  first_external_record_id TEXT NOT NULL REFERENCES external_records(id),
  second_external_record_id TEXT NOT NULL REFERENCES external_records(id),
  first_review_item_id TEXT NOT NULL REFERENCES review_items(id),
  second_review_item_id TEXT NOT NULL REFERENCES review_items(id),
  allocation_value TEXT NOT NULL,
  unit TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('accepted', 'committed', 'invalidated')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (first_external_record_id < second_external_record_id),
  UNIQUE(first_external_record_id, second_external_record_id)
);

CREATE INDEX review_relationships_by_first_record
  ON review_relationships(first_external_record_id, status);

CREATE INDEX review_relationships_by_second_record
  ON review_relationships(second_external_record_id, status);

CREATE TABLE jobs (
  id TEXT PRIMARY KEY,
  job_type TEXT NOT NULL CHECK (job_type = 'commit_review_batch'),
  status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'blocked', 'cancelled')),
  priority INTEGER NOT NULL DEFAULT 0,
  attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
  max_attempts INTEGER NOT NULL DEFAULT 3 CHECK (max_attempts > 0),
  input_json TEXT NOT NULL,
  step_state_json TEXT,
  result_json TEXT,
  error_json TEXT,
  blocked_reason TEXT,
  related_source_document_id TEXT REFERENCES source_documents(id),
  related_money_source_id TEXT REFERENCES money_sources(id),
  related_review_item_id TEXT REFERENCES review_items(id),
  lease_owner TEXT,
  lease_until TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  started_at TEXT,
  finished_at TEXT
);

CREATE INDEX jobs_commit_review_claim
  ON jobs(job_type, status, lease_until, created_at);
