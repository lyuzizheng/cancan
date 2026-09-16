use super::*;

const KEY: [u8; KEY_LEN] = [0x31; KEY_LEN];

fn open_connection() -> (tempfile::TempDir, Connection) {
    let root = tempfile::tempdir().expect("temporary Vault");
    let database_path = root.path().join(DATABASE_FILE_NAME);
    let connection = open_encrypted_database(
        &database_path,
        &KEY,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .expect("open encrypted database");
    (root, connection)
}

fn insert_missing_document(connection: &Connection, id: &str, hash_byte: char) {
    connection
        .execute(
            "INSERT INTO source_documents( \
               id, file_sha256, original_filename, mime_type, byte_size, file_state \
             ) VALUES (?1, ?2, 'statement.pdf', 'application/pdf', 1, 'missing')",
            params![id, hash_byte.to_string().repeat(64)],
        )
        .expect("seed source document");
}

#[test]
fn upgrades_a_populated_v10_vault_through_the_production_registry() {
    let (_root, mut connection) = open_connection();
    apply_migration_set(&mut connection, &MIGRATIONS[..10]).expect("apply v10 schema");
    connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-existing', 'dbs', 'Existing DBS', 'bank')",
            [],
        )
        .expect("seed Money Source");
    insert_missing_document(&connection, "document-existing", 'a');
    connection
        .execute(
            "INSERT INTO jobs( \
               id, job_type, status, input_json, related_source_document_id \
             ) VALUES ( \
               'job-existing', 'parse_document', 'queued', \
               '{\"documentId\":\"document-existing\",\"logicalRunKey\":\"job-existing\"}', \
               'document-existing' \
             )",
            [],
        )
        .expect("seed parse job");
    connection
        .execute(
            "INSERT INTO gmail_accounts( \
               id, mailbox_address, secret_storage_key, connection_status \
             ) VALUES ( \
               'gmail-existing', 'owner@example.com', 'gmail-refresh-token:existing', 'connected' \
             )",
            [],
        )
        .expect("seed Gmail identity");

    apply_migrations(&mut connection).expect("apply production v13 migration");

    let schema_version: i64 = connection
        .query_row("SELECT max(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("read schema version");
    assert_eq!(schema_version, 13);
    assert_eq!(
        connection
            .query_row(
                "SELECT display_name FROM money_sources WHERE id = 'source-existing'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("read preserved Money Source"),
        "Existing DBS"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT status FROM jobs WHERE id = 'job-existing'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("read preserved job"),
        "queued"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT connection_status FROM gmail_accounts WHERE id = 'gmail-existing'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("read preserved Gmail identity"),
        "connected"
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("check upgraded foreign keys"),
        0
    );
}

#[test]
fn enforces_candidate_identity_and_document_owner_state() {
    let (_root, mut connection) = open_connection();
    apply_migrations(&mut connection).expect("apply production schema");
    connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
            [],
        )
        .expect("seed Money Source");
    connection
        .execute(
            "INSERT INTO money_source_candidates( \
               id, candidate_key_version, provider_key, candidate_scope_kind, \
               candidate_scope_value, status \
             ) VALUES ( \
               'candidate-dbs', 1, 'dbs', 'provider_singleton', '', 'pending' \
             )",
            [],
        )
        .expect("seed source candidate");
    insert_missing_document(&connection, "document-unassigned", 'b');
    connection
        .execute(
            "UPDATE source_documents \
             SET money_source_candidate_id = 'candidate-dbs', \
                 attention_parked_reason = 'source_confirmation', \
                 attention_parked_at = '2026-08-02T00:00:00Z' \
             WHERE id = 'document-unassigned'",
            [],
        )
        .expect("link and park unassigned evidence");

    assert!(
        connection
            .execute(
                "UPDATE source_documents \
                 SET money_source_id = 'source-dbs' \
                 WHERE id = 'document-unassigned'",
                [],
            )
            .is_err(),
        "a document cannot retain candidate and configured-source owners"
    );
    assert!(
        connection
            .execute(
                "INSERT INTO money_source_candidates( \
                   id, candidate_key_version, provider_key, candidate_scope_kind, \
                   candidate_scope_value, status \
                 ) VALUES ( \
                   'candidate-invalid-singleton', 1, 'dbs', 'provider_singleton', \
                   'not-empty', 'pending' \
                 )",
                [],
            )
            .is_err(),
        "singleton identity must use the tagged empty scope"
    );
    assert!(
        connection
            .execute(
                "INSERT INTO money_source_candidates( \
                   id, candidate_key_version, provider_key, candidate_scope_kind, \
                   candidate_scope_value, status \
                 ) VALUES ( \
                   'candidate-duplicate', 1, 'dbs', 'provider_singleton', '', 'pending' \
                 )",
                [],
            )
            .is_err(),
        "composite identity must be unique without string concatenation"
    );
    assert!(
        connection
            .execute(
                "INSERT INTO money_source_candidates( \
                   id, candidate_key_version, provider_key, candidate_scope_kind, \
                   candidate_scope_value, status \
                 ) VALUES (?1, 1, 'dbs', 'provider_root_id', ?2, 'pending')",
                params!["candidate-oversized", "x".repeat(513)],
            )
            .is_err(),
        "provider root identity must be bounded"
    );
}

#[test]
fn enforces_receipt_finalization_and_rejection_supersession() {
    let (_root, mut connection) = open_connection();
    apply_migrations(&mut connection).expect("apply production schema");
    connection
        .execute(
            "INSERT INTO intake_batches( \
               id, acquisition_channel, sealed_at \
             ) VALUES ('batch-first', 'local_inbox', '2026-08-02T00:00:00Z')",
            [],
        )
        .expect("create sealed batch");
    connection
        .execute(
            "INSERT INTO intake_batch_items( \
               id, intake_batch_id, input_ordinal, safe_input_label, capture_outcome, \
               acquisition_input_key, acquisition_input_version \
             ) VALUES ( \
               'item-first', 'batch-first', 0, 'statement.pdf', 'pending', ?1, ?2 \
             )",
            params!["a".repeat(64), format!("v1:{}", "b".repeat(64))],
        )
        .expect("create pending item");
    assert!(
        connection
            .execute(
                "UPDATE intake_batch_items \
                 SET capture_outcome = 'captured', finalized_at = '2026-08-02T00:01:00Z' \
                 WHERE id = 'item-first'",
                [],
            )
            .is_err(),
        "captured receipt must resolve a source document"
    );
    connection
        .execute(
            "UPDATE intake_batch_items \
             SET capture_outcome = 'rejected', \
                 rejection_kind = 'background_action_required', \
                 rejection_code = 'capture_failed', \
                 rejection_parked_at = '2026-08-02T00:01:00Z', \
                 finalized_at = '2026-08-02T00:01:00Z' \
             WHERE id = 'item-first'",
            [],
        )
        .expect("finalize actionable rejection");
    assert!(
        connection
            .execute(
                "UPDATE intake_batch_items \
                 SET capture_outcome = 'pending', finalized_at = NULL \
                 WHERE id = 'item-first'",
                [],
            )
            .is_err(),
        "terminal outcome must not return to pending"
    );
    assert!(
        connection
            .execute(
                "UPDATE intake_batch_items SET rejection_code = 'different_failure' \
                 WHERE id = 'item-first'",
                [],
            )
            .is_err(),
        "terminal rejection details must remain immutable"
    );

    connection
        .execute(
            "INSERT INTO intake_batches( \
               id, acquisition_channel, sealed_at \
             ) VALUES ('batch-retry', 'local_inbox', '2026-08-02T00:02:00Z')",
            [],
        )
        .expect("create retry batch");
    connection
        .execute(
            "INSERT INTO intake_batch_items( \
               id, intake_batch_id, input_ordinal, safe_input_label, capture_outcome, \
               acquisition_input_key, acquisition_input_version, retry_of_batch_item_id \
             ) VALUES ( \
               'item-unrelated', 'batch-retry', 0, 'other.pdf', 'pending', ?1, ?2, NULL \
             )",
            params!["d".repeat(64), format!("v1:{}", "c".repeat(64))],
        )
        .expect("create unrelated attempt");
    assert!(
        connection
            .execute(
                "UPDATE intake_batch_items \
                 SET rejection_resolved_at = '2026-08-02T00:02:00Z', \
                     rejection_resolution_kind = 'superseded_by_new_attempt', \
                     resolved_by_batch_item_id = 'item-unrelated' \
                 WHERE id = 'item-first'",
                [],
            )
            .is_err(),
        "an unrelated attempt must not hide an actionable rejection"
    );
    connection
        .execute(
            "INSERT INTO intake_batch_items( \
               id, intake_batch_id, input_ordinal, safe_input_label, capture_outcome, \
               acquisition_input_key, acquisition_input_version, retry_of_batch_item_id \
             ) VALUES ( \
               'item-retry', 'batch-retry', 1, 'statement.pdf', 'pending', ?1, ?2, 'item-first' \
             )",
            params!["a".repeat(64), format!("v1:{}", "c".repeat(64))],
        )
        .expect("create explicitly correlated replacement");
    connection
        .execute(
            "UPDATE intake_batch_items \
             SET rejection_resolved_at = '2026-08-02T00:02:00Z', \
                 rejection_resolution_kind = 'superseded_by_new_attempt', \
                 resolved_by_batch_item_id = 'item-retry' \
             WHERE id = 'item-first'",
            [],
        )
        .expect("resolve stale parked rejection");

    let state: (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT rejection_resolved_at, resolved_by_batch_item_id \
             FROM intake_batch_items WHERE id = 'item-first'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read resolved rejection");
    assert_eq!(
        state,
        (
            Some("2026-08-02T00:02:00Z".to_owned()),
            Some("item-retry".to_owned())
        )
    );
    assert!(
        connection
            .execute(
                "UPDATE intake_batch_items SET rejection_resolved_at = NULL \
                 WHERE id = 'item-first'",
                [],
            )
            .is_err(),
        "resolved rejection must not become actionable again"
    );
    assert!(
        connection
            .execute(
                "UPDATE intake_batch_items SET rejection_parked_at = NULL \
                 WHERE id = 'item-first'",
                [],
            )
            .is_err(),
        "parking history must not be rewritten"
    );
}
