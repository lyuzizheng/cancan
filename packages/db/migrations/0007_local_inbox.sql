ALTER TABLE source_documents ADD COLUMN document_type TEXT;
ALTER TABLE source_documents ADD COLUMN statement_period_from TEXT;
ALTER TABLE source_documents ADD COLUMN statement_period_to TEXT;

CREATE INDEX source_documents_statement_coverage
  ON source_documents(money_source_id, document_type, statement_period_from, statement_period_to);

CREATE TABLE source_document_accounts (
  source_document_id TEXT NOT NULL REFERENCES source_documents(id),
  account_id TEXT NOT NULL REFERENCES accounts(id),
  PRIMARY KEY (source_document_id, account_id)
);

CREATE TABLE statement_coverage_decisions (
  money_source_id TEXT NOT NULL REFERENCES money_sources(id),
  account_id TEXT NOT NULL REFERENCES accounts(id),
  document_type TEXT NOT NULL,
  statement_period_from TEXT NOT NULL,
  statement_period_to TEXT NOT NULL,
  decision TEXT NOT NULL CHECK (decision IN ('not_expected', 'remind_later')),
  remind_after TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (
    money_source_id,
    account_id,
    document_type,
    statement_period_from,
    statement_period_to
  ),
  CHECK (
    (decision = 'not_expected' AND remind_after IS NULL)
    OR (decision = 'remind_later' AND remind_after IS NOT NULL)
  )
);

CREATE TABLE jobs_next (
  id TEXT PRIMARY KEY,
  job_type TEXT NOT NULL CHECK (
    job_type IN (
      'commit_review_batch',
      'source_document_ingest',
      'parse_document',
      'reconcile_document'
    )
  ),
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

INSERT INTO jobs_next(
  id, job_type, status, priority, attempts, max_attempts, input_json,
  step_state_json, result_json, error_json, blocked_reason,
  related_source_document_id, related_money_source_id, related_review_item_id,
  lease_owner, lease_until, created_at, updated_at, started_at, finished_at
)
SELECT
  id, job_type, status, priority, attempts, max_attempts, input_json,
  step_state_json, result_json, error_json, blocked_reason,
  related_source_document_id, related_money_source_id, related_review_item_id,
  lease_owner, lease_until, created_at, updated_at, started_at, finished_at
FROM jobs;

DROP TABLE jobs;
ALTER TABLE jobs_next RENAME TO jobs;

CREATE INDEX jobs_claim
  ON jobs(job_type, status, lease_until, created_at);
