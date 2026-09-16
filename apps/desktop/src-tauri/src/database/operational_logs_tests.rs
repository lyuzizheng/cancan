use super::tests::{KEY, open_store};
use super::*;
use crate::diagnostics::{
    JobFailureDetail, OPERATIONAL_LOG_RETENTION_DAYS, OperationalLogEntry, redact,
};
use std::io;
use zeroize::Zeroizing;

fn import_source<'a>(
    source_path: &'a Path,
    document_id: &'a str,
    audit_id: &'a str,
) -> SourceDocumentImport<'a> {
    SourceDocumentImport {
        audit_actor: "user",
        audit_id,
        audit_policy_version: "manual-import-v1",
        audit_reason: "manual_import",
        document_id,
        mime_type: "application/pdf",
        original_filename: "DBS-July-2026.pdf",
        source_path,
    }
}

#[test]
fn retention_purges_entries_older_than_the_window_and_keeps_the_rest() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    for component in ["test.recent", "test.stale"] {
        store
            .record_operational_log(&OperationalLogEntry::failure_code(
                component,
                "test_failure",
            ))
            .expect("record failure");
    }
    store
        .connection
        .execute(
            "UPDATE operational_logs SET created_at = datetime('now', ?1) WHERE component = ?2",
            params![
                format!("-{} days", OPERATIONAL_LOG_RETENTION_DAYS + 1),
                "test.stale"
            ],
        )
        .expect("age one entry");

    assert_eq!(
        store.purge_expired_operational_logs().expect("purge"),
        1,
        "only the entry outside the retention window is deleted"
    );
    let preview = store
        .operational_diagnostics_preview()
        .expect("preview after purge");
    assert_eq!(preview.entry_count, 1);
    assert_eq!(preview.retention_days, OPERATIONAL_LOG_RETENTION_DAYS);
    assert_eq!(
        preview
            .components
            .iter()
            .map(|category| category.label.as_str())
            .collect::<Vec<_>>(),
        vec!["test.recent"]
    );
}

#[test]
fn preview_and_export_carry_the_redacted_detail_and_state_the_boundary() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store = open_store(root.path());
    let failure = io::Error::new(
        io::ErrorKind::PermissionDenied,
        "copy /Users/owner/Statements/DBS-July-2026.pdf failed for owner@example.com: \
         balance 1234.56 EUR",
    );
    store
        .record_operational_log(&OperationalLogEntry::failure(
            "documents.store_prepared",
            "import_failed",
            &failure,
        ))
        .expect("record failure");

    let preview = store
        .operational_diagnostics_preview()
        .expect("preview before export");
    assert_eq!(preview.entry_count, 1);
    assert_eq!(
        preview
            .error_codes
            .iter()
            .map(|category| category.label.as_str())
            .collect::<Vec<_>>(),
        vec!["import_failed"]
    );
    let sample = preview.sample_lines.join("\n");
    assert!(sample.contains("documents.store_prepared"));
    assert!(sample.contains("import_failed"));
    assert!(sample.contains("<path>"), "{sample}");
    assert!(!sample.contains("/Users/owner"), "{sample}");
    assert!(!sample.contains("DBS-July-2026.pdf"), "{sample}");
    assert!(!sample.contains("owner@example.com"), "{sample}");
    assert!(!sample.contains("1234.56"), "{sample}");

    let export = store
        .operational_diagnostics_export()
        .expect("export after preview");
    assert!(export.contains("CanCan operational diagnostics export"));
    assert!(export.contains("retention_days: 30"));
    assert!(export.contains("entries_retained: 1"));
    assert!(
        export.contains("quoted values are removed before this file is written"),
        "the export states its redaction boundary"
    );
    for secret in [
        "/Users/owner",
        "DBS-July-2026.pdf",
        "owner@example.com",
        "1234.56",
        "PermissionDenied",
    ] {
        assert!(
            !export.contains(secret),
            "{secret} must not leave the Vault"
        );
    }
}

