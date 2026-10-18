use super::database_test_support::import_source;
use super::tests::{KEY, open_store};
use super::*;
use crate::diagnostics::{
    JobFailureDetail, MAX_REDACTED_CHARS, OPERATIONAL_LOG_RETENTION_DAYS, OperationalLogEntry,
    redact,
};
use std::io;
use zeroize::Zeroizing;

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
fn redaction_removes_dot_separated_tokens_and_national_id_style_identifiers() {
    // A JWT: three base64url segments whose long ones mix letters with digits.
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
               eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0.\
               SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let redacted = redact(&format!(
        "authorization failed for S1234567A with token {jwt} after 3 attempts"
    ));

    for secret in [jwt, "S1234567A"] {
        assert!(!redacted.contains(secret), "{redacted}");
    }
    assert!(redacted.contains("<token>"), "{redacted}");
    assert!(redacted.contains("<id>"), "{redacted}");

    // Dotted version and host text stays readable: over-redaction would make
    // the export useless without protecting anything.
    let versioned = redact("connector protocol 1.2.3 on docs.example.com");
    assert!(versioned.contains("1.2.3"), "{versioned}");
    assert!(versioned.contains("docs.example.com"), "{versioned}");
}

#[test]
fn redaction_removes_one_digit_amounts_and_every_decimal_scale() {
    // 888-F02: the whole part's first digit was never counted and the fraction
    // had to be exactly two digits, so `1.23` and `12.345` were written verbatim
    // into the stored row, the preview, and the export.
    for amount in [
        "1.23",
        "12.345",
        "0.000001",
        "0.1234567",
        "-7.5",
        "+0.25",
        "0.5",
        "1,234.567",
    ] {
        let redacted = redact(&format!("balance {amount} on the statement"));
        assert!(!redacted.contains(amount), "{amount}: {redacted}");
        assert!(redacted.contains("<amount>"), "{amount}: {redacted}");
        assert!(
            redacted.contains("on the statement"),
            "{amount}: {redacted}"
        );
    }

    // A scale long enough that the digits form a reference number on their own
    // is redacted through the number placeholder instead.
    let long_scale = redact("fund units 0.1234567890123456");
    assert!(!long_scale.contains("0.1234567890123456"), "{long_scale}");
    assert!(long_scale.ends_with("<number>"), "{long_scale}");
}

#[test]
fn redaction_removes_eight_digit_account_numbers_and_short_secrets() {
    // 888-F02: eight digits were below the nine-digit bar and `secret42` was
    // below the length bar of every rule, so both travelled verbatim.
    let redacted = redact("account 12345678 password secret42 reference 1234567890");
    for secret in ["12345678", "secret42", "1234567890"] {
        assert!(!redacted.contains(secret), "{redacted}");
    }
    assert!(redacted.contains("<number>"), "{redacted}");
    assert!(redacted.contains("<token>"), "{redacted}");
    assert!(redacted.contains("account"), "{redacted}");
    assert!(redacted.contains("password"), "{redacted}");
}

#[test]
fn redaction_keeps_versions_addresses_dates_and_counts_readable() {
    // Over-redaction would make the export useless without protecting
    // anything: a version, a host, an address, a date, and a retry count are
    // context the user needs to read the report.
    let context = redact(
        "connector protocol 1.2.3 on docs.example.com, host 192.168.1.1, \
         statement dated 2026-10-18 or 2026.10.18, retried 3 times",
    );
    for readable in [
        "1.2.3",
        "docs.example.com",
        "192.168.1.1",
        "2026-10-18",
        "2026.10.18",
        "retried 3 times",
    ] {
        assert!(context.contains(readable), "{readable} in {context}");
    }
}

