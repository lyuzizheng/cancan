use super::tasks_test_support::{import, runtime_fixture, write_source};
use super::*;
use crate::diagnostics::JobFailureDetail;

#[test]
fn a_failed_store_step_keeps_the_user_facing_code_and_logs_the_discarded_detail() {
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
        "statement.csv",
        b"date,amount\n2026-07-01,20.00\n",
    );
    let imported = import(&runtime, &source);
    let job = runtime
        .queued_local_inbox_parse_documents()
        .expect("read queued parse jobs")
        .pop()
        .expect("queued parse job");
    let attempt = runtime
        .start_local_inbox_parse(&job)
        .expect("start parse")
        .expect("claim parse");
    let detail = JobFailureDetail::from_code(Some("normalizer"), "normalizer_unavailable");
    runtime
        .fail_local_inbox_parse(&attempt, "normalizer_failed", Some(&detail))
        .expect("record parse failure");

    // The claim is spent, so this store step fails. The renderer still gets the
    // static code the command contract already publishes, and the store error
    // it replaced is recorded instead of dropped.
    let code = runtime
        .fail_local_inbox_parse(&attempt, "normalizer_failed", Some(&detail))
        .expect_err("a spent claim cannot fail again")
        .code;
    assert_eq!(code, "local_inbox_parse_failed");

    let preview = runtime
        .operational_diagnostics_preview()
        .expect("preview recorded failures");
    assert!(
        preview
            .error_codes
            .iter()
            .any(|category| category.label == "local_inbox_parse_failed"),
        "{:?}",
        preview.error_codes
    );
    let lines = preview.sample_lines.join("\n");
    assert!(lines.contains("fail_parse_document_job"), "{lines}");
    assert!(lines.contains("parse job is no longer claimed"), "{lines}");
    assert_eq!(imported.document_id, attempt.claim.document_id);
}

#[test]
fn an_import_failure_records_the_real_reason_not_only_the_static_code() {
    let (parent, runtime) = runtime_fixture();
    let missing = parent.path().join("statement-that-is-gone.csv");

    let code = runtime
        .import_selected_document(&missing)
        .expect_err("a missing file cannot be imported")
        .code;
    assert_eq!(code, "import_failed");

    // The renderer only ever sees the static code; the log has to explain it.
    let preview = runtime
        .operational_diagnostics_preview()
        .expect("preview recorded failures");
    let lines = preview.sample_lines.join("\n");
    assert!(lines.contains("source_document_metadata"), "{lines}");
    assert!(
        !lines.contains("statement-that-is-gone.csv"),
        "the source filename never reaches the log: {lines}"
    );
    assert!(lines.contains("No such file"), "{lines}");
}

#[test]
fn saving_diagnostics_writes_the_redacted_export_and_reports_an_unwritable_target() {
    let (parent, runtime) = runtime_fixture();
    let unavailable = parent
        .path()
        .join("missing-directory")
        .join("CanCan Diagnostics.txt");
    let code = runtime
        .save_operational_diagnostics(&unavailable)
        .expect_err("an unwritable destination fails")
        .code;
    assert_eq!(code, "diagnostics_export_failed");

    let destination = parent.path().join("CanCan Diagnostics.txt");
    runtime
        .save_operational_diagnostics(&destination)
        .expect("write export");
    let export = fs::read_to_string(&destination).expect("read export");
    assert!(export.contains("CanCan operational diagnostics export"));
    assert!(export.contains("diagnostics_export_failed"), "{export}");
    assert!(
        export.contains("operational_diagnostics_export"),
        "the export is described in the export itself"
    );

    // Both the failed write and its own record stay visible to the preview.
    let preview = runtime
        .operational_diagnostics_preview()
        .expect("preview recorded failures");
    assert!(
        preview
            .error_codes
            .iter()
            .any(|category| category.label == "diagnostics_export_failed"),
        "{:?}",
        preview.error_codes
    );
}

#[test]
fn a_failure_from_a_scope_that_held_the_store_guard_still_reaches_the_log() {
    // The recovery file is written while the store guard is held, so its entry
    // is written once the guard is released — and it carries the real IO error
    // instead of repeating the static code.
    let (parent, runtime) = runtime_fixture();
    let code = runtime
        .save_recovery_file(
            &parent
                .path()
                .join("missing")
                .join("recovery.cancan-recovery"),
        )
        .expect_err("reject unavailable destination")
        .code;
    assert_eq!(code, "recovery_save_failed");

    let preview = runtime
        .operational_diagnostics_preview()
        .expect("preview recorded failures");
    let lines = preview.sample_lines.join(
        "
",
    );
    assert!(lines.contains("recovery_destination"), "{lines}");
    assert!(lines.contains("recovery_save_failed"), "{lines}");
    assert!(lines.contains("No such file"), "{lines}");
}
