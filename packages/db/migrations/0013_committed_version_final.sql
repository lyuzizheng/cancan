-- committed external record finality: a committed external record version
-- is terminal. Reparse never creates a successor version and commit rejects
-- any record whose stable_record_key already has a committed version.

-- 存在 committed 兄弟的非 committed 后代版本是 bug 产物
UPDATE external_records SET status = 'superseded'
WHERE status IN ('staged', 'review', 'removed')
  AND EXISTS (
    SELECT 1 FROM external_records c
    WHERE c.stable_record_key = external_records.stable_record_key
      AND c.status = 'committed'
  );

UPDATE review_items SET status = 'resolved'
WHERE status = 'open' AND external_record_id IN (
  SELECT id FROM external_records
  WHERE status = 'superseded'
    AND EXISTS (
      SELECT 1 FROM external_records c
      WHERE c.stable_record_key = external_records.stable_record_key
        AND c.status = 'committed'
    )
);

UPDATE review_relationships SET status = 'invalidated'
WHERE status = 'accepted' AND (
  first_external_record_id IN (
    SELECT id FROM external_records
    WHERE status = 'superseded'
      AND EXISTS (
        SELECT 1 FROM external_records c
        WHERE c.stable_record_key = external_records.stable_record_key
          AND c.status = 'committed'
      )
  )
  OR second_external_record_id IN (
    SELECT id FROM external_records
    WHERE status = 'superseded'
      AND EXISTS (
        SELECT 1 FROM external_records c
        WHERE c.stable_record_key = external_records.stable_record_key
          AND c.status = 'committed'
      )
  )
);

CREATE TRIGGER external_record_committed_version_is_final
BEFORE INSERT ON external_records
WHEN EXISTS(
  SELECT 1 FROM external_records
  WHERE stable_record_key = NEW.stable_record_key AND status = 'committed'
)
BEGIN
  SELECT RAISE(ABORT, 'committed stable_record_key cannot gain a new version');
END;
