-- Spec 0015 operational diagnostics boundary.
--
-- One durable, local-only run log for failures that the user-facing static
-- codes cannot explain. It lives inside the encrypted Vault database, so it
-- inherits SQLCipher at rest and disappears with the Vault.
--
-- Field whitelist (docs/specs/0015-job-engine-error-model.md "Operational
-- diagnostics boundary"): timestamps, stable static error codes, job
-- type/state, retry count, duration, app/runtime version, and redacted
-- component context. `component`, `error_code`, `error_kind`, `detail`, and
-- `cause` are that redacted context: `component` is a frozen identifier, and
-- `detail`/`cause` are written only through the redactor in
-- `apps/desktop/src-tauri/src/diagnostics.rs`, which is the single writer-side
-- guarantee that document or email content, extracted text, raw financial
-- fields, amounts, filenames or filesystem paths, mailbox addresses, OAuth
-- tokens, API keys, statement passwords, and model payloads never land here.
--
-- Retention: rows older than 30 days are deleted on Vault open
-- (`purge_expired_operational_logs`); export is user-initiated only.

CREATE TABLE operational_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  level TEXT NOT NULL CHECK (level IN ('warning', 'error')),
  component TEXT NOT NULL,
  error_code TEXT,
  error_kind TEXT,
  detail TEXT,
  cause TEXT,
  job_type TEXT,
  job_status TEXT,
  attempt INTEGER,
  duration_ms INTEGER,
  app_version TEXT NOT NULL
);

CREATE INDEX operational_logs_created_at ON operational_logs(created_at);