#[test]
fn exhausted_parse_job_records_the_technical_layer_and_the_job_context_entry() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF transient sidecar failure").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import_source(&source_path, "document-diagnostics", "audit-diagnostics"),
            None,
        )
        .expect("import source");
    let detail = JobFailureDetail::from_code(Some("normalizer"), "normalizer_unavailable");

    let mut attempts = 0;
    while let Some(job) = store
        .queued_parse_document_jobs()
        .expect("read queued parse job")
        .pop()
    {
        let claim = store
            .start_parse_document_job(&job)
            .expect("start parse job")
            .expect("claim parse job");
        store
            .fail_parse_document_job(&claim, "normalizer_failed", Some(&detail))
            .expect("record parse failure");
        attempts += 1;
    }
    assert_eq!(
        attempts, 3,
        "the parse job retries until its attempts are spent"
    );

    let error_json: String = store
        .connection
        .query_row(
            "SELECT error_json FROM jobs WHERE job_type = 'parse_document'",
            [],
            |row| row.get(0),
        )
        .expect("read failed job error_json");
    let error_json: serde_json::Value =
        serde_json::from_str(&error_json).expect("error_json is an object");
    assert_eq!(error_json["errorCode"], "normalizer_failed");
    assert_eq!(error_json["technical"]["errorCode"], "normalizer_failed");
    assert_eq!(error_json["technical"]["provider"], "normalizer");
    assert_eq!(error_json["technical"]["jobType"], "parse_document");
    assert_eq!(error_json["technical"]["attempt"], 3);
    assert_eq!(error_json["technical"]["maxAttempts"], 3);
    assert_eq!(error_json["technical"]["retryable"], false);
    assert_eq!(
        error_json["technical"]["related"]["sourceDocumentId"],
        "document-diagnostics"
    );

    // The same failure is in the operational log with the job context the
    // `jobs` row carries.
    let preview = store
        .operational_diagnostics_preview()
        .expect("preview recorded failures");
    let lines = preview.sample_lines.join("\n");
    assert!(lines.contains("job.parse_document"), "{lines}");
    assert!(lines.contains("parse_document"));
    assert!(
        preview
            .components
            .iter()
            .any(|category| category.label == "job.parse_document"),
        "{:?}",
        preview.components
    );
}

#[test]
fn redaction_removes_paths_filenames_addresses_account_numbers_and_amounts() {
    let redacted = redact(
        "read /Users/owner/Library/Statements/DBS-July-2026.pdf for owner@example.com \
         account 1234567890 total 1,234.56 SGD token sk_live_51H8xQ2eZvKYlo2C",
    );
    for secret in [
        "/Users/owner",
        "DBS-July-2026.pdf",
        "owner@example.com",
        "1234567890",
        "1,234.56",
        "sk_live_51H8xQ2eZvKYlo2C",
    ] {
        assert!(!redacted.contains(secret), "{redacted}");
    }
    assert!(!redacted.contains('\n'));
}

#[test]
fn reopening_the_vault_keeps_entries_inside_the_retention_window() {
    // Retention runs on every open: reopening the encrypted Vault twice must
    // leave the retained entries in place and stay idempotent.
    let root = tempfile::tempdir().expect("temporary Vault");
    {
        let store = open_store(root.path());
        store
            .record_operational_log(&OperationalLogEntry::failure_code(
                "test.retained",
                "test_failure",
            ))
            .expect("record failure");
    }
    let first = ManualImportStore::open(root.path(), Zeroizing::new(KEY)).expect("reopen Vault");
    assert_eq!(
        first
            .operational_diagnostics_preview()
            .expect("preview")
            .entry_count,
        1
    );
    drop(first);
    let second = ManualImportStore::open(root.path(), Zeroizing::new(KEY)).expect("reopen again");
    assert_eq!(
        second
            .operational_diagnostics_preview()
            .expect("preview")
            .entry_count,
        1
    );
}