#[test]
fn redaction_handles_unicode_and_the_maximum_length() {
    // Unicode prose stays and the values around it are redacted like ASCII
    // ones: the predicates never split a multibyte character, they just do not
    // recognise one as a digit.
    let redacted = redact("转账失败 账户 12345678 余额 1.23 口令 secret42");
    for secret in ["1.23", "12345678", "secret42"] {
        assert!(!redacted.contains(secret), "{redacted}");
    }
    assert!(redacted.contains("转账失败"), "{redacted}");

    // A multibyte word is context, not a value, even though its byte length is
    // above the credential bar.
    let multibyte = redact("口令 密码密码密码密码 12345678");
    assert!(multibyte.contains("密码密码密码密码"), "{multibyte}");
    assert!(!multibyte.contains("12345678"), "{multibyte}");

    // A value longer than the cap is replaced whole rather than truncated.
    let long_value = redact(&format!("1.{}", "0".repeat(400)));
    assert!(!long_value.contains('0'), "{long_value}");

    // Prose longer than the cap ends on a character boundary, not a byte one.
    let capped = redact(&"数据表".repeat(200));
    assert_eq!(capped.chars().count(), MAX_REDACTED_CHARS + 1);
    assert!(capped.ends_with('…'), "{capped}");
    assert!(
        capped.trim_end_matches('…').ends_with('据'),
        "the cap cuts between characters: {capped}"
    );
}

#[test]
fn the_q08_gap_values_never_reach_a_stored_row_a_preview_or_an_export() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store = open_store(root.path());
    let failure = io::Error::new(
        io::ErrorKind::PermissionDenied,
        "statement rejected at 08:15: balance 1.23 EUR, then 12.345 SGD, \
         account 12345678, password secret42, fund units 0.000001, \
         long scale 1.234567890123, 金额 9.87",
    );
    store
        .record_operational_log(&OperationalLogEntry::failure(
            "documents.store_prepared",
            "import_failed",
            &failure,
        ))
        .expect("record failure");
    let secrets = [
        "1.23",
        "12.345",
        "12345678",
        "secret42",
        "0.000001",
        "1.234567890123",
        "9.87",
    ];

    // The stored row: redaction runs at write, so the durable column never held
    // the values in the first place.
    let (detail, cause): (String, Option<String>) = store
        .connection
        .query_row(
            "SELECT detail, cause FROM operational_logs ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read the stored row");
    for secret in secrets {
        assert!(!detail.contains(secret), "{secret} in {detail}");
        if let Some(cause) = &cause {
            assert!(!cause.contains(secret), "{secret} in {cause}");
        }
    }
    assert!(detail.contains("<amount>"), "{detail}");
    assert!(detail.contains("<number>"), "{detail}");
    assert!(detail.contains("<token>"), "{detail}");
    assert!(detail.contains("statement rejected"), "{detail}");
    assert!(detail.contains("08:15"), "{detail}");

    // The preview and the export carry the same redacted lines.
    let preview = store
        .operational_diagnostics_preview()
        .expect("preview recorded failures");
    let sample = preview.sample_lines.join("\n");
    let export = store
        .operational_diagnostics_export()
        .expect("export the diagnostics");
    for secret in secrets {
        assert!(!sample.contains(secret), "{secret} in {sample}");
        assert!(!export.contains(secret), "{secret} in {export}");
    }
    assert!(sample.contains("import_failed"), "{sample}");
    assert!(sample.contains("statement rejected"), "{sample}");
    assert!(export.contains("amounts, filenames"), "{export}");
    assert!(export.contains("statement rejected"), "{export}");
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

