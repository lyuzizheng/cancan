use super::super::tests::open_store;
use super::*;
use crate::content_fingerprint::CONTENT_FINGERPRINT_VERSION;
use rusqlite::params;

fn fingerprint(seed: char) -> ContentFingerprint {
    ContentFingerprint {
        version: CONTENT_FINGERPRINT_VERSION,
        hex: seed.to_string().repeat(64),
    }
}

/// Seeds one evidence row of a statement the Vault already parsed: the stored
/// artifact carrying the canonical content, its succeeded parse job, and one
/// reusable record.
fn seed_parsed_statement(
    store: &ManualImportStore,
    document_id: &str,
    hash_byte: char,
    content: &ContentFingerprint,
    job_status: &str,
    with_record: bool,
) {
    store
        .connection
        .execute(
            "INSERT INTO source_documents( \
               id, money_source_id, file_sha256, semantic_document_key, original_filename, \
               mime_type, byte_size, encrypted_locator, file_state, document_type, \
               statement_period_from, statement_period_to, canonical_content_fingerprint, \
               canonical_content_fingerprint_version \
             ) VALUES ( \
               ?1, 'source-dbs', ?2, 'dbs:cash:2026-07', 'statement-july.pdf', \
               'application/pdf', 1024, 'vault-seed-locator', 'available', 'bank_statement', \
               '2026-07-01', '2026-07-31', ?3, ?4 \
             )",
            params![
                document_id,
                hash_byte.to_string().repeat(64),
                content.hex.as_str(),
                content.version,
            ],
        )
        .expect("seed parsed statement");
    store
        .connection
        .execute(
            "INSERT INTO jobs(id, job_type, status, input_json, related_source_document_id, \
                    finished_at) \
             VALUES (?1, 'parse_document', ?2, '{}', ?3, CURRENT_TIMESTAMP)",
            params![format!("job-{document_id}"), job_status, document_id],
        )
        .expect("seed parse job");
    if !with_record {
        return;
    }
    store
        .connection
        .execute(
            "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, logical_run_key, \
               profile_json, input_hash, status \
             ) VALUES (?1, ?2, 'statement-v1', ?1, '{}', 'input-hash', 'succeeded')",
            params![format!("run-{document_id}"), document_id],
        )
        .expect("seed parse run");
    store
        .connection
        .execute(
            "INSERT INTO external_records( \
               id, parse_run_id, source_document_id, stable_record_key, version, status, \
               record_type, raw_json, validation_json \
             ) VALUES (?1, ?2, ?3, 'dbs:cash:2026-07:1', 1, 'staged', 'transaction', '{}', '{}')",
            params![
                format!("record-{document_id}"),
                format!("run-{document_id}"),
                document_id,
            ],
        )
        .expect("seed reusable record");
}

/// Seeds the artifact under test and returns its claimed parse job.
fn seed_claimed_artifact(
    store: &mut ManualImportStore,
    document_id: &str,
    hash_byte: char,
) -> ParseDocumentClaim {
    store
        .connection
        .execute(
            "INSERT INTO source_documents( \
               id, file_sha256, original_filename, mime_type, byte_size, encrypted_locator, \
               file_state \
             ) VALUES (?1, ?2, 'statement-july-reprint.pdf', 'application/pdf', 2048, \
                       'vault-seed-locator', 'available')",
            params![document_id, hash_byte.to_string().repeat(64)],
        )
        .expect("seed artifact");
    claim_parse_job(store, document_id)
}

/// Enqueues one more parse job for a stored document and claims it.
fn claim_parse_job(store: &mut ManualImportStore, document_id: &str) -> ParseDocumentClaim {
    store
        .enqueue_source_document_pipeline(document_id)
        .expect("enqueue parse job");
    let job = store
        .queued_parse_document_jobs()
        .expect("read queued parse jobs")
        .into_iter()
        .find(|job| job.document_id == document_id)
        .expect("queued parse job");
    store
        .start_parse_document_job(&job)
        .expect("claim parse job")
        .expect("parse job is claimable")
}

