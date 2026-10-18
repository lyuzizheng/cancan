//! Confirmed Money Source ownership for statement-password saves.
//!
//! Spec 0015/0017 allow a password to be saved or replaced only for the Money
//! Source trusted classification confirmed on the document itself. The tests
//! use the in-memory Keychain fake and synthetic statement bytes only.
#![cfg(target_os = "macos")]

use super::tasks_test_support::classify_document;
use super::test_support::{
    MemoryStatementPasswordStore, statement_password_runtime, statement_password_state,
};
use super::tests::{protected_text_pdf, synthetic_pdf_normalizer_result};
use super::*;
use crate::database::SourceDocumentRoutingStatus;
use std::{fs, sync::Arc};

fn protected_statement(runtime: &VaultRuntime, parent: &Path) -> String {
    let source = parent.join("protected-statement.pdf");
    fs::write(&source, protected_text_pdf()).expect("write protected PDF fixture");
    runtime
        .import_selected_document(&source)
        .expect("import protected statement")
        .document_id
}

fn password_context(runtime: &VaultRuntime, document_id: &str) -> StatementPasswordUnlockContext {
    runtime
        .statement_password_document_context(document_id)
        .expect("read statement password context")
}

fn confirm_owner(runtime: &VaultRuntime, document_id: &str) {
    let mut store = runtime.store().expect("open store");
    let store = store.as_mut().expect("unlocked store");
    assert_eq!(
        classify_document(store, document_id).status,
        SourceDocumentRoutingStatus::Routed
    );
}

#[test]
fn reports_no_owner_until_trusted_classification_and_keeps_session_only_unlock() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());
    let document_id = protected_statement(&runtime, parent.path());

    assert_eq!(
        password_context(&runtime, &document_id),
        StatementPasswordUnlockContext {
            has_saved_password: false,
            money_source_display_name: None,
            money_source_id: None,
        }
    );
    assert_eq!(
        runtime
            .unlock_source_document(
                &document_id,
                Some("source-dbs"),
                b"statement-password",
                true,
            )
            .expect_err("an unconfirmed document must not save a password")
            .code(),
        "statement_password_source_unconfirmed"
    );
    assert_eq!(
        runtime
            .unlock_source_document(&document_id, None, b"statement-password", true)
            .expect_err("a save without a confirmed target is unconfirmed")
            .code(),
        "statement_password_source_unconfirmed"
    );
    assert!(
        statement_passwords
            .load("money-source:source-dbs")
            .expect("load statement password")
            .is_none(),
        "a rejected save writes no Keychain entry"
    );

    // Session-only unlock needs no owner and stores nothing.
    runtime
        .unlock_source_document(&document_id, None, b"statement-password", false)
        .expect("session-only unlock");
    let bundle = runtime
        .normalization_input(&document_id)
        .expect("extract the session-unlocked statement");
    assert!(!bundle.observations.is_empty());
    assert!(
        statement_passwords
            .load("money-source:source-dbs")
            .expect("load statement password")
            .is_none()
    );

    // The session password does not survive a Vault lock.
    runtime.test_support_lock().expect("lock Vault");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("reopen Vault");
    assert_eq!(
        runtime
            .normalization_input(&document_id)
            .expect_err("session password is gone")
            .code(),
        "statement_password_required"
    );

    // Trusted classification confirms the owner; the context now names it.
    runtime
        .seed_money_source(
            "source-synthetic",
            "synthetic-bank",
            "Synthetic Bank",
            "bank",
        )
        .expect("seed routing source");
    let routed = runtime
        .apply_normalizer_result(&document_id, &bundle, synthetic_pdf_normalizer_result())
        .expect("route protected text-layer PDF");
    assert_eq!(routed.status, SourceDocumentRoutingStatus::Routed);
    assert_eq!(
        password_context(&runtime, &document_id),
        StatementPasswordUnlockContext {
            has_saved_password: false,
            money_source_display_name: Some("Synthetic Bank".to_owned()),
            money_source_id: Some("source-synthetic".to_owned()),
        }
    );
    assert_eq!(
        serde_json::to_value(password_context(&runtime, &document_id))
            .expect("serialize presentation-safe context"),
        serde_json::json!({
            "hasSavedPassword": false,
            "moneySourceDisplayName": "Synthetic Bank",
            "moneySourceId": "source-synthetic",
        })
    );
}