#[test]
fn a_failure_reported_from_the_real_error_keeps_the_reason_and_retryability() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF transient transport failure").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import_source(&source_path, "document-transport", "audit-transport"),
            None,
        )
        .expect("import source");
    // The runtime attaches the error where it happens; the job row and the log
    // entry have to keep it instead of restating the static code.
    let detail = JobFailureDetail::from_error(
        Some("normalizer"),
        &io::Error::new(
            io::ErrorKind::TimedOut,
            "the normalizer produced no result before it timed out",
        ),
    );

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
    }

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
    assert_eq!(error_json["technical"]["errorKind"], "io");
    assert_eq!(error_json["technical"]["retryable"], true);
    assert_eq!(
        error_json["technical"]["message"],
        "the normalizer produced no result before it timed out"
    );

    let (detail, error_kind, error_code): (String, Option<String>, String) = store
        .connection
        .query_row(
            "SELECT detail, error_kind, error_code FROM operational_logs \
             WHERE component = 'job.parse_document' ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read the job entry");
    assert_eq!(error_kind.as_deref(), Some("io"));
    assert_eq!(error_code, "normalizer_failed");
    assert_eq!(
        detail, "the normalizer produced no result before it timed out",
        "the log keeps the reported reason, not the static code"
    );
}
#[test]
fn failed_reconcile_keeps_the_context_of_the_running_job_not_a_superseded_one() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF reconcile context").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import_source(
                &source_path,
                "document-reconcile-context",
                "audit-reconcile-context",
            ),
            None,
        )
        .expect("import source");
    // A superseded run of the same document is still in the table and wins
    // every scan order an unfiltered read could pick — it is inserted first,
    // its id sorts first, and `failed` sorts before `running` in the claim
    // index — so the context read must filter, not hope.
    store
        .connection
        .execute(
            "INSERT INTO jobs( \
               id, job_type, status, input_json, error_json, related_source_document_id, \
               attempts, max_attempts, created_at \
             ) VALUES ( \
               'job-reconcile-old', 'reconcile_document', 'failed', '{}', \
               '{\"errorCode\":\"provider_unavailable\"}', \
               'document-reconcile-context', 1, 3, datetime('now', '-1 hour') \
             )",
            [],
        )
        .expect("seed superseded reconcile job");
    store
        .connection
        .execute(
            "INSERT INTO jobs( \
               id, job_type, status, input_json, related_source_document_id, \
               related_money_source_id, lease_owner, lease_until, attempts, max_attempts \
             ) VALUES ( \
               'job-reconcile-running', 'reconcile_document', 'running', '{}', \
               'document-reconcile-context', 'source-dbs', 'reconcile-document', \
               datetime('now', '+5 minutes'), 2, 3 \
             )",
            [],
        )
        .expect("seed running reconcile job");

    store
        .fail_reconcile_document(
            "document-reconcile-context",
            "provider_unavailable",
            Some(&JobFailureDetail::from_code(
                Some("provider"),
                "provider_unavailable",
            )),
        )
        .expect("record reconcile failure");

    let error_json: String = store
        .connection
        .query_row(
            "SELECT error_json FROM jobs WHERE id = 'job-reconcile-running'",
            [],
            |row| row.get(0),
        )
        .expect("read the failed job error_json");
    let error_json: serde_json::Value =
        serde_json::from_str(&error_json).expect("error_json is an object");
    assert_eq!(error_json["errorCode"], "provider_unavailable");
    assert_eq!(error_json["technical"]["attempt"], 2);
    assert_eq!(error_json["technical"]["maxAttempts"], 3);
    assert_eq!(error_json["technical"]["jobType"], "reconcile_document");
    assert_eq!(
        error_json["technical"]["related"]["moneySourceId"],
        "source-dbs"
    );
    assert_eq!(
        error_json["technical"]["related"]["sourceDocumentId"],
        "document-reconcile-context"
    );

    // The superseded run keeps its own failure, and the log entry belongs to
    // the job the failure was recorded against.
    let superseded: Option<String> = store
        .connection
        .query_row(
            "SELECT error_json FROM jobs WHERE id = 'job-reconcile-old'",
            [],
            |row| row.get(0),
        )
        .expect("read the superseded job");
    assert_eq!(
        superseded.as_deref(),
        Some("{\"errorCode\":\"provider_unavailable\"}")
    );
    let (attempt, job_status): (i64, String) = store
        .connection
        .query_row(
            "SELECT attempt, job_status FROM operational_logs \
             WHERE component = 'job.reconcile_document' ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read the reconcile failure entry");
    assert_eq!(attempt, 2);
    assert_eq!(job_status, "running");
}

