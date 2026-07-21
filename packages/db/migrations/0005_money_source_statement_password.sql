CREATE TABLE statement_secret_refs (
  id TEXT PRIMARY KEY,
  money_source_id TEXT NOT NULL UNIQUE REFERENCES money_sources(id),
  secret_storage_key TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL
    CHECK (status IN ('pending_save', 'saved', 'pending_delete')),
  hint_label TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
