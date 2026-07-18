CREATE TABLE source_documents_next (
  id TEXT PRIMARY KEY,
  money_source_id TEXT REFERENCES money_sources(id),
  file_sha256 TEXT NOT NULL UNIQUE,
  semantic_document_key TEXT,
  received_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  original_filename TEXT NOT NULL DEFAULT 'Imported document',
  mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
  byte_size INTEGER NOT NULL DEFAULT 0 CHECK (byte_size >= 0),
  encrypted_locator TEXT,
  file_state TEXT NOT NULL DEFAULT 'missing'
    CHECK (file_state IN ('available', 'deleted', 'missing')),
  deleted_at TEXT,
  deletion_audit_id TEXT REFERENCES audit_log(id)
);

INSERT INTO source_documents_next(
  id,
  money_source_id,
  file_sha256,
  semantic_document_key,
  received_at,
  original_filename,
  mime_type,
  byte_size,
  encrypted_locator,
  file_state,
  deleted_at,
  deletion_audit_id
)
SELECT
  id,
  money_source_id,
  file_sha256,
  semantic_document_key,
  received_at,
  original_filename,
  mime_type,
  byte_size,
  encrypted_locator,
  file_state,
  deleted_at,
  deletion_audit_id
FROM source_documents;

DROP TABLE source_documents;
ALTER TABLE source_documents_next RENAME TO source_documents;

CREATE INDEX source_documents_money_source_received
  ON source_documents(money_source_id, received_at DESC);

CREATE TRIGGER source_document_file_state_is_valid_on_insert
BEFORE INSERT ON source_documents
WHEN NOT (
  (NEW.file_state = 'available'
    AND NEW.encrypted_locator IS NOT NULL
    AND length(NEW.encrypted_locator) > 0
    AND NEW.deleted_at IS NULL
    AND NEW.deletion_audit_id IS NULL)
  OR (NEW.file_state = 'deleted'
    AND NEW.encrypted_locator IS NULL
    AND NEW.deleted_at IS NOT NULL
    AND NEW.deletion_audit_id IS NOT NULL)
  OR (NEW.file_state = 'missing'
    AND NEW.deleted_at IS NULL
    AND NEW.deletion_audit_id IS NULL)
)
BEGIN
  SELECT RAISE(ABORT, 'invalid source document file lifecycle');
END;

CREATE TRIGGER source_document_file_state_is_valid_on_update
BEFORE UPDATE OF encrypted_locator, file_state, deleted_at, deletion_audit_id
ON source_documents
WHEN NOT (
  (NEW.file_state = 'available'
    AND NEW.encrypted_locator IS NOT NULL
    AND length(NEW.encrypted_locator) > 0
    AND NEW.deleted_at IS NULL
    AND NEW.deletion_audit_id IS NULL)
  OR (NEW.file_state = 'deleted'
    AND NEW.encrypted_locator IS NULL
    AND NEW.deleted_at IS NOT NULL
    AND NEW.deletion_audit_id IS NOT NULL)
  OR (NEW.file_state = 'missing'
    AND NEW.deleted_at IS NULL
    AND NEW.deletion_audit_id IS NULL)
)
BEGIN
  SELECT RAISE(ABORT, 'invalid source document file lifecycle');
END;