#[test]
fn a_deterministic_wire_rejection_fails_terminally_on_the_first_attempt() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF deterministic sidecar rejection").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import_source(
                &source_path,
                "document-deterministic",
                "audit-deterministic",
            ),
            None,
        )
        .expect("import source");
    let job = store
        .queued_parse_document_jobs()
        .expect("read queued parse job")
        .pop()
        .expect("queued parse job");
    let claim = store
        .start_parse_document_job(&job)
        .expect("start parse job")
        .expect("claim parse job");
    let detail = JobFailureDetail::from_reported_code(
        Some("normalizer"),
        "the normalizer reported evidence_grounding_failed",
        false,
        Some("evidence_grounding_failed"),
    );
    store
        .fail_parse_document_job(&claim, "normalizer_failed", Some(&detail))
        .expect("record deterministic failure");

    let state: (String, i64, String, String) = store
        .connection
        .query_row(
            "SELECT status, attempts, error_json, blocked_reason FROM jobs \
             WHERE id = ?1",
            [&job.job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read terminal parse state");
    assert_eq!(state.0, "failed");
    assert_eq!(
        state.1, 1,
        "a deterministic rejection must not consume retries"
    );
    assert_eq!(state.3, "normalizer_failed");
    let error_json: serde_json::Value =
        serde_json::from_str(&state.2).expect("error_json is an object");
    assert_eq!(error_json["errorCode"], "normalizer_failed");
    assert_eq!(
        error_json["technical"]["reportedCode"],
        "evidence_grounding_failed"
    );
    assert_eq!(error_json["technical"]["retryable"], false);
    assert_eq!(error_json["technical"]["attempt"], 1);

    let (level, error_code): (String, String) = store
        .connection
        .query_row(
            "SELECT level, error_code FROM operational_logs \
             WHERE component = 'job.parse_document' ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read the job entry");
    assert_eq!(
        level, "error",
        "a terminal first attempt logs at error, not warning"
    );
    assert_eq!(error_code, "normalizer_failed");

    assert!(
        store
            .queued_parse_document_jobs()
            .expect("read queue after terminal failure")
            .is_empty(),
        "a deterministic rejection must not requeue"
    );
}

#[test]
fn a_transient_wire_code_keeps_the_attempt_retry() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF transient wire failure").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import_source(&source_path, "document-transient", "audit-transient"),
            None,
        )
        .expect("import source");
    let job = store
        .queued_parse_document_jobs()
        .expect("read queued parse job")
        .pop()
        .expect("queued parse job");
    let claim = store
        .start_parse_document_job(&job)
        .expect("start parse job")
        .expect("claim parse job");
    let detail = JobFailureDetail::from_reported_code(
        Some("normalizer"),
        "the normalizer reported command_failed",
        true,
        None,
    );
    store
        .fail_parse_document_job(&claim, "normalizer_failed", Some(&detail))
        .expect("record transient failure");

    let state: (String, i64) = store
        .connection
        .query_row(
            "SELECT status, attempts FROM jobs WHERE id = ?1",
            [&job.job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read requeued parse state");
    assert_eq!(state, ("queued".to_owned(), 1));
}

#[test]
fn a_vault_whose_operational_log_is_unreadable_still_opens() {
    let root = tempfile::tempdir().expect("temporary Vault");
    {
        let store = open_store(root.path());
        store
            .connection
            .execute("DROP TABLE operational_logs", [])
            .expect("corrupt the log table");
    }

    // Retention is best-effort: the log is not worth locking the user out of
    // their evidence, so the Vault opens and still serves the financial tables.
    let reopened = ManualImportStore::open_existing(root.path(), Zeroizing::new(KEY))
        .expect("a broken operational log must not block the Vault");
    assert_eq!(
        reopened.list_money_sources().expect("read sources").len(),
        1
    );
}
