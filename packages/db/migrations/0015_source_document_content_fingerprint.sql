-- Specs 0002 (document and record identity) and 0004 (normalizer boundary)
-- canonical-content fingerprint.
--
-- One evidence row keeps three separate identities: the exact imported byte
-- sequence (`file_sha256`), the canonical locally extracted content
-- (`canonical_content_fingerprint` plus its algorithm version), and the
-- provider statement identity (`semantic_document_key`). The fingerprint is
-- written by the trusted host only after complete local canonical extraction
-- succeeds, and it is deliberately non-unique because byte-different
-- artifacts with equal content remain separate evidence rows.
--
-- `canonical_content_match_document_id` records the single durable fact the
-- same-content short-circuit produces: this artifact is additional evidence
-- under the classified statement of the matched document, whose successful
-- trusted parse and reusable records the host reused instead of running AI
-- normalization and regenerating records. It stays null for the first (or
-- only) artifact of a statement and for every artifact whose local
-- extraction could not prove equality.
--
-- The pointer cannot outlive the evidence it joins: the column's
-- self-referencing foreign key clears it when the matched document row is
-- deleted (the product tombstones documents instead, so this only matters for
-- a row delete), and an artifact's own row delete takes its pointer with it.
-- The triggers below only keep one row's fingerprint, version, and pointer
-- mutually consistent; referent existence is the foreign key's job.

ALTER TABLE source_documents ADD COLUMN canonical_content_fingerprint TEXT;

ALTER TABLE source_documents ADD COLUMN canonical_content_fingerprint_version INTEGER;

ALTER TABLE source_documents
  ADD COLUMN canonical_content_match_document_id TEXT
    REFERENCES source_documents(id) ON DELETE SET NULL;

CREATE INDEX source_documents_canonical_content_fingerprint
  ON source_documents(canonical_content_fingerprint, canonical_content_fingerprint_version);

CREATE TRIGGER source_document_content_fingerprint_is_valid_on_insert
BEFORE INSERT ON source_documents
WHEN NOT (
  (NEW.canonical_content_fingerprint IS NULL
    AND NEW.canonical_content_fingerprint_version IS NULL
    AND NEW.canonical_content_match_document_id IS NULL)
  OR (
    NEW.canonical_content_fingerprint IS NOT NULL
    AND length(CAST(NEW.canonical_content_fingerprint AS BLOB)) BETWEEN 1 AND 128
    AND NEW.canonical_content_fingerprint_version IS NOT NULL
    AND NEW.canonical_content_fingerprint_version > 0
    AND (NEW.canonical_content_match_document_id IS NULL
         OR NEW.canonical_content_match_document_id <> NEW.id)
  )
)
BEGIN
  SELECT RAISE(ABORT, 'invalid source document content fingerprint');
END;

CREATE TRIGGER source_document_content_fingerprint_is_valid_on_update
BEFORE UPDATE OF
  canonical_content_fingerprint,
  canonical_content_fingerprint_version,
  canonical_content_match_document_id
ON source_documents
WHEN NOT (
  (NEW.canonical_content_fingerprint IS NULL
    AND NEW.canonical_content_fingerprint_version IS NULL
    AND NEW.canonical_content_match_document_id IS NULL)
  OR (
    NEW.canonical_content_fingerprint IS NOT NULL
    AND length(CAST(NEW.canonical_content_fingerprint AS BLOB)) BETWEEN 1 AND 128
    AND NEW.canonical_content_fingerprint_version IS NOT NULL
    AND NEW.canonical_content_fingerprint_version > 0
    AND (NEW.canonical_content_match_document_id IS NULL
         OR NEW.canonical_content_match_document_id <> NEW.id)
  )
)
BEGIN
  SELECT RAISE(ABORT, 'invalid source document content fingerprint');
END;