fn source_document(
    store: &ManualImportStore,
    document_id: &str,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    store
        .connection
        .query_row(
            "SELECT canonical_content_fingerprint, canonical_content_match_document_id, \
                    semantic_document_key, document_type \
             FROM source_documents WHERE id = ?1",
            [document_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read source document")
}

fn parse_job_state(store: &ManualImportStore, document_id: &str) -> (String, Option<String>) {
    store
        .connection
        .query_row(
            "SELECT status, result_json FROM jobs \
             WHERE related_source_document_id = ?1 AND job_type = 'parse_document' \
             ORDER BY created_at, id LIMIT 1",
            [document_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read parse job")
}

#[test]
fn links_same_content_artifacts_without_parsing_them_again() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let content = fingerprint('a');
    seed_parsed_statement(&store, "document-first", '1', &content, "succeeded", true);
    let claim = seed_claimed_artifact(&mut store, "document-reprint", '2');

    let decision = store
        .record_canonical_content_for_claimed_job(&claim, &content)
        .expect("record canonical content");

    assert_eq!(
        decision,
        CanonicalContentDecision::Matched {
            document_id: "document-first".to_owned()
        }
    );
    let (stored_fingerprint, matched_document_id, semantic_key, document_type) =
        source_document(&store, "document-reprint");
    assert_eq!(stored_fingerprint, Some(content.hex.clone()));
    assert_eq!(matched_document_id, Some("document-first".to_owned()));
    assert_eq!(semantic_key, Some("dbs:cash:2026-07".to_owned()));
    assert_eq!(document_type, Some("bank_statement".to_owned()));
    let (job_status, result_json) = parse_job_state(&store, "document-reprint");
    assert_eq!(job_status, "succeeded");
    assert_eq!(
        result_json.as_deref(),
        Some("{\"status\":\"same_content\"}")
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM jobs \
                 WHERE related_source_document_id = 'document-reprint'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count artifact jobs"),
        1,
        "a reprint must not enqueue a second parse job",
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM external_records \
                 WHERE source_document_id = 'document-reprint'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count artifact records"),
        0,
        "a reprint must not regenerate records",
    );
    let (action, entity_id, source_ref, policy_version): (String, String, String, String) = store
        .connection
        .query_row(
            "SELECT action, entity_id, source_ref, policy_version FROM audit_log \
             WHERE action = 'source_document_content_matched'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read content match audit row");
    assert_eq!(entity_id, "document-reprint");
    assert_eq!(source_ref, "document-first");
    assert_eq!(policy_version, "content-fingerprint-v1");
    assert_eq!(action, "source_document_content_matched");
}

#[test]
fn keeps_the_artifact_as_its_own_evidence_row() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let content = fingerprint('a');
    seed_parsed_statement(&store, "document-first", '1', &content, "succeeded", true);
    let claim = seed_claimed_artifact(&mut store, "document-reprint", '2');

    store
        .record_canonical_content_for_claimed_job(&claim, &content)
        .expect("record canonical content");

    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT file_sha256 FROM source_documents WHERE id = 'document-reprint'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("read artifact bytes"),
        "2".repeat(64),
        "the byte-different artifact keeps its own file identity",
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM source_documents", [], |row| row
                .get::<_, i64>(0))
            .expect("count documents"),
        2,
        "the artifact is additional evidence, not a replacement",
    );
}

#[test]
fn falls_back_to_normal_parsing_for_every_incomplete_match() {
    // A second artifact of content whose only other evidence row is a tombstone
    // for the same bytes still parses: exact-hash restore decisions own that
    // path, and a blocked parse has no records to reuse.
    for (label, job_status, with_record) in [
        ("blocked parse", "blocked", true),
        ("failed parse", "failed", true),
        ("parse without records", "succeeded", false),
    ] {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        let content = fingerprint('a');
        seed_parsed_statement(
            &store,
            "document-first",
            '1',
            &content,
            job_status,
            with_record,
        );
        let claim = seed_claimed_artifact(&mut store, "document-reprint", '2');

        let decision = store
            .record_canonical_content_for_claimed_job(&claim, &content)
            .expect("record canonical content");

        assert_eq!(
            decision,
            CanonicalContentDecision::Parsed,
            "{label} must continue through normal parsing",
        );
        let (_, matched_document_id, semantic_key, _) = source_document(&store, "document-reprint");
        assert_eq!(matched_document_id, None, "{label}");
        assert_eq!(semantic_key, None, "{label}");
        assert_eq!(
            parse_job_state(&store, "document-reprint").0,
            "running",
            "{label}: the claimed job stays open for the normalizer",
        );
        assert_eq!(
            source_document(&store, "document-reprint").0,
            Some(content.hex.clone()),
            "{label}: the fingerprint is durable even without a match",
        );
    }
}

#[test]
fn separates_fingerprint_versions() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let content = fingerprint('a');
    seed_parsed_statement(&store, "document-first", '1', &content, "succeeded", true);
    let claim = seed_claimed_artifact(&mut store, "document-reprint", '2');

    let other_version = ContentFingerprint {
        version: CONTENT_FINGERPRINT_VERSION + 1,
        hex: content.hex.clone(),
    };
    assert_eq!(
        store
            .record_canonical_content_for_claimed_job(&claim, &other_version)
            .expect("record canonical content"),
        CanonicalContentDecision::Parsed,
        "a digest from another algorithm version proves nothing",
    );
    assert_eq!(
        store
            .record_canonical_content_for_claimed_job(&claim, &content)
            .expect("record canonical content"),
        CanonicalContentDecision::Matched {
            document_id: "document-first".to_owned()
        },
    );
}

#[test]
fn never_treats_the_artifact_as_its_own_statement() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let content = fingerprint('a');
    let claim = seed_claimed_artifact(&mut store, "document-only", '1');

    assert_eq!(
        store
            .record_canonical_content_for_claimed_job(&claim, &content)
            .expect("record canonical content"),
        CanonicalContentDecision::Parsed,
        "the first artifact of a statement has nothing to reuse",
    );
    assert_eq!(
        store
            .record_canonical_content_for_claimed_job(&claim, &content)
            .expect("record canonical content"),
        CanonicalContentDecision::Parsed,
        "a stored fingerprint cannot match its own document",
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log WHERE action = 'source_document_content_matched'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count content match audits"),
        0,
    );
}