#[test]
fn rejects_mismatched_and_wrong_passwords_and_replaces_only_the_owned_secret() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());
    runtime
        .seed_money_source("source-card", "dbs-card", "DBS Card", "credit_card")
        .expect("seed card source");
    let document_id = protected_statement(&runtime, parent.path());
    confirm_owner(&runtime, &document_id);

    // A previous secret already exists for the confirmed source.
    runtime
        .save_statement_password("source-dbs", b"previous-password")
        .expect("seed previous statement password");
    assert_eq!(
        statement_password_state(&runtime)
            .expect("previous saved state")
            .status,
        StatementPasswordStatus::Saved
    );

    assert_eq!(
        runtime
            .unlock_source_document(
                &document_id,
                Some("source-card"),
                b"statement-password",
                true,
            )
            .expect_err("a document owned by another source must not save")
            .code(),
        "statement_password_source_mismatch"
    );
    assert_eq!(
        statement_passwords
            .load("money-source:source-dbs")
            .expect("load previous secret")
            .expect("previous secret survives")
            .as_slice(),
        b"previous-password"
    );
    assert!(
        statement_passwords
            .load("money-source:source-card")
            .expect("load mismatched secret")
            .is_none()
    );

    assert_eq!(
        runtime
            .unlock_source_document(&document_id, Some("source-dbs"), b"wrong-password", true,)
            .expect_err("a wrong password must not replace the secret")
            .code(),
        "statement_password_invalid"
    );
    assert_eq!(
        statement_passwords
            .load("money-source:source-dbs")
            .expect("load previous secret")
            .expect("previous secret survives")
            .as_slice(),
        b"previous-password"
    );

    // The verified password replaces the previous secret for the owner.
    runtime
        .unlock_source_document(
            &document_id,
            Some("source-dbs"),
            b"statement-password",
            true,
        )
        .expect("replace the confirmed source password");
    assert_eq!(
        statement_passwords
            .load("money-source:source-dbs")
            .expect("load replaced secret")
            .expect("replaced secret exists")
            .as_slice(),
        b"statement-password"
    );
    assert!(password_context(&runtime, &document_id).has_saved_password);

    // A later session unlocks through the saved secret.
    runtime.test_support_lock().expect("lock Vault");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("reopen Vault");
    assert_eq!(
        runtime
            .try_saved_statement_passwords(&document_id)
            .expect("try the saved password"),
        SavedStatementPasswordResult::Unlocked
    );

    statement_passwords
        .delete("money-source:source-dbs")
        .expect("remove the device-local secret");
    assert_eq!(
        runtime
            .try_saved_statement_passwords(&document_id)
            .expect("report unavailable saved password"),
        SavedStatementPasswordResult::Unavailable
    );
    assert_eq!(
        statement_password_state(&runtime)
            .expect("preserve the saved reference")
            .status,
        StatementPasswordStatus::Saved
    );
    statement_passwords
        .save("money-source:source-dbs", b"wrong-password")
        .expect("save invalid password fixture");
    assert_eq!(
        runtime
            .try_saved_statement_passwords(&document_id)
            .expect("report invalid saved password"),
        SavedStatementPasswordResult::Invalid
    );
}

#[test]
fn rejects_a_stale_document_and_keeps_the_previous_secret() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());
    let document_id = protected_statement(&runtime, parent.path());
    confirm_owner(&runtime, &document_id);
    runtime
        .save_statement_password("source-dbs", b"previous-password")
        .expect("seed previous statement password");

    runtime
        .delete_source_document(&document_id)
        .expect("tombstone the document");

    assert_eq!(
        runtime
            .unlock_source_document(
                &document_id,
                Some("source-dbs"),
                b"statement-password",
                true,
            )
            .expect_err("a deleted document cannot certify a save")
            .code(),
        "document_unavailable"
    );
    assert_eq!(
        statement_passwords
            .load("money-source:source-dbs")
            .expect("load previous secret")
            .expect("previous secret survives")
            .as_slice(),
        b"previous-password"
    );
    assert_eq!(
        statement_password_state(&runtime)
            .expect("preserve the saved reference")
            .status,
        StatementPasswordStatus::Saved
    );
}
