-- phase1-intake-experience: persist the exact provider root identity on Money
-- Sources so root-scoped candidate confirmation can match a configured source
-- by exact identity instead of failing closed.
ALTER TABLE money_sources
  ADD COLUMN provider_root_id TEXT CHECK (
    provider_root_id IS NULL
    OR (
      length(provider_root_id) > 0
      AND length(CAST(provider_root_id AS BLOB)) <= 512
    )
  );

CREATE UNIQUE INDEX money_sources_provider_root_identity
  ON money_sources(provider_key, provider_root_id)
  WHERE provider_root_id IS NOT NULL;