#[test]
fn reparsing_a_linked_artifact_reuses_the_same_statement() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let content = fingerprint('a');
    seed_parsed_statement(&store, "document-first", '1', &content, "succeeded", true);
    let claim = seed_claimed_artifact(&mut store, "document-reprint", '2');
    store
        .record_canonical_content_for_claimed_job(&claim, &content)
        .expect("record canonical content");

    let reparsed = claim_parse_job(&mut store, "document-reprint");
    assert_eq!(
        store
            .record_canonical_content_for_claimed_job(&reparsed, &content)
            .expect("record canonical content"),
        CanonicalContentDecision::Matched {
            document_id: "document-first".to_owned()
        },
        "a reparse of a reprint reuses the same statement",
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log WHERE action = 'source_document_content_matched'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count content match audits"),
        1,
        "re-linking the same pair writes no second audit row",
    );
}

#[test]
fn writes_nothing_when_the_claim_is_no_longer_current() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let content = fingerprint('a');
    seed_parsed_statement(&store, "document-first", '1', &content, "succeeded", true);
    let claim = seed_claimed_artifact(&mut store, "document-reprint", '2');
    let superseded = ParseDocumentClaim {
        claim_token: "parse-lease-superseded".to_owned(),
        ..claim
    };

    let error = store
        .record_canonical_content_for_claimed_job(&superseded, &content)
        .expect_err("a superseded claim cannot finish the job");

    assert!(error.to_string().contains("no longer claimed"), "{error}");
    let (stored_fingerprint, matched_document_id, _, _) =
        source_document(&store, "document-reprint");
    assert_eq!(
        stored_fingerprint, None,
        "the fingerprint write rolls back with the link",
    );
    assert_eq!(matched_document_id, None);
    assert_eq!(parse_job_state(&store, "document-reprint").0, "running");
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM audit_log", [], |row| row
                .get::<_, i64>(0))
            .expect("count audit rows"),
        0,
    );
}

#[test]
fn keeps_the_tombstone_decision_path_untouched() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let content = fingerprint('a');
    seed_parsed_statement(&store, "document-first", '1', &content, "succeeded", true);
    store
        .connection
        .execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES ( \
               'audit-seed-deletion', 'source_document', 'document-first', \
               'source_file_deletion_decided', 'user', 'user_requested', \
               'source-file-deletion-v1' \
             )",
            [],
        )
        .expect("seed deletion audit row");
    store
        .connection
        .execute(
            "UPDATE source_documents SET encrypted_locator = NULL, file_state = 'deleted', \
                    deleted_at = CURRENT_TIMESTAMP, \
                    deletion_audit_id = 'audit-seed-deletion' \
             WHERE id = 'document-first'",
            [],
        )
        .expect("record the deleted source file");
    let claim = seed_claimed_artifact(&mut store, "document-reprint", '2');

    assert_eq!(
        store
            .record_canonical_content_for_claimed_job(&claim, &content)
            .expect("record canonical content"),
        CanonicalContentDecision::Matched {
            document_id: "document-first".to_owned()
        },
        "a deleted source file still owns reusable statement content",
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT file_state FROM source_documents WHERE id = 'document-first'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("read matched file state"),
        "deleted",
        "content matching never restores or re-deletes a tombstone",
    );
}

/// The product tombstones documents instead of deleting their rows, so this
/// exercises the schema's own contract for the one case a row delete creates:
/// the pointer clears with the row it points at instead of failing the delete
/// or dangling, and the artifact keeps the identity it adopted.
#[test]
fn clears_the_link_when_the_matched_document_row_is_deleted() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let content = fingerprint('a');
    seed_parsed_statement(&store, "document-first", '1', &content, "succeeded", true);
    let claim = seed_claimed_artifact(&mut store, "document-reprint", '2');
    store
        .record_canonical_content_for_claimed_job(&claim, &content)
        .expect("record canonical content");
    for statement in [
        "DELETE FROM external_records WHERE source_document_id = 'document-first'",
        "DELETE FROM parse_runs WHERE source_document_id = 'document-first'",
        "DELETE FROM jobs WHERE related_source_document_id = 'document-first'",
        "DELETE FROM source_documents WHERE id = 'document-first'",
    ] {
        store
            .connection
            .execute(statement, [])
            .expect("delete the matched statement row");
    }

    let (fingerprint, matched_document_id, semantic_key, document_type) =
        source_document(&store, "document-reprint");
    assert_eq!(fingerprint, Some(content.hex.clone()));
    assert_eq!(
        matched_document_id, None,
        "the link cannot outlive the row it points at",
    );
    assert_eq!(semantic_key, Some("dbs:cash:2026-07".to_owned()));
    assert_eq!(document_type, Some("bank_statement".to_owned()));
    let (job_status, result_json) = parse_job_state(&store, "document-reprint");
    assert_eq!(job_status, "succeeded");
    assert_eq!(
        result_json.as_deref(),
        Some("{\"status\":\"same_content\"}")
    );
}
