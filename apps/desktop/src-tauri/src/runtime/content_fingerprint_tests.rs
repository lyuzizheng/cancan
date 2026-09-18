//! End-to-end coverage of the canonical-content duplicate short-circuit on the
//! local parse pipeline: byte-different artifacts of one statement must settle
//! their claimed job without reaching the normalizer, and content the Vault has
//! never parsed must keep flowing through it.

use super::tests::{
    structured_parse_test_state, synthetic_normalizer_result, synthetic_statement_csv,
};
use super::*;
use crate::content_fingerprint::canonical_content_fingerprint;
use crate::database::SourceDocumentRoutingStatus;

/// Captures and claims the queued parse job of one freshly imported document.
fn claim_imported_parse(runtime: &VaultRuntime, document_id: &str) -> ParseDocumentJob {
    runtime
        .queued_local_inbox_parse_documents()
        .expect("read queued parse jobs")
        .into_iter()
        .find(|job| job.document_id == document_id)
        .expect("queued parse job for the captured document")
}

#[test]
fn a_byte_different_reprint_settles_without_reaching_the_normalizer() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_path = parent.path().join("statement-july.csv");
    let reprint_path = parent.path().join("statement-july-reprint.csv");
    let statement = synthetic_statement_csv();
    fs::write(&statement_path, &statement).expect("write statement fixture");
    fs::write(&reprint_path, format!("{statement}\n\n")).expect("write reprint fixture");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .seed_money_source(
            "source-synthetic",
            "synthetic-bank",
            "Synthetic Bank",
            "bank",
        )
        .expect("seed source");

    // The statement the Vault already parsed: one trusted successful parse whose
    // records are reusable, driven exactly like the parse pump drives it.
    let statement_document = runtime
        .import_selected_document(&statement_path)
        .expect("capture statement");
    let statement_job = claim_imported_parse(&runtime, &statement_document.document_id);
    let statement_attempt = runtime
        .start_local_inbox_parse(&statement_job)
        .expect("claim statement parse job")
        .expect("statement parse job is claimable");
    let statement_input = runtime
        .normalization_input(&statement_document.document_id)
        .expect("extract statement");
    assert!(
        !tauri::async_runtime::block_on(settle_canonical_content(
            &runtime,
            &statement_attempt,
            &statement_input,
        ))
        .expect("settle canonical content"),
        "the first artifact of a statement has nothing to reuse",
    );
    assert_eq!(
        runtime
            .apply_normalizer_result_for_job(
                &statement_attempt.claim,
                &statement_input,
                synthetic_normalizer_result(),
            )
            .expect("apply trusted routing")
            .status,
        SourceDocumentRoutingStatus::Routed,
    );
    let parsed_statement = structured_parse_test_state(&runtime, &statement_document.document_id);
    assert!(
        parsed_statement.records > 0,
        "the statement's parse must produce reusable records",
    );
    let (statement_records, statement_staged_records) =
        (parsed_statement.records, parsed_statement.staged_records);

    // Same statement content, different bytes.
    let reprint = runtime
        .import_selected_document(&reprint_path)
        .expect("capture reprint");
    assert_ne!(statement_document.document_id, reprint.document_id);
    let job = claim_imported_parse(&runtime, &reprint.document_id);
    let attempt = runtime
        .start_local_inbox_parse(&job)
        .expect("claim reprint parse job")
        .expect("reprint parse job is claimable");
    let reprint_input = runtime
        .normalization_input(&reprint.document_id)
        .expect("extract reprint");
    assert_ne!(
        statement_input.file_sha256, reprint_input.file_sha256,
        "the reprint is a different byte sequence",
    );
    assert_eq!(
        canonical_content_fingerprint(&statement_input.bundle),
        canonical_content_fingerprint(&reprint_input.bundle),
        "the reprint carries the same canonical statement content",
    );

    let settled = tauri::async_runtime::block_on(settle_canonical_content(
        &runtime,
        &attempt,
        &reprint_input,
    ))
    .expect("settle canonical content");

    assert!(
        settled,
        "the reprint must settle instead of reaching the normalizer",
    );
    let reprint_state = structured_parse_test_state(&runtime, &reprint.document_id);
    assert_eq!(
        (reprint_state.records, reprint_state.parse_runs),
        (0, 0),
        "a reprint never runs its own parse or regenerates records",
    );
    assert_eq!(reprint_state.reconcile_status, None);
    assert!(
        !runtime
            .queued_document_reconciliations()
            .expect("read queued reconciliations")
            .contains(&reprint.document_id),
        "a reprint enqueues no reconciliation",
    );
    assert!(
        runtime
            .queued_local_inbox_parse_documents()
            .expect("read queued parse jobs")
            .iter()
            .all(|job| job.document_id != reprint.document_id),
        "the reprint parse job is finished",
    );

    // The statement keeps its own records, and the reprint appears in Tasks as
    // the same-content outcome rather than as an unclassified document.
    let statement_state = structured_parse_test_state(&runtime, &statement_document.document_id);
    assert_eq!(
        (
            statement_state.records,
            statement_state.staged_records,
            statement_state.parse_runs,
        ),
        (statement_records, statement_staged_records, 1),
        "a reprint neither duplicates nor removes the statement's records",
    );
    let guard = runtime.store().expect("store");
    let store = guard.as_ref().expect("unlocked store");
    let rows = store.derive_task_rows(true).expect("derive task rows");
    assert!(
        rows.iter().any(|row| matches!(
            row.kind,
            crate::database::tasks::RawTaskKind::SameStatementContent { .. }
        )),
        "Tasks must distinguish the same-content outcome",
    );
}

#[test]
fn content_the_vault_never_parsed_keeps_flowing_through_the_normalizer() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_path = parent.path().join("statement-july.csv");
    let other_path = parent.path().join("statement-august.csv");
    fs::write(&statement_path, synthetic_statement_csv()).expect("write statement fixture");
    fs::write(
        &other_path,
        synthetic_statement_csv().replace("750.00", "640.00"),
    )
    .expect("write revised statement fixture");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .seed_money_source(
            "source-synthetic",
            "synthetic-bank",
            "Synthetic Bank",
            "bank",
        )
        .expect("seed source");
    let statement_document = runtime
        .import_selected_document(&statement_path)
        .expect("capture statement");
    let statement_input = runtime
        .normalization_input(&statement_document.document_id)
        .expect("extract statement");
    runtime
        .apply_normalizer_result(
            &statement_document.document_id,
            &statement_input,
            synthetic_normalizer_result(),
        )
        .expect("apply trusted routing");

    let revised = runtime
        .import_selected_document(&other_path)
        .expect("capture revised statement");
    let job = claim_imported_parse(&runtime, &revised.document_id);
    let attempt = runtime
        .start_local_inbox_parse(&job)
        .expect("claim revised parse job")
        .expect("revised parse job is claimable");
    let revised_input = runtime
        .normalization_input(&revised.document_id)
        .expect("extract revised statement");
    assert_ne!(
        canonical_content_fingerprint(&statement_input.bundle),
        canonical_content_fingerprint(&revised_input.bundle),
        "a revised statement is not the same content",
    );

    let settled = tauri::async_runtime::block_on(settle_canonical_content(
        &runtime,
        &attempt,
        &revised_input,
    ))
    .expect("settle canonical content");

    assert!(
        !settled,
        "content the Vault has never parsed continues through the normalizer",
    );
    assert!(
        runtime
            .apply_normalizer_result_for_job(
                &attempt.claim,
                &revised_input,
                synthetic_normalizer_result()
            )
            .is_ok(),
        "the claimed job is still open for the normalizer",
    );
}
