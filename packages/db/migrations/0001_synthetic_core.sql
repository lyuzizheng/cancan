CREATE TABLE money_sources (
  id TEXT PRIMARY KEY,
  provider_key TEXT NOT NULL,
  display_name TEXT NOT NULL,
  source_type TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  money_source_id TEXT NOT NULL REFERENCES money_sources(id),
  provider_key TEXT NOT NULL,
  provider_account_id TEXT,
  account_type TEXT NOT NULL,
  display_name TEXT NOT NULL,
  masked_identifier TEXT,
  currency TEXT,
  status TEXT NOT NULL CHECK (status IN ('candidate', 'confirmed', 'archived', 'merged')),
  merged_into_account_id TEXT REFERENCES accounts(id),
  raw_identity_json TEXT,
  first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX accounts_provider_identity
  ON accounts(money_source_id, provider_key, provider_account_id)
  WHERE provider_account_id IS NOT NULL AND status <> 'merged';

CREATE TABLE instruments (
  id TEXT PRIMARY KEY,
  instrument_type TEXT NOT NULL,
  symbol TEXT NOT NULL,
  currency TEXT,
  display_name TEXT NOT NULL
);

CREATE TABLE source_documents (
  id TEXT PRIMARY KEY,
  money_source_id TEXT NOT NULL REFERENCES money_sources(id),
  file_sha256 TEXT NOT NULL UNIQUE,
  semantic_document_key TEXT NOT NULL,
  received_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE parse_runs (
  id TEXT PRIMARY KEY,
  source_document_id TEXT NOT NULL REFERENCES source_documents(id),
  normalization_profile_id TEXT NOT NULL,
  profile_json TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(source_document_id, normalization_profile_id)
);

CREATE TABLE external_records (
  id TEXT PRIMARY KEY,
  parse_run_id TEXT NOT NULL REFERENCES parse_runs(id),
  source_document_id TEXT NOT NULL REFERENCES source_documents(id),
  account_id TEXT REFERENCES accounts(id),
  stable_record_key TEXT NOT NULL,
  version INTEGER NOT NULL CHECK (version > 0),
  status TEXT NOT NULL CHECK (status IN ('staged', 'review', 'committed', 'superseded')),
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

CREATE TABLE ledger_events (
  id TEXT PRIMARY KEY,
  event_type TEXT NOT NULL,
  event_class TEXT NOT NULL CHECK (event_class IN ('posting', 'observation')),
  event_date TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'committed')),
  commit_idempotency_key TEXT NOT NULL UNIQUE,
  reverses_event_id TEXT REFERENCES ledger_events(id),
  replaces_event_id TEXT REFERENCES ledger_events(id),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE ledger_legs (
  id TEXT PRIMARY KEY,
  ledger_event_id TEXT NOT NULL REFERENCES ledger_events(id),
  account_id TEXT NOT NULL REFERENCES accounts(id),
  instrument_id TEXT NOT NULL REFERENCES instruments(id),
  amount_value TEXT,
  balance_value TEXT,
  quantity_value TEXT,
  currency TEXT,
  CHECK (amount_value IS NOT NULL OR balance_value IS NOT NULL OR quantity_value IS NOT NULL)
);

CREATE INDEX ledger_legs_by_account_currency
  ON ledger_legs(account_id, currency, ledger_event_id);

CREATE TABLE match_edges (
  external_record_id TEXT NOT NULL REFERENCES external_records(id),
  ledger_event_id TEXT NOT NULL REFERENCES ledger_events(id),
  allocation_value TEXT NOT NULL,
  unit TEXT NOT NULL,
  review_status TEXT NOT NULL CHECK (review_status IN ('suggested', 'confirmed', 'rejected')),
  unmatched_remainder_value TEXT NOT NULL DEFAULT '0',
  PRIMARY KEY(external_record_id, ledger_event_id)
);

CREATE INDEX match_edges_by_event ON match_edges(ledger_event_id, external_record_id);

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

CREATE TABLE review_items (
  id TEXT PRIMARY KEY,
  external_record_id TEXT NOT NULL REFERENCES external_records(id),
  reason_code TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open', 'resolved', 'dismissed')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE audit_log (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  action TEXT NOT NULL,
  actor TEXT NOT NULL,
  reason TEXT NOT NULL,
  source_ref TEXT,
  policy_version TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER committed_ledger_event_is_immutable
BEFORE UPDATE ON ledger_events
WHEN OLD.status = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed ledger event is immutable');
END;

CREATE TRIGGER committed_ledger_event_cannot_be_deleted
BEFORE DELETE ON ledger_events
WHEN OLD.status = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed ledger event is immutable');
END;

CREATE TRIGGER committed_ledger_leg_cannot_be_inserted
BEFORE INSERT ON ledger_legs
WHEN (SELECT status FROM ledger_events WHERE id = NEW.ledger_event_id) = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed ledger event legs are immutable');
END;

CREATE TRIGGER committed_ledger_leg_cannot_be_updated
BEFORE UPDATE ON ledger_legs
WHEN (SELECT status FROM ledger_events WHERE id = OLD.ledger_event_id) = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed ledger event legs are immutable');
END;

CREATE TRIGGER committed_ledger_leg_cannot_be_deleted
BEFORE DELETE ON ledger_legs
WHEN (SELECT status FROM ledger_events WHERE id = OLD.ledger_event_id) = 'committed'
BEGIN
  SELECT RAISE(ABORT, 'committed ledger event legs are immutable');
END;

CREATE TRIGGER audit_log_is_append_only_on_update
BEFORE UPDATE ON audit_log
BEGIN
  SELECT RAISE(ABORT, 'audit log is append-only');
END;

CREATE TRIGGER audit_log_is_append_only_on_delete
BEFORE DELETE ON audit_log
BEGIN
  SELECT RAISE(ABORT, 'audit log is append-only');
END;
