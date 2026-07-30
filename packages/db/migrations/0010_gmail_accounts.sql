CREATE TABLE gmail_accounts (
  id TEXT PRIMARY KEY,
  mailbox_address TEXT NOT NULL UNIQUE,
  secret_storage_key TEXT NOT NULL UNIQUE,
  connection_status TEXT NOT NULL
    CHECK (
      connection_status IN (
        'pending_save',
        'connected',
        'pending_delete',
        'disconnected'
      )
    ),
  ai_processing_provider_fingerprint TEXT,
  attachment_ai_processing_authorized_at TEXT,
  transaction_body_ai_processing_authorized_at TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
