ALTER TABLE external_records
  ADD COLUMN posting_status TEXT CHECK (posting_status IN ('provisional', 'posted'));
