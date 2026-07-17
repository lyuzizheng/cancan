ALTER TABLE source_documents
  ADD COLUMN original_filename TEXT NOT NULL DEFAULT 'Imported document';

ALTER TABLE source_documents
  ADD COLUMN mime_type TEXT NOT NULL DEFAULT 'application/octet-stream';

ALTER TABLE source_documents
  ADD COLUMN byte_size INTEGER NOT NULL DEFAULT 0 CHECK (byte_size >= 0);

ALTER TABLE source_documents
  ADD COLUMN encrypted_locator TEXT;

ALTER TABLE source_documents
  ADD COLUMN file_state TEXT NOT NULL DEFAULT 'missing'
    CHECK (file_state IN ('available', 'deleted', 'missing'));

ALTER TABLE source_documents
  ADD COLUMN deleted_at TEXT;

ALTER TABLE source_documents
  ADD COLUMN deletion_audit_id TEXT REFERENCES audit_log(id);

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
