-- committed external record finality on the UPDATE path.
-- 0013 guarded only INSERT, but both commit paths flip a staged/review record
-- to committed with UPDATE external_records SET status = 'committed'. Without
-- this guard the same stable_record_key can still gain a second committed
-- version through direct SQL. Without a committed sibling the update is legal,
-- so every first commit and every non-committed transition is untouched.

CREATE TRIGGER external_record_committed_update_is_final
BEFORE UPDATE OF status ON external_records
WHEN NEW.status = 'committed'
  AND OLD.status <> 'committed'
  AND EXISTS (
    SELECT 1 FROM external_records
    WHERE stable_record_key = NEW.stable_record_key
      AND status = 'committed'
  )
BEGIN
  SELECT RAISE(ABORT, 'committed stable_record_key cannot gain a new version');
END;
