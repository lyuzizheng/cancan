CREATE TABLE money_source_candidates (
  id TEXT PRIMARY KEY,
  candidate_key_version INTEGER NOT NULL CHECK (candidate_key_version > 0),
  provider_key TEXT NOT NULL CHECK (
    length(provider_key) > 0
    AND length(CAST(provider_key AS BLOB)) <= 128
  ),
  candidate_scope_kind TEXT NOT NULL
    CHECK (candidate_scope_kind IN ('provider_singleton', 'provider_root_id')),
  candidate_scope_value TEXT NOT NULL CHECK (
    length(CAST(candidate_scope_value AS BLOB)) <= 512
  ),
  status TEXT NOT NULL CHECK (status IN ('pending', 'kept_unassigned', 'confirmed')),
  version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
  confirmed_money_source_id TEXT REFERENCES money_sources(id),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (
    candidate_key_version,
    provider_key,
    candidate_scope_kind,
    candidate_scope_value
  ),
  CHECK (
    (candidate_scope_kind = 'provider_singleton' AND candidate_scope_value = '')
    OR (
      candidate_scope_kind = 'provider_root_id'
      AND length(candidate_scope_value) > 0
    )
  ),
  CHECK (
    (status = 'confirmed' AND confirmed_money_source_id IS NOT NULL)
    OR (status IN ('pending', 'kept_unassigned') AND confirmed_money_source_id IS NULL)
  )
);

ALTER TABLE source_documents
  ADD COLUMN money_source_candidate_id TEXT REFERENCES money_source_candidates(id);

ALTER TABLE source_documents
  ADD COLUMN attention_parked_reason TEXT;

ALTER TABLE source_documents
  ADD COLUMN attention_parked_at TEXT;

CREATE TRIGGER source_document_intake_owner_is_valid_on_insert
BEFORE INSERT ON source_documents
WHEN NOT (
  (NEW.money_source_candidate_id IS NULL OR NEW.money_source_id IS NULL)
  AND (
    (NEW.attention_parked_reason IS NULL AND NEW.attention_parked_at IS NULL)
    OR (
      NEW.attention_parked_reason IS NOT NULL
      AND length(NEW.attention_parked_reason) > 0
      AND length(CAST(NEW.attention_parked_reason AS BLOB)) <= 128
      AND NEW.attention_parked_at IS NOT NULL
    )
  )
)
BEGIN
  SELECT RAISE(ABORT, 'invalid source document intake owner state');
END;

CREATE TRIGGER source_document_intake_owner_is_valid_on_update
BEFORE UPDATE OF
  money_source_id,
  money_source_candidate_id,
  attention_parked_reason,
  attention_parked_at
ON source_documents
WHEN NOT (
  (NEW.money_source_candidate_id IS NULL OR NEW.money_source_id IS NULL)
  AND (
    (NEW.attention_parked_reason IS NULL AND NEW.attention_parked_at IS NULL)
    OR (
      NEW.attention_parked_reason IS NOT NULL
      AND length(NEW.attention_parked_reason) > 0
      AND length(CAST(NEW.attention_parked_reason AS BLOB)) <= 128
      AND NEW.attention_parked_at IS NOT NULL
    )
  )
)
BEGIN
  SELECT RAISE(ABORT, 'invalid source document intake owner state');
END;

CREATE TABLE intake_batches (
  id TEXT PRIMARY KEY,
  acquisition_channel TEXT NOT NULL
    CHECK (acquisition_channel IN ('explicit_handoff', 'local_inbox', 'gmail')),
  opened_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  sealed_at TEXT NOT NULL,
  completed_at TEXT,
  notification_state TEXT NOT NULL DEFAULT 'undecided'
    CHECK (notification_state IN ('undecided', 'suppressed', 'pending', 'emitted')),
  notification_decided_at TEXT,
  notification_emitted_at TEXT,
  CHECK (
    (
      completed_at IS NULL
      AND notification_state = 'undecided'
      AND notification_decided_at IS NULL
      AND notification_emitted_at IS NULL
    )
    OR (
      completed_at IS NOT NULL
      AND notification_state IN ('suppressed', 'pending')
      AND notification_decided_at IS NOT NULL
      AND notification_emitted_at IS NULL
    )
    OR (
      completed_at IS NOT NULL
      AND notification_state = 'emitted'
      AND notification_decided_at IS NOT NULL
      AND notification_emitted_at IS NOT NULL
    )
  )
);

