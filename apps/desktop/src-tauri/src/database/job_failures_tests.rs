//! The two failure contracts `job_failures.rs` owns: an attempt that still has
//! retries left requeues instead of failing, and a claim that refuses an
//! exhausted job still records the failure spec 0015 requires.

use super::database_test_support::import_source;
use super::tests::open_store;
use super::*;
use crate::diagnostics::JobFailureDetail;
use std::io;

/// Asserts the record spec 0015 requires of a job that could not run: the
/// two-layer `error_json` on the row plus one operational log entry naming it.
///
/// A claim that refused an exhausted job used to leave neither, which made the
/// dead letter invisible in diagnostics.
fn assert_refused_job_failure(
    store: &ManualImportStore,
    job_id: &str,
    job_type: &str,
    component: &str,
    attempts: i64,
) {
    let (status, blocked_reason, error_json): (String, String, String) = store
        .connection
        .query_row(
            "SELECT status, blocked_reason, error_json FROM jobs WHERE id = ?1",
            [job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read the refused job");
    assert_eq!(status, "failed");
    assert_eq!(blocked_reason, "retry_limit_reached");
    let error_json: serde_json::Value =
        serde_json::from_str(&error_json).expect("error_json is an object");
    assert_eq!(error_json["errorCode"], "retry_limit_reached");
    assert_eq!(error_json["technical"]["errorCode"], "retry_limit_reached");
    assert_eq!(error_json["technical"]["jobType"], job_type);
    assert_eq!(error_json["technical"]["attempt"], attempts);
    assert_eq!(error_json["technical"]["maxAttempts"], attempts);
    assert_eq!(error_json["technical"]["retryable"], false);

    let (level, error_code, logged_job_type, attempt): (String, String, String, i64) = store
        .connection
        .query_row(
            "SELECT level, error_code, job_type, attempt FROM operational_logs \
             WHERE component = ?1 ORDER BY id DESC LIMIT 1",
            [component],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read the refusal entry");
    assert_eq!(level, "error");
    assert_eq!(error_code, "retry_limit_reached");
    assert_eq!(attempt, attempts);
    assert_eq!(
        logged_job_type, job_type,
        "the entry must describe the job it was written for"
    );
}

/// Spec 0015: a transient failure is worth another attempt. Startup recovery
/// requeues a running job whose lease expired, so a transient reconcile failure
/// must return the document to the queue instead of stranding its staged
/// records behind a terminal job.
#[test]
fn a_transient_reconcile_failure_requeues_the_job_for_another_attempt() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF reconcile retry").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import_source(
                &source_path,
                "document-reconcile-retry",
                "audit-reconcile-retry",
            ),
            None,
        )
        .expect("import source");
    store
        .connection
        .execute(
            "INSERT INTO jobs( \
               id, job_type, status, input_json, related_source_document_id, \
               related_money_source_id, lease_owner, lease_until, attempts, max_attempts, started_at \
             ) VALUES ( \
               'job-reconcile-retry', 'reconcile_document', 'running', '{}', \
               'document-reconcile-retry', 'source-dbs', 'reconcile-document', \
               datetime('now', '-1 second'), 1, 3, datetime('now', '-2 seconds') \
             )",
            [],
        )
        .expect("seed a running reconcile job");
    let detail = JobFailureDetail::from_error(
        None,
        &io::Error::new(
            io::ErrorKind::TimedOut,
            "the reconcile transaction timed out",
        ),
    );

    store
        .fail_reconcile_document(
            "document-reconcile-retry",
            "reconcile_failed",
            Some(&detail),
        )
        .expect("record the transient reconcile failure");

    let state: (String, Option<String>, Option<String>, Option<String>) = store
        .connection
        .query_row(
            "SELECT status, error_json, blocked_reason, finished_at FROM jobs WHERE id = ?1",
            ["job-reconcile-retry"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read the requeued reconcile job");
    assert_eq!(
        state,
        ("queued".to_owned(), None, None, None),
        "a transient failure must requeue the attempt, not fail the job"
    );
    assert!(
        store
            .queued_reconcile_document_ids()
            .expect("read the reconcile queue")
            .contains(&"document-reconcile-retry".to_owned()),
        "the reconcile pump must see the document again"
    );
    let (level, error_code, detail): (String, String, String) = store
        .connection
        .query_row(
            "SELECT level, error_code, detail FROM operational_logs \
             WHERE component = 'job.reconcile_document' ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read the requeue entry");
    assert_eq!(
        level, "warning",
        "an attempt that keeps its retries has not failed yet"
    );
    assert_eq!(error_code, "reconcile_failed");
    assert_eq!(detail, "the reconcile transaction timed out");
}

#[test]
fn a_parse_job_whose_attempts_are_spent_fails_with_a_logged_failure() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF spent attempts").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import_source(
                &source_path,
                "document-refused-parse",
                "audit-refused-parse",
            ),
            None,
        )
        .expect("import source");
    store
        .connection
        .execute(
            "UPDATE jobs SET attempts = 3, max_attempts = 3 WHERE job_type = 'parse_document'",
            [],
        )
        .expect("spend the remaining attempts");
    let job = store
        .queued_parse_document_jobs()
        .expect("read the queued parse job")
        .pop()
        .expect("queued parse job");

    assert!(
        store
            .start_parse_document_job(&job)
            .expect("refuse the exhausted claim")
            .is_none()
    );
    assert_refused_job_failure(
        &store,
        &job.job_id,
        "parse_document",
        "job.parse_document",
        3,
    );
}

#[test]
fn a_reconcile_job_whose_attempts_are_spent_fails_with_a_logged_failure() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF spent attempts").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import_source(
                &source_path,
                "document-refused-reconcile",
                "audit-refused-reconcile",
            ),
            None,
        )
        .expect("import source");
    store
        .connection
        .execute(
            "INSERT INTO jobs( \
               id, job_type, status, input_json, related_source_document_id, attempts, max_attempts \
             ) VALUES ( \
               'job-reconcile-refused', 'reconcile_document', 'queued', '{}', \
               'document-refused-reconcile', 3, 3 \
             )",
            [],
        )
        .expect("seed an exhausted reconcile job");

    assert!(
        !store
            .start_reconcile_document("document-refused-reconcile")
            .expect("refuse the exhausted claim")
    );
    assert_refused_job_failure(
        &store,
        "job-reconcile-refused",
        "reconcile_document",
        "job.reconcile_document",
        3,
    );
}

#[test]
fn a_review_batch_whose_attempts_are_spent_fails_with_a_logged_failure() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    store
        .connection
        .execute(
            "INSERT INTO jobs(id, job_type, status, input_json, attempts, max_attempts) \
             VALUES ( \
               'job-review-refused', 'commit_review_batch', 'queued', \
               '{\"reviewItemIds\":[\"review-refused\"]}', 3, 3 \
             )",
            [],
        )
        .expect("seed an exhausted review batch");

    assert!(
        store
            .claim_review_batch("job-review-refused", "test-worker")
            .expect("refuse the exhausted claim")
            .is_none()
    );
    assert_refused_job_failure(
        &store,
        "job-review-refused",
        "commit_review_batch",
        "job.commit_review_batch",
        3,
    );
}
