CREATE TABLE accounts_next (
  id TEXT PRIMARY KEY,
  money_source_id TEXT NOT NULL REFERENCES money_sources(id),
  provider_key TEXT NOT NULL,
  provider_account_id TEXT,
  account_type TEXT NOT NULL,
  display_name TEXT NOT NULL,
  masked_identifier TEXT,
  currency TEXT,
  status TEXT NOT NULL CHECK (status IN ('candidate', 'confirmed', 'dismissed', 'archived', 'merged')),
  merged_into_account_id TEXT REFERENCES accounts(id),
  raw_identity_json TEXT,
  first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO accounts_next(
  id, money_source_id, provider_key, provider_account_id, account_type, display_name,
  masked_identifier, currency, status, merged_into_account_id, raw_identity_json,
  first_seen_at, last_seen_at, created_at, updated_at
)
SELECT
  id, money_source_id, provider_key, provider_account_id, account_type, display_name,
  masked_identifier, currency, status, merged_into_account_id, raw_identity_json,
  first_seen_at, last_seen_at, created_at, updated_at
FROM accounts;

DROP TABLE accounts;
ALTER TABLE accounts_next RENAME TO accounts;
CREATE UNIQUE INDEX accounts_provider_identity
  ON accounts(money_source_id, provider_key, provider_account_id)
  WHERE provider_account_id IS NOT NULL AND status <> 'merged';

CREATE TABLE parse_runs_next (
  id TEXT PRIMARY KEY,
  source_document_id TEXT NOT NULL REFERENCES source_documents(id),
  normalization_profile_id TEXT NOT NULL,
  logical_run_key TEXT NOT NULL,
  profile_json TEXT NOT NULL,
  input_hash TEXT NOT NULL,
  output_hash TEXT,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(source_document_id, logical_run_key)
);

INSERT INTO parse_runs_next(
  id, source_document_id, normalization_profile_id, logical_run_key, profile_json,
  input_hash, output_hash, status, created_at
)
SELECT
  id, source_document_id, normalization_profile_id, id, profile_json,
  '', NULL, status, created_at
FROM parse_runs;

DROP TABLE parse_runs;
ALTER TABLE parse_runs_next RENAME TO parse_runs;
CREATE INDEX parse_runs_source_profile
  ON parse_runs(source_document_id, normalization_profile_id, created_at);

CREATE TABLE jobs_next (
  id TEXT PRIMARY KEY,
  job_type TEXT NOT NULL CHECK (
    job_type IN ('commit_review_batch', 'parse_document', 'reconcile_document')
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
  id, job_type, status, priority, attempts, max_attempts,
  CASE
    WHEN job_type = 'parse_document' THEN json_set(
      input_json,
      '$.logicalRunKey',
      COALESCE(json_extract(input_json, '$.logicalRunKey'), id)
    )
    ELSE input_json
  END,
  step_state_json, result_json, error_json, blocked_reason,
  related_source_document_id, related_money_source_id, related_review_item_id,
  lease_owner, lease_until, created_at, updated_at, started_at, finished_at
FROM jobs
WHERE job_type <> 'source_document_ingest';

DROP TABLE jobs;
ALTER TABLE jobs_next RENAME TO jobs;
CREATE INDEX jobs_claim
  ON jobs(job_type, status, lease_until, created_at);

CREATE TABLE local_inbox_entry_observations (
  entry_key TEXT PRIMARY KEY,
  creation_nanoseconds INTEGER NOT NULL,
  creation_seconds INTEGER NOT NULL,
  change_nanoseconds INTEGER NOT NULL,
  change_seconds INTEGER NOT NULL,
  device INTEGER NOT NULL,
  inode INTEGER NOT NULL,
  last_observed_entry_identity TEXT NOT NULL,
  modified_nanoseconds INTEGER NOT NULL,
  modified_seconds INTEGER NOT NULL,
  size_bytes INTEGER NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