CREATE TABLE intake_batch_items (
  id TEXT PRIMARY KEY,
  intake_batch_id TEXT NOT NULL REFERENCES intake_batches(id) ON DELETE CASCADE,
  input_ordinal INTEGER NOT NULL CHECK (input_ordinal >= 0),
  safe_input_label TEXT NOT NULL CHECK (
    length(safe_input_label) > 0
    AND length(CAST(safe_input_label AS BLOB)) <= 240
    AND instr(safe_input_label, '/') = 0
    AND instr(safe_input_label, char(92)) = 0
    AND instr(safe_input_label, char(0)) = 0
  ),
  source_document_id TEXT REFERENCES source_documents(id),
  capture_outcome TEXT NOT NULL CHECK (
    capture_outcome IN (
      'pending',
      'captured',
      'already_present',
      'restore_confirmation_required',
      'suppressed',
      'rejected'
    )
  ),
  rejection_kind TEXT CHECK (
    rejection_kind IN ('visible_receipt', 'background_action_required')
  ),
  rejection_code TEXT CHECK (
    rejection_code IS NULL
    OR (
      length(rejection_code) > 0
      AND length(CAST(rejection_code AS BLOB)) <= 128
    )
  ),
  rejection_parked_at TEXT,
  rejection_resolved_at TEXT,
  rejection_resolution_kind TEXT CHECK (
    rejection_resolution_kind = 'superseded_by_new_attempt'
  ),
  resolved_by_batch_item_id TEXT REFERENCES intake_batch_items(id),
  acquisition_input_key TEXT,
  acquisition_input_version TEXT,
  retry_of_batch_item_id TEXT REFERENCES intake_batch_items(id),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  finalized_at TEXT,
  UNIQUE (intake_batch_id, input_ordinal),
  CHECK (
    (capture_outcome = 'pending' AND finalized_at IS NULL)
    OR (capture_outcome <> 'pending' AND finalized_at IS NOT NULL)
  ),
  CHECK (
    (capture_outcome = 'pending' AND source_document_id IS NULL)
    OR (
      capture_outcome IN ('captured', 'already_present', 'restore_confirmation_required')
      AND source_document_id IS NOT NULL
    )
    OR (capture_outcome IN ('suppressed', 'rejected'))
  ),
  CHECK (
    (
      capture_outcome = 'rejected'
      AND rejection_kind IS NOT NULL
      AND rejection_code IS NOT NULL
      AND source_document_id IS NULL
    )
    OR (
      capture_outcome = 'suppressed'
      AND rejection_kind IS NULL
      AND rejection_parked_at IS NULL
      AND rejection_resolved_at IS NULL
      AND rejection_resolution_kind IS NULL
      AND resolved_by_batch_item_id IS NULL
    )
    OR (
      capture_outcome NOT IN ('rejected', 'suppressed')
      AND rejection_kind IS NULL
      AND rejection_code IS NULL
      AND rejection_parked_at IS NULL
      AND rejection_resolved_at IS NULL
      AND rejection_resolution_kind IS NULL
      AND resolved_by_batch_item_id IS NULL
    )
  ),
  CHECK (
    rejection_parked_at IS NULL
    OR (
      capture_outcome = 'rejected'
      AND rejection_kind = 'background_action_required'
    )
  ),
  CHECK (
    (
      rejection_resolved_at IS NULL
      AND rejection_resolution_kind IS NULL
      AND resolved_by_batch_item_id IS NULL
    )
    OR (
      rejection_resolved_at IS NOT NULL
      AND rejection_resolution_kind = 'superseded_by_new_attempt'
      AND resolved_by_batch_item_id IS NOT NULL
      AND capture_outcome = 'rejected'
      AND rejection_kind = 'background_action_required'
      AND resolved_by_batch_item_id <> id
    )
  ),
  CHECK (
    (
      acquisition_input_key IS NULL
      AND acquisition_input_version IS NULL
    )
    OR (
      acquisition_input_key IS NOT NULL
      AND length(acquisition_input_key) = 64
      AND acquisition_input_key NOT GLOB '*[^0-9a-f]*'
      AND acquisition_input_version IS NOT NULL
      AND length(acquisition_input_version) = 67
      AND substr(acquisition_input_version, 1, 3) = 'v1:'
      AND substr(acquisition_input_version, 4) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  CHECK (retry_of_batch_item_id IS NULL OR retry_of_batch_item_id <> id)
);

CREATE TRIGGER intake_batch_completion_is_monotonic
BEFORE UPDATE OF completed_at ON intake_batches
WHEN OLD.completed_at IS NOT NULL AND NEW.completed_at IS NOT OLD.completed_at
BEGIN
  SELECT RAISE(ABORT, 'intake batch completion is immutable');
END;

CREATE TRIGGER intake_batch_identity_is_immutable
BEFORE UPDATE OF id, acquisition_channel, opened_at, sealed_at ON intake_batches
WHEN NEW.id IS NOT OLD.id
  OR NEW.acquisition_channel IS NOT OLD.acquisition_channel
  OR NEW.opened_at IS NOT OLD.opened_at
  OR NEW.sealed_at IS NOT OLD.sealed_at
BEGIN
  SELECT RAISE(ABORT, 'intake batch identity is immutable');
END;

CREATE TRIGGER intake_batch_notification_is_monotonic
BEFORE UPDATE OF notification_state ON intake_batches
WHEN NOT (
  NEW.notification_state = OLD.notification_state
  OR (OLD.notification_state = 'undecided' AND NEW.notification_state IN ('suppressed', 'pending'))
  OR (OLD.notification_state = 'pending' AND NEW.notification_state = 'emitted')
)
BEGIN
  SELECT RAISE(ABORT, 'intake batch notification state cannot move backwards');
END;

CREATE TRIGGER intake_batch_notification_timestamps_are_immutable
BEFORE UPDATE OF notification_decided_at, notification_emitted_at ON intake_batches
WHEN (
    OLD.notification_decided_at IS NOT NULL
    AND NEW.notification_decided_at IS NOT OLD.notification_decided_at
  )
  OR (
    OLD.notification_emitted_at IS NOT NULL
    AND NEW.notification_emitted_at IS NOT OLD.notification_emitted_at
  )
BEGIN
  SELECT RAISE(ABORT, 'intake batch notification timestamps are immutable');
END;

CREATE TRIGGER intake_batch_item_identity_is_immutable
BEFORE UPDATE OF
  id,
  intake_batch_id,
  input_ordinal,
  safe_input_label,
  acquisition_input_key,
  acquisition_input_version,
  retry_of_batch_item_id,
  created_at
ON intake_batch_items
WHEN NEW.id IS NOT OLD.id
  OR NEW.intake_batch_id IS NOT OLD.intake_batch_id
  OR NEW.input_ordinal IS NOT OLD.input_ordinal
  OR NEW.safe_input_label IS NOT OLD.safe_input_label
  OR NEW.acquisition_input_key IS NOT OLD.acquisition_input_key
  OR NEW.acquisition_input_version IS NOT OLD.acquisition_input_version
  OR NEW.retry_of_batch_item_id IS NOT OLD.retry_of_batch_item_id
  OR NEW.created_at IS NOT OLD.created_at
BEGIN
  SELECT RAISE(ABORT, 'intake batch item identity is immutable');
END;

CREATE TRIGGER intake_batch_item_outcome_is_monotonic
BEFORE UPDATE OF capture_outcome ON intake_batch_items
WHEN NEW.capture_outcome <> OLD.capture_outcome
  AND (OLD.capture_outcome <> 'pending' OR NEW.capture_outcome = 'pending')
BEGIN
  SELECT RAISE(ABORT, 'intake batch item outcome is immutable');
END;

CREATE TRIGGER intake_batch_item_terminal_receipt_is_immutable
BEFORE UPDATE OF source_document_id, rejection_kind, rejection_code ON intake_batch_items
WHEN OLD.capture_outcome <> 'pending'
  AND (
    NEW.source_document_id IS NOT OLD.source_document_id
    OR NEW.rejection_kind IS NOT OLD.rejection_kind
    OR NEW.rejection_code IS NOT OLD.rejection_code
  )
BEGIN
  SELECT RAISE(ABORT, 'intake batch item terminal receipt is immutable');
END;

CREATE TRIGGER intake_batch_item_finalized_at_is_immutable
BEFORE UPDATE OF finalized_at ON intake_batch_items
WHEN OLD.finalized_at IS NOT NULL AND NEW.finalized_at IS NOT OLD.finalized_at
BEGIN
  SELECT RAISE(ABORT, 'intake batch item finalization is immutable');
END;

CREATE TRIGGER intake_batch_item_parked_at_is_immutable
BEFORE UPDATE OF rejection_parked_at ON intake_batch_items
WHEN OLD.rejection_parked_at IS NOT NULL
  AND NEW.rejection_parked_at IS NOT OLD.rejection_parked_at
BEGIN
  SELECT RAISE(ABORT, 'intake batch item parking is immutable');
END;

CREATE TRIGGER intake_batch_item_resolution_is_immutable
BEFORE UPDATE OF
  rejection_resolved_at,
  rejection_resolution_kind,
  resolved_by_batch_item_id
ON intake_batch_items
WHEN OLD.rejection_resolved_at IS NOT NULL
  AND (
    NEW.rejection_resolved_at IS NOT OLD.rejection_resolved_at
    OR NEW.rejection_resolution_kind IS NOT OLD.rejection_resolution_kind
    OR NEW.resolved_by_batch_item_id IS NOT OLD.resolved_by_batch_item_id
  )
BEGIN
  SELECT RAISE(ABORT, 'intake batch item resolution is immutable');
END;

CREATE TRIGGER intake_batch_item_resolution_has_matching_replacement
BEFORE UPDATE OF
  rejection_resolved_at,
  rejection_resolution_kind,
  resolved_by_batch_item_id
ON intake_batch_items
WHEN OLD.rejection_resolved_at IS NULL
  AND NEW.rejection_resolved_at IS NOT NULL
  AND NOT EXISTS (
    SELECT 1
    FROM intake_batch_items AS replacement
    JOIN intake_batches AS replacement_batch
      ON replacement_batch.id = replacement.intake_batch_id
    JOIN intake_batches AS original_batch
      ON original_batch.id = NEW.intake_batch_id
    WHERE replacement.id = NEW.resolved_by_batch_item_id
      AND replacement.capture_outcome = 'pending'
      AND (
        replacement.retry_of_batch_item_id = NEW.id
        OR (
          original_batch.acquisition_channel = 'local_inbox'
          AND replacement_batch.acquisition_channel = 'local_inbox'
          AND NEW.acquisition_input_key IS NOT NULL
          AND replacement.acquisition_input_key = NEW.acquisition_input_key
        )
      )
  )
BEGIN
  SELECT RAISE(ABORT, 'intake batch item resolution requires a matching replacement');
END;
