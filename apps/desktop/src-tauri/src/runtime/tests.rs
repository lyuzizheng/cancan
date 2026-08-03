use super::test_support::{statement_password_runtime, statement_password_state};
use super::*;
use crate::database::{CandidateAccountDecision, SourceDocumentImportStatus};
use crate::vault::open_recovery_file;
#[cfg(target_os = "macos")]
use crate::viewer::tests::{protected_pdf_fixture, synthetic_png_fixture};
use std::{collections::HashMap, thread, time::Duration};

#[derive(Default)]
pub(super) struct MemoryRememberedKeyStore {
    secret: Mutex<Option<Vec<u8>>>,
}

impl RememberedKeyStore for MemoryRememberedKeyStore {
    fn delete(&self) -> Result<(), ()> {
        *self.secret.lock().map_err(|_| ())? = None;
        Ok(())
    }

    fn is_present(&self) -> Result<bool, ()> {
        Ok(self.secret.lock().map_err(|_| ())?.is_some())
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        Ok(self
            .secret
            .lock()
            .map_err(|_| ())?
            .clone()
            .map(Zeroizing::new))
    }

    fn save(&self, secret: &[u8]) -> Result<(), ()> {
        *self.secret.lock().map_err(|_| ())? = Some(secret.to_vec());
        Ok(())
    }
}

struct FailingRememberedKeyStore;

impl RememberedKeyStore for FailingRememberedKeyStore {
    fn delete(&self) -> Result<(), ()> {
        Err(())
    }

    fn is_present(&self) -> Result<bool, ()> {
        Err(())
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        Ok(None)
    }

    fn save(&self, _secret: &[u8]) -> Result<(), ()> {
        Err(())
    }
}

struct PresenceOnlyRememberedKeyStore;

impl RememberedKeyStore for PresenceOnlyRememberedKeyStore {
    fn delete(&self) -> Result<(), ()> {
        Err(())
    }

    fn is_present(&self) -> Result<bool, ()> {
        Ok(true)
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        panic!("status must not load the remembered secret")
    }

    fn save(&self, _secret: &[u8]) -> Result<(), ()> {
        Err(())
    }
}

struct MalformedDeleteFailingRememberedKeyStore;

impl RememberedKeyStore for MalformedDeleteFailingRememberedKeyStore {
    fn delete(&self) -> Result<(), ()> {
        Err(())
    }

    fn is_present(&self) -> Result<bool, ()> {
        Ok(true)
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        Ok(Some(Zeroizing::new(vec![0x55; KEY_LEN - 1])))
    }

    fn save(&self, _secret: &[u8]) -> Result<(), ()> {
        Err(())
    }
}

#[derive(Default)]
pub(super) struct MemoryStatementPasswordStore {
    fail_delete: AtomicBool,
    fail_save: AtomicBool,
    secrets: Mutex<HashMap<String, Vec<u8>>>,
}

impl StatementPasswordStore for MemoryStatementPasswordStore {
    fn delete(&self, secret_ref: &str) -> Result<(), ()> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(());
        }
        self.secrets.lock().map_err(|_| ())?.remove(secret_ref);
        Ok(())
    }

    fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        Ok(self
            .secrets
            .lock()
            .map_err(|_| ())?
            .get(secret_ref)
            .cloned()
            .map(Zeroizing::new))
    }

    fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), ()> {
        if self.fail_save.load(Ordering::SeqCst) {
            return Err(());
        }
        self.secrets
            .lock()
            .map_err(|_| ())?
            .insert(secret_ref.to_owned(), secret.to_vec());
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct MemoryLocalInboxBookmarkStore {
    bookmark: Mutex<Option<Vec<u8>>>,
}

impl LocalInboxBookmarkStore for MemoryLocalInboxBookmarkStore {
    fn delete(&self) -> Result<(), ()> {
        *self.bookmark.lock().map_err(|_| ())? = None;
        Ok(())
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        Ok(self
            .bookmark
            .lock()
            .map_err(|_| ())?
            .clone()
            .map(Zeroizing::new))
    }

    fn save(&self, bookmark: &[u8]) -> Result<(), ()> {
        *self.bookmark.lock().map_err(|_| ())? = Some(bookmark.to_vec());
        Ok(())
    }
}

#[test]
fn refuses_an_ambiguous_or_mismatched_core_relationship_candidate() {
    assert!(is_unique_requested_relationship_candidate(
        &[CoreCandidate {
            id: "record-dbs-card".to_owned(),
        }],
        "record-dbs-card",
    ));
    assert!(!is_unique_requested_relationship_candidate(
        &[
            CoreCandidate {
                id: "record-dbs-card".to_owned(),
            },
            CoreCandidate {
                id: "record-other-card".to_owned(),
            },
        ],
        "record-dbs-card",
    ));
    assert!(!is_unique_requested_relationship_candidate(
        &[CoreCandidate {
            id: "record-other-card".to_owned(),
        }],
        "record-dbs-card",
    ));
}

#[test]
fn local_inbox_bookmark_is_paused_while_locked_and_disable_removes_it() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let bookmarks = Arc::new(MemoryLocalInboxBookmarkStore::default());
    let runtime = VaultRuntime::with_secret_stores(
        parent.path().join("vault"),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        bookmarks.clone(),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    bookmarks
        .save(b"security-scoped-bookmark")
        .expect("save bookmark");
    runtime.lock().expect("lock Vault");

    let paused = runtime.local_inbox_status().expect("paused status");
    assert_eq!(paused.access_state, LocalInboxAccessState::Paused);
    assert!(paused.enabled);
    assert!(!paused.backups_prepared);

    let disabled = runtime.disable_local_inbox().expect("disable Inbox");
    assert_eq!(disabled.access_state, LocalInboxAccessState::Disabled);
    assert!(!disabled.enabled);
    assert!(bookmarks.load().expect("load bookmark").is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn protects_pdf_passwords_inside_the_unlocked_vault_session() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());
    let source = parent.path().join("protected-statement.pdf");
    fs::write(&source, protected_text_pdf()).expect("write protected PDF fixture");
    let outcome = runtime
        .import_selected_document(&source)
        .expect("import protected statement");

    let documents = runtime
        .list_unassigned_source_documents()
        .expect("list protected statement");
    assert_eq!(documents[0].document_id, outcome.document_id);
    assert_eq!(
        documents[0].document_status,
        SourceDocumentStatus::Processing
    );
    assert_eq!(
        runtime
            .normalization_input(&outcome.document_id)
            .expect_err("protected PDF must not normalize before unlock")
            .code(),
        "statement_password_required"
    );
    assert_eq!(
        runtime
            .render_source_document_page(&outcome.document_id, 1)
            .expect_err("locked PDF must not render")
            .code(),
        "statement_password_required"
    );
    assert_eq!(
        runtime
            .unlock_source_document(&outcome.document_id, "source-dbs", b"wrong-password", true,)
            .expect_err("wrong password must not save")
            .code(),
        "statement_password_invalid"
    );
    assert_eq!(statement_password_state(&runtime), None);

    runtime
        .unlock_source_document(
            &outcome.document_id,
            "source-dbs",
            b"statement-password",
            false,
        )
        .expect("use password once");
    assert_eq!(
        runtime
            .list_unassigned_source_documents()
            .expect("list session-unlocked statement")[0]
            .document_status,
        SourceDocumentStatus::Processing
    );
    let bundle = runtime
        .normalization_input(&outcome.document_id)
        .expect("extract session-unlocked protected PDF");
    assert_eq!(bundle.mime_type, "application/pdf");
    assert_eq!(bundle.observations.len(), 1);
    assert!(bundle.observations[0].text.contains("synthetic-bank"));
    assert!(bundle.observations[0].text.contains("transfer-2026-07"));
    assert!(bundle.observations[0].text.contains("Café"));
    assert_eq!(
        bundle.observations[0]
            .text_span
            .as_ref()
            .map(|span| span.end),
        Some(bundle.observations[0].text.encode_utf16().count() as u64)
    );
    runtime
        .render_source_document_page(&outcome.document_id, 1)
        .expect("render session-unlocked statement");
    runtime.request_system_lock().expect("system-lock Vault");
    runtime
        .resume_system_session()
        .expect("resume system session");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("reopen system-locked Vault");
    assert_eq!(
        runtime
            .list_unassigned_source_documents()
            .expect("list after session lock")[0]
            .document_status,
        SourceDocumentStatus::Processing
    );

    runtime
        .unlock_source_document(
            &outcome.document_id,
            "source-dbs",
            b"statement-password",
            true,
        )
        .expect("verify and save password");
    assert_eq!(
        statement_passwords
            .load("money-source:source-dbs")
            .expect("load saved password")
            .expect("saved password exists")
            .as_slice(),
        b"statement-password"
    );
    runtime.lock().expect("lock saved-password session");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("reopen saved-password Vault");
    assert_eq!(
        runtime
            .try_saved_statement_password(&outcome.document_id, "source-dbs")
            .expect("try saved password"),
        SavedStatementPasswordResult::Unlocked
    );
    assert_eq!(
        runtime
            .list_unassigned_source_documents()
            .expect("list saved-password statement")[0]
            .document_status,
        SourceDocumentStatus::Processing
    );

    statement_passwords
        .delete("money-source:source-dbs")
        .expect("remove device-local saved password");
    assert_eq!(
        runtime
            .try_saved_statement_password(&outcome.document_id, "source-dbs")
            .expect("report unavailable saved password"),
        SavedStatementPasswordResult::Unavailable
    );
    assert_eq!(
        statement_password_state(&runtime)
            .expect("preserve saved-password reference")
            .status,
        StatementPasswordStatus::Saved
    );
    statement_passwords
        .save("money-source:source-dbs", b"wrong-password")
        .expect("save invalid password fixture");
    assert_eq!(
        runtime
            .try_saved_statement_password(&outcome.document_id, "source-dbs")
            .expect("report invalid saved password"),
        SavedStatementPasswordResult::Invalid
    );
    runtime
        .seed_money_source(
            "source-synthetic",
            "synthetic-bank",
            "Synthetic Bank",
            "bank",
        )
        .expect("seed routing source");
    let routed = runtime
        .apply_normalizer_result(
            &outcome.document_id,
            &bundle,
            synthetic_pdf_normalizer_result(),
        )
        .expect("route protected text-layer PDF");
    assert_eq!(
        routed.status,
        crate::database::SourceDocumentRoutingStatus::Routed
    );
    assert_eq!(routed.money_source_id.as_deref(), Some("source-synthetic"));

    let protected_copy = parent.path().join("protected-copy.pdf");
    let (_, copy_generation) = runtime
        .source_document_copy_context(&outcome.document_id)
        .expect("read protected copy context");
    runtime
        .save_source_document_copy(&outcome.document_id, &protected_copy, copy_generation)
        .expect("save protected source copy");
    let copied_bytes = fs::read(&protected_copy).expect("read protected source copy");
    assert_eq!(
        copied_bytes,
        fs::read(&source).expect("read original protected source")
    );
    assert_eq!(
        pdf_access(&copied_bytes, None).expect("inspect protected source copy"),
        PdfAccess::PasswordRequired
    );

    let files_directory = parent.path().join("vault").join("files");
    let original_permissions = fs::metadata(&files_directory)
        .expect("read files directory metadata")
        .permissions();
    fs::set_permissions(&files_directory, fs::Permissions::from_mode(0o500))
        .expect("make encrypted files directory read-only");
    let deletion = runtime.delete_source_document(&outcome.document_id);
    fs::set_permissions(&files_directory, original_permissions)
        .expect("restore files directory permissions");
    assert_eq!(
        deletion
            .expect_err("surface encrypted-blob cleanup failure")
            .code(),
        "delete_source_failed"
    );
    assert!(
        !runtime
            .document_passwords()
            .expect("document password cache")
            .contains_key(&outcome.document_id),
        "deletion must clear the document password even when blob cleanup fails"
    );
    assert_eq!(
        runtime
            .source_document_copy_context(&outcome.document_id)
            .expect_err("deleted source must not open an export dialog")
            .code(),
        "document_unavailable"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn fails_closed_when_a_pdf_cannot_be_inspected() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let source = parent.path().join("corrupt.pdf");
    fs::write(
        &source,
        b"%PDF corrupt\nCANCAN_SYNTHETIC_STATEMENT_V1\nprovider=synthetic-bank\nstatement_id=transfer-2026-07",
    )
    .expect("write corrupt PDF fixture");
    let outcome = runtime
        .import_selected_document(&source)
        .expect("import corrupt PDF");

    assert_eq!(
        runtime
            .list_unassigned_source_documents()
            .expect("list corrupt PDF")[0]
            .document_status,
        SourceDocumentStatus::Processing
    );
    assert_eq!(
        runtime
            .normalization_input(&outcome.document_id)
            .expect_err("corrupt PDF must not reach normalization")
            .code(),
        "document_render_failed"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn keeps_listing_other_documents_when_one_encrypted_blob_is_unreadable() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::new(vault_root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");

    let pdf = parent.path().join("protected-statement.pdf");
    fs::write(&pdf, protected_pdf_fixture()).expect("write protected PDF fixture");
    let pdf_outcome = runtime
        .import_selected_document(&pdf)
        .expect("import protected statement");
    let csv = parent.path().join("transactions.csv");
    fs::write(&csv, b"date,amount\n2026-07-01,10.00\n").expect("write CSV fixture");
    let csv_outcome = runtime.import_selected_document(&csv).expect("import CSV");

    let encrypted_locator = {
        let store = runtime.store().expect("active store");
        store
            .as_ref()
            .expect("unlocked store")
            .list_unassigned_documents()
            .expect("list imported documents")
            .into_iter()
            .find(|document| document.document_id == pdf_outcome.document_id)
            .and_then(|document| document.encrypted_locator)
            .expect("protected PDF encrypted locator")
    };
    fs::remove_file(vault_root.join(encrypted_locator))
        .expect("remove protected PDF encrypted blob");

    let documents = runtime
        .list_unassigned_source_documents()
        .expect("list remaining documents");
    assert_eq!(documents.len(), 2);
    assert_eq!(
        documents
            .iter()
            .find(|document| document.document_id == pdf_outcome.document_id)
            .expect("unreadable PDF row")
            .document_status,
        SourceDocumentStatus::Processing
    );
    assert_eq!(
        documents
            .iter()
            .find(|document| document.document_id == csv_outcome.document_id)
            .expect("readable CSV row")
            .document_status,
        SourceDocumentStatus::Processing
    );
}

#[test]
fn accepts_only_the_expected_normalizer_handshake() {
    assert!(valid_normalizer_ready(1, "single-pass-mock", true));
    assert!(!valid_normalizer_ready(2, "single-pass-mock", true));
    assert!(!valid_normalizer_ready(1, "live-runtime", true));
    assert!(!valid_normalizer_ready(1, "single-pass-mock", false));
}

#[test]
fn rust_normalizer_command_matches_the_worker_golden_fixture() {
    let extraction_bundle = extract_bundle(
        "document-smoke",
        &"a".repeat(64),
        "text/csv",
        b"balance,2026-06-30,checking-001,1000.00,SGD,CANCAN_SYNTHETIC_STATEMENT_V1\n\
2026-07-01,Transfer to savings,250.00,SGD,750.00,provider=synthetic-bank\n\
balance,2026-07-01,checking-001,750.00,SGD,statement_id=transfer-2026-07\n\
balance,2026-06-30,savings-002,100.00,SGD\n\
2026-07-01,Transfer from checking,250.00,SGD,350.00\n\
balance,2026-07-01,savings-002,350.00,SGD",
        None,
    )
    .expect("extract fixture observations");
    let command = NormalizerCommand {
        document_id: "document-smoke",
        extraction_bundle: &extraction_bundle,
        request_id: "build-smoke",
        kind: "normalize",
    };
    let serialized = serde_json::to_value(command).expect("serialize normalizer command");
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../packages/ai/fixtures/normalizer-command-v1.json"
    ))
    .expect("parse shared normalizer command fixture");

    assert_eq!(serialized, fixture);
}

#[test]
fn maps_platform_and_document_render_failures_separately() {
    assert_eq!(
        document_render_error(io::Error::from(io::ErrorKind::Unsupported)).code(),
        "viewer_unsupported"
    );
    assert_eq!(
        document_render_error(io::Error::from(io::ErrorKind::InvalidInput)).code(),
        "invalid_document_request"
    );
    assert_eq!(
        document_render_error(io::Error::from(io::ErrorKind::InvalidData)).code(),
        "document_render_failed"
    );
}

#[test]
fn saves_an_atomic_plaintext_copy_only_outside_the_vault() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source = parent.path().join("statement.csv");
    let source_bytes = b"date,amount\n2026-07-22,42";
    fs::write(&source, source_bytes).expect("write source fixture");
    let root = parent.path().join("vault");
    let runtime = VaultRuntime::new(root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let imported = runtime
        .import_selected_document(&source)
        .expect("import source");

    let (mime_type, session_generation) = runtime
        .source_document_copy_context(&imported.document_id)
        .expect("read source copy context");
    assert_eq!(mime_type, "text/csv");
    let destination = parent.path().join("statement-copy.csv");
    fs::write(&destination, b"existing copy").expect("write existing destination");
    runtime
        .save_source_document_copy(&imported.document_id, &destination, session_generation)
        .expect("save source copy");
    assert_eq!(
        fs::read(&destination).expect("read saved copy"),
        source_bytes
    );
    assert!(
        fs::read_dir(parent.path())
            .expect("list export directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".cancan-export-"))
    );

    assert_eq!(
        runtime
            .save_source_document_copy(
                &imported.document_id,
                &root.join("copy-inside-vault.csv"),
                session_generation,
            )
            .expect_err("reject copy inside Vault")
            .code(),
        "source_copy_location_invalid"
    );
    runtime.lock().expect("lock Vault");
    assert_eq!(
        runtime
            .save_source_document_copy(&imported.document_id, &destination, session_generation,)
            .expect_err("reject copy while locked")
            .code(),
        "vault_locked"
    );
}

#[cfg(unix)]
#[test]
fn preserves_the_existing_destination_when_source_copy_write_fails() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source = parent.path().join("statement.csv");
    fs::write(&source, b"date,amount\n2026-07-22,42").expect("write source fixture");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let imported = runtime
        .import_selected_document(&source)
        .expect("import source");
    let destination_directory = parent.path().join("exports");
    fs::create_dir(&destination_directory).expect("create export directory");
    let destination = destination_directory.join("statement.csv");
    fs::write(&destination, b"existing copy").expect("write existing destination");
    let original_permissions = fs::metadata(&destination_directory)
        .expect("read destination permissions")
        .permissions();
    fs::set_permissions(&destination_directory, fs::Permissions::from_mode(0o500))
        .expect("make destination read-only");

    let (_, session_generation) = runtime
        .source_document_copy_context(&imported.document_id)
        .expect("read source copy context");
    let result =
        runtime.save_source_document_copy(&imported.document_id, &destination, session_generation);
    fs::set_permissions(&destination_directory, original_permissions)
        .expect("restore destination permissions");

    assert_eq!(
        result.expect_err("surface copy write failure").code(),
        "source_copy_save_failed"
    );
    assert_eq!(
        fs::read(&destination).expect("read preserved destination"),
        b"existing copy"
    );
    assert!(
        fs::read_dir(&destination_directory)
            .expect("list destination directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".cancan-export-"))
    );

    let directory_destination = destination_directory.join("existing-directory");
    fs::create_dir(&directory_destination).expect("create directory destination");
    assert_eq!(
        runtime
            .save_source_document_copy(
                &imported.document_id,
                &directory_destination,
                session_generation,
            )
            .expect_err("rename over a directory must fail")
            .code(),
        "source_copy_save_failed"
    );
    assert!(directory_destination.is_dir());
    assert!(
        fs::read_dir(&destination_directory)
            .expect("list destination after rename failure")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".cancan-export-"))
    );
}

#[test]
fn creates_locks_and_unlocks_a_vault_without_exposing_the_master_key() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));

    assert_eq!(
        runtime.status().expect("initial status"),
        VaultStatus::NotCreated
    );
    assert_eq!(
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault"),
        VaultStatus::Unlocked
    );
    let wrapper =
        fs::read(parent.path().join("vault").join(KEY_FILE_NAME)).expect("read password wrapper");
    assert!(
        !wrapper
            .windows("synthetic-vault-password".len())
            .any(|bytes| bytes == b"synthetic-vault-password")
    );
    assert_eq!(runtime.lock().expect("lock Vault"), VaultStatus::Locked);
    assert_eq!(
        runtime
            .unlock(b"wrong-password")
            .expect_err("reject wrong password")
            .code(),
        "invalid_credentials"
    );
    assert_eq!(
        runtime.status().expect("locked status"),
        VaultStatus::Locked
    );
    assert_eq!(
        runtime
            .unlock(b"synthetic-vault-password")
            .expect("unlock Vault"),
        VaultStatus::Unlocked
    );
}

#[test]
fn saves_recovery_outside_the_vault_and_persists_only_its_fingerprint() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let destination = parent.path().join("CanCan Recovery.cancan-recovery");
    let runtime = VaultRuntime::new(root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    assert!(
        !runtime
            .access_status()
            .expect("access status")
            .recovery_configured
    );

    runtime
        .save_recovery_file(&destination)
        .expect("save recovery file");

    let recovery_file = fs::read(&destination).expect("read recovery file");
    assert_eq!(&recovery_file[..8], b"CCREC001");
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(&destination)
            .expect("recovery metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let recovered_key = open_recovery_file(&recovery_file).expect("recover master key");
    let store = runtime.store().expect("active store");
    assert_eq!(
        recovered_key.as_slice(),
        store.as_ref().expect("unlocked store").master_key()
    );
    drop(store);

    let status = fs::read(root.join(RECOVERY_STATUS_FILE_NAME)).expect("read status");
    assert_eq!(status.len(), RECOVERY_STATUS_MAGIC.len() + KEY_LEN);
    assert_eq!(
        &status[..RECOVERY_STATUS_MAGIC.len()],
        RECOVERY_STATUS_MAGIC
    );
    assert_eq!(
        &status[RECOVERY_STATUS_MAGIC.len()..],
        recovery_file_fingerprint(&recovery_file)
    );
    assert!(
        runtime
            .access_status()
            .expect("access status")
            .recovery_configured
    );
    assert_eq!(
        runtime
            .save_recovery_file(&parent.path().join("second.cancan-recovery"))
            .expect_err("recovery is generated once")
            .code(),
        "recovery_already_configured"
    );

    runtime.lock().expect("lock Vault");
    drop(runtime);
    let restarted = VaultRuntime::new(root);
    assert!(
        restarted
            .access_status()
            .expect("restart status")
            .recovery_configured
    );
}

#[test]
fn rejects_recovery_inside_the_vault_without_marking_it_configured() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let runtime = VaultRuntime::new(root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");

    assert_eq!(
        runtime
            .save_recovery_file(&root.join("recovery.cancan-recovery"))
            .expect_err("reject recovery inside Vault")
            .code(),
        "recovery_location_invalid"
    );
    assert!(!root.join("recovery.cancan-recovery").exists());
    assert!(
        !runtime
            .access_status()
            .expect("access status")
            .recovery_configured
    );
}

#[test]
fn failed_recovery_write_does_not_mark_the_vault_configured() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");

    assert_eq!(
        runtime
            .save_recovery_file(
                &parent
                    .path()
                    .join("missing")
                    .join("recovery.cancan-recovery"),
            )
            .expect_err("reject unavailable destination")
            .code(),
        "recovery_save_failed"
    );
    assert!(
        !runtime
            .access_status()
            .expect("access status")
            .recovery_configured
    );
}

#[test]
fn system_lock_waits_for_store_cleanup_and_rejects_a_stale_store_generation() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let stale_generation = runtime.inner.system_lock_generation.load(Ordering::SeqCst);
    let held_store = runtime.raw_store().expect("hold active store");
    let locking_runtime = runtime.clone();
    let (finished, completion) = std::sync::mpsc::channel();
    thread::spawn(move || {
        finished
            .send(locking_runtime.request_system_lock())
            .expect("send lock result");
    });

    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while runtime.system_session_active() && std::time::Instant::now() < deadline {
        thread::yield_now();
    }
    assert!(!runtime.system_session_active());
    assert!(completion.try_recv().is_err());
    drop(held_store);
    completion
        .recv_timeout(Duration::from_secs(1))
        .expect("system lock completion")
        .expect("system lock");
    assert_eq!(
        runtime.status().expect("locked status"),
        VaultStatus::Locked
    );

    runtime
        .resume_system_session()
        .expect("resume system session");
    let stale_store = runtime.store_for_system_generation(stale_generation);
    assert_eq!(
        stale_store
            .err()
            .expect("reject a pre-lock store generation")
            .code(),
        "vault_locked"
    );
}

#[test]
fn remembers_unlock_in_the_secret_store_and_removes_it_explicitly() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let remembered_keys = Arc::new(MemoryRememberedKeyStore::default());
    let runtime = VaultRuntime::with_remembered_keys(root.clone(), remembered_keys.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime.remember_on_this_mac().expect("remember Vault");
    assert_eq!(
        runtime.access_status().expect("remembered status"),
        VaultAccessStatus {
            recovery_configured: false,
            remembered_on_this_mac: Some(true),
            status: VaultStatus::Unlocked,
        }
    );
    runtime.lock().expect("lock Vault");
    drop(runtime);

    let restarted = VaultRuntime::with_remembered_keys(root.clone(), remembered_keys.clone());
    assert_eq!(
        restarted.access_status().expect("restart status"),
        VaultAccessStatus {
            recovery_configured: false,
            remembered_on_this_mac: Some(true),
            status: VaultStatus::Locked,
        }
    );
    assert_eq!(
        restarted
            .unlock_with_keychain()
            .expect("unlock through Keychain"),
        VaultStatus::Unlocked
    );
    restarted.forget_this_mac().expect("forget this Mac");
    restarted.lock().expect("lock forgotten Vault");
    drop(restarted);

    let forgotten = VaultRuntime::with_remembered_keys(root, remembered_keys);
    assert_eq!(
        forgotten.access_status().expect("forgotten status"),
        VaultAccessStatus {
            recovery_configured: false,
            remembered_on_this_mac: Some(false),
            status: VaultStatus::Locked,
        }
    );
    assert_eq!(
        forgotten
            .unlock_with_keychain()
            .expect_err("remembered key was removed")
            .code(),
        "remembered_unlock_unavailable"
    );
}

#[test]
fn access_status_checks_presence_without_loading_the_secret() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let setup = VaultRuntime::with_remembered_keys(
        root.clone(),
        Arc::new(MemoryRememberedKeyStore::default()),
    );
    setup
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    setup.lock().expect("lock Vault");
    drop(setup);

    let runtime =
        VaultRuntime::with_remembered_keys(root, Arc::new(PresenceOnlyRememberedKeyStore));
    assert_eq!(
        runtime.access_status().expect("presence-only status"),
        VaultAccessStatus {
            recovery_configured: false,
            remembered_on_this_mac: Some(true),
            status: VaultStatus::Locked,
        }
    );
}

#[test]
fn removes_a_malformed_remembered_secret_before_password_fallback() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let remembered_keys = Arc::new(MemoryRememberedKeyStore::default());
    let runtime = VaultRuntime::with_remembered_keys(root, remembered_keys.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime.lock().expect("lock Vault");
    remembered_keys
        .save(&[0x55; KEY_LEN - 1])
        .expect("seed malformed secret");

    assert_eq!(
        runtime
            .unlock_with_keychain()
            .expect_err("reject malformed secret")
            .code(),
        "remembered_unlock_unavailable"
    );
    assert_eq!(
        runtime
            .access_status()
            .expect("status after malformed secret")
            .remembered_on_this_mac,
        Some(false)
    );
}

#[test]
fn reports_cleanup_failure_for_a_malformed_remembered_secret() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let setup = VaultRuntime::with_remembered_keys(
        root.clone(),
        Arc::new(MemoryRememberedKeyStore::default()),
    );
    setup
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    setup.lock().expect("lock Vault");
    drop(setup);

    let runtime = VaultRuntime::with_remembered_keys(
        root,
        Arc::new(MalformedDeleteFailingRememberedKeyStore),
    );
    assert_eq!(
        runtime
            .unlock_with_keychain()
            .expect_err("surface malformed-secret cleanup failure")
            .code(),
        "remembered_unlock_failed"
    );
    assert_eq!(
        runtime
            .access_status()
            .expect("failed cleanup remains visible")
            .remembered_on_this_mac,
        Some(true)
    );
}

#[test]
fn preserves_an_unverified_key_after_open_failure_and_keeps_password_unlock_available() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let remembered_keys = Arc::new(MemoryRememberedKeyStore::default());
    let runtime = VaultRuntime::with_remembered_keys(root.clone(), remembered_keys.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime.lock().expect("lock Vault");
    remembered_keys
        .save(&[0x55; KEY_LEN])
        .expect("seed stale key");
    drop(runtime);

    let restarted = VaultRuntime::with_remembered_keys(root, remembered_keys);
    assert_eq!(
        restarted
            .unlock_with_keychain()
            .expect_err("reject unverified remembered key")
            .code(),
        "remembered_unlock_failed"
    );
    assert_eq!(
        restarted
            .access_status()
            .expect("status after open failure")
            .remembered_on_this_mac,
        Some(true)
    );
    assert_eq!(
        restarted
            .unlock(b"synthetic-vault-password")
            .expect("password fallback"),
        VaultStatus::Unlocked
    );
}

#[test]
fn reports_a_safe_error_when_keychain_storage_fails() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::with_remembered_keys(
        parent.path().join("vault"),
        Arc::new(FailingRememberedKeyStore),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");

    assert_eq!(
        runtime
            .remember_on_this_mac()
            .expect_err("surface Keychain failure")
            .code(),
        "remember_failed"
    );
    assert_eq!(
        runtime.status().expect("Vault remains open"),
        VaultStatus::Unlocked
    );
    assert_eq!(
        runtime
            .forget_this_mac()
            .expect_err("surface Keychain deletion failure")
            .code(),
        "forget_failed"
    );
    assert_eq!(
        runtime
            .access_status()
            .expect("status remains available")
            .remembered_on_this_mac,
        None
    );
}

#[test]
fn saves_updates_and_removes_one_statement_password_per_money_source() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());

    runtime
        .save_statement_password("source-dbs", b"first-statement-password")
        .expect("save statement password");
    let first_state = statement_password_state(&runtime).expect("saved state");
    let secret_ref = first_state.secret_storage_key;
    assert_eq!(first_state.status, StatementPasswordStatus::Saved);
    assert_eq!(
        statement_passwords
            .load(&secret_ref)
            .expect("load statement password")
            .expect("saved statement password")
            .as_slice(),
        b"first-statement-password"
    );

    runtime
        .save_statement_password("source-dbs", b"updated-statement-password")
        .expect("replace statement password");
    let updated_state = statement_password_state(&runtime).expect("updated state");
    assert_eq!(updated_state.secret_storage_key, secret_ref);
    assert_eq!(
        statement_passwords
            .load(&secret_ref)
            .expect("load updated statement password")
            .expect("updated statement password")
            .as_slice(),
        b"updated-statement-password"
    );

    runtime
        .remove_statement_password("source-dbs")
        .expect("remove statement password");
    assert_eq!(statement_password_state(&runtime), None);
    assert!(
        statement_passwords
            .load(&secret_ref)
            .expect("load removed statement password")
            .is_none()
    );

    runtime
        .save_statement_password("source-dbs", b"locked-statement-password")
        .expect("save before lock");
    let locked_state = statement_password_state(&runtime).expect("state before lock");
    runtime.lock().expect("lock Vault");
    assert_eq!(
        runtime
            .save_statement_password("source-dbs", b"password")
            .expect_err("reject save while locked")
            .code(),
        "vault_locked"
    );
    assert_eq!(
        runtime
            .remove_statement_password("source-dbs")
            .expect_err("reject remove while locked")
            .code(),
        "vault_locked"
    );
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("unlock Vault");
    assert_eq!(
        statement_password_state(&runtime).expect("state after locked commands"),
        locked_state
    );
}

#[test]
fn keeps_statement_password_reference_state_consistent_when_keychain_fails() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());

    statement_passwords.fail_save.store(true, Ordering::SeqCst);
    assert_eq!(
        runtime
            .save_statement_password("source-dbs", b"statement-password")
            .expect_err("surface Keychain save failure")
            .code(),
        "statement_password_save_failed"
    );
    assert_eq!(
        statement_password_state(&runtime)
            .expect("recoverable save state")
            .status,
        StatementPasswordStatus::PendingSave
    );

    statement_passwords.fail_save.store(false, Ordering::SeqCst);
    runtime
        .save_statement_password("source-dbs", b"statement-password")
        .expect("save statement password");
    statement_passwords
        .fail_delete
        .store(true, Ordering::SeqCst);
    assert_eq!(
        runtime
            .remove_statement_password("source-dbs")
            .expect_err("surface Keychain remove failure")
            .code(),
        "statement_password_remove_failed"
    );
    assert_eq!(
        statement_password_state(&runtime)
            .expect("recoverable delete state")
            .status,
        StatementPasswordStatus::PendingDelete
    );
    statement_passwords
        .fail_delete
        .store(false, Ordering::SeqCst);
    runtime
        .remove_statement_password("source-dbs")
        .expect("retry pending delete");
    assert_eq!(statement_password_state(&runtime), None);
}

#[test]
fn reconciles_statement_password_crash_boundaries_after_unlock() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());
    let storage_key = statement_password_storage_key("source-dbs");

    runtime
        .store()
        .expect("active store")
        .as_ref()
        .expect("unlocked store")
        .begin_statement_password_save("source-dbs", &storage_key)
        .expect("persist pending save");
    statement_passwords
        .save(&storage_key, b"unverified-different-password")
        .expect("simulate unverified Keychain write before crash");
    runtime.lock().expect("simulate process lock");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("unlock and discard unverified pending save");
    assert_eq!(statement_password_state(&runtime), None);
    assert!(
        statement_passwords
            .load(&storage_key)
            .expect("load discarded pending secret")
            .is_none()
    );

    runtime
        .save_statement_password("source-dbs", b"verified-statement-password")
        .expect("save verified password after recovery");
    runtime
        .store()
        .expect("active store")
        .as_ref()
        .expect("unlocked store")
        .begin_statement_password_delete("source-dbs")
        .expect("persist pending delete");
    statement_passwords
        .delete(&storage_key)
        .expect("simulate Keychain delete before crash");
    runtime.lock().expect("simulate second process lock");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("unlock and reconcile pending delete");
    assert_eq!(statement_password_state(&runtime), None);
}

#[test]
fn unlock_paths_propagate_statement_password_reconciliation_failures() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());

    runtime
        .save_statement_password("source-dbs", b"verified-statement-password")
        .expect("save statement password");
    runtime
        .store()
        .expect("active store")
        .as_ref()
        .expect("unlocked store")
        .begin_statement_password_delete("source-dbs")
        .expect("persist pending delete");
    statement_passwords
        .fail_delete
        .store(true, Ordering::SeqCst);
    runtime.lock().expect("lock Vault");

    assert_eq!(
        runtime
            .unlock(b"synthetic-vault-password")
            .expect_err("surface password-unlock reconciliation failure")
            .code(),
        "statement_password_remove_failed"
    );
    assert!(runtime.store().expect("runtime store").is_none());

    statement_passwords
        .fail_delete
        .store(false, Ordering::SeqCst);
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("retry password unlock");
    assert_eq!(statement_password_state(&runtime), None);

    runtime
        .save_statement_password("source-dbs", b"replacement-statement-password")
        .expect("save replacement password");
    runtime
        .store()
        .expect("active store")
        .as_ref()
        .expect("unlocked store")
        .begin_statement_password_delete("source-dbs")
        .expect("persist second pending delete");
    runtime.remember_on_this_mac().expect("remember Vault");
    statement_passwords
        .fail_delete
        .store(true, Ordering::SeqCst);
    runtime.lock().expect("lock Vault again");

    assert_eq!(
        runtime
            .unlock_with_keychain()
            .expect_err("surface Keychain-unlock reconciliation failure")
            .code(),
        "statement_password_remove_failed"
    );
    assert!(runtime.store().expect("runtime store").is_none());

    statement_passwords
        .fail_delete
        .store(false, Ordering::SeqCst);
    runtime
        .unlock_with_keychain()
        .expect("retry Keychain unlock");
    assert_eq!(statement_password_state(&runtime), None);
}

#[cfg(target_os = "macos")]
#[test]
fn production_keychain_store_round_trips_binary_secret() {
    struct Cleanup(KeychainRememberedKeyStore);

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete();
        }
    }

    let service = format!("{KEYCHAIN_SERVICE}.test.{}", candidate_name());
    let store = KeychainRememberedKeyStore::new(service.clone(), KEYCHAIN_ACCOUNT);
    store.delete().expect("remove pre-existing test entry");
    let _cleanup = Cleanup(store.clone());
    assert!(!store.is_present().expect("test entry starts absent"));

    let secret = [0_u8, 1, 2, 0, 4, 5, 6, 7];
    store.save(&secret).expect("save binary Keychain secret");
    assert!(store.is_present().expect("test entry is present"));

    let restarted = KeychainRememberedKeyStore::new(service, KEYCHAIN_ACCOUNT);
    assert_eq!(
        restarted
            .load()
            .expect("load Keychain secret")
            .expect("saved secret exists")
            .as_slice(),
        secret
    );
    restarted.delete().expect("delete Keychain secret");
    assert!(!restarted.is_present().expect("test entry is absent"));
}

#[cfg(target_os = "macos")]
#[test]
fn production_statement_password_store_replaces_and_removes_secret() {
    struct Cleanup {
        secret_ref: String,
        store: KeychainStatementPasswordStore,
    }

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.store.delete(&self.secret_ref);
        }
    }

    let service = format!(
        "{STATEMENT_PASSWORD_KEYCHAIN_SERVICE}.test.{}",
        candidate_name()
    );
    let secret_ref = random_identifier("statement-password-test");
    let store = KeychainStatementPasswordStore::new(service);
    store
        .delete(&secret_ref)
        .expect("remove pre-existing test entry");
    let _cleanup = Cleanup {
        secret_ref: secret_ref.clone(),
        store: store.clone(),
    };

    store
        .save(&secret_ref, b"first-synthetic-password")
        .expect("save statement password");
    store
        .save(&secret_ref, b"updated-synthetic-password")
        .expect("replace statement password");
    assert_eq!(
        store
            .load(&secret_ref)
            .expect("load statement password")
            .expect("saved statement password exists")
            .as_slice(),
        b"updated-synthetic-password"
    );
    store
        .delete(&secret_ref)
        .expect("delete statement password");
    assert!(
        store
            .load(&secret_ref)
            .expect("load deleted statement password")
            .is_none()
    );
}

#[test]
fn rejects_empty_passwords_and_existing_or_corrupt_vaults() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let runtime = VaultRuntime::new(root.clone());

    assert_eq!(
        runtime
            .create(b"")
            .expect_err("reject empty password")
            .code(),
        "password_required"
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime.lock().expect("lock Vault");
    assert_eq!(
        runtime
            .create(b"another-password")
            .expect_err("reject existing Vault")
            .code(),
        "vault_already_exists"
    );

    fs::write(root.join(KEY_FILE_NAME), b"corrupt wrapper").expect("corrupt wrapper fixture");
    assert_eq!(
        runtime
            .unlock(b"synthetic-vault-password")
            .expect_err("reject corrupt wrapper")
            .code(),
        "invalid_vault"
    );
    assert_eq!(
        runtime.status().expect_err("corrupt status").code(),
        "invalid_vault"
    );
}

#[test]
fn serializes_status_with_a_concurrent_create_transition() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    let creating = runtime.clone();
    let create = thread::spawn(move || creating.create(b"synthetic-vault-password"));

    let mut observed_transition = false;
    for _ in 0..100 {
        match runtime.inner.store.try_lock() {
            Err(std::sync::TryLockError::WouldBlock) => {
                observed_transition = true;
                break;
            }
            Err(std::sync::TryLockError::Poisoned(_)) => panic!("runtime mutex poisoned"),
            Ok(guard) => drop(guard),
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        observed_transition,
        "create never acquired the session mutex"
    );
    assert_eq!(
        runtime.status().expect("status after serialized create"),
        VaultStatus::Unlocked
    );
    assert_eq!(
        create.join().expect("create thread").expect("create Vault"),
        VaultStatus::Unlocked
    );
}

#[test]
fn classifies_a_missing_wrapper_consistently_as_an_invalid_vault() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    fs::create_dir(&root).expect("create incomplete Vault");
    let runtime = VaultRuntime::new(root);

    assert_eq!(
        runtime.status().expect_err("invalid status").code(),
        "invalid_vault"
    );
    assert_eq!(
        runtime
            .unlock(b"synthetic-vault-password")
            .expect_err("invalid unlock")
            .code(),
        "invalid_vault"
    );
    assert_eq!(
        runtime.lock().expect_err("invalid lock").code(),
        "invalid_vault"
    );
}

#[test]
fn rejects_unlock_when_the_existing_vault_database_is_missing() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let runtime = VaultRuntime::new(root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime.lock().expect("lock Vault");
    let database_path = root.join(DATABASE_FILE_NAME);
    fs::remove_file(&database_path).expect("remove Vault database");

    assert_eq!(
        runtime
            .status()
            .expect_err("reject incomplete status")
            .code(),
        "invalid_vault"
    );
    assert_eq!(
        runtime
            .unlock(b"synthetic-vault-password")
            .expect_err("reject incomplete Vault")
            .code(),
        "invalid_vault"
    );
    assert!(
        !database_path.exists(),
        "unlock must not recreate the database"
    );
}

#[test]
fn reopens_an_activated_vault_as_locked_after_runtime_restart() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let runtime = VaultRuntime::new(root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    drop(runtime);

    let restarted = VaultRuntime::new(root);
    assert_eq!(
        restarted.status().expect("restart status"),
        VaultStatus::Locked
    );
    assert_eq!(
        restarted
            .unlock(b"synthetic-vault-password")
            .expect("unlock restarted Vault"),
        VaultStatus::Unlocked
    );
}

#[test]
fn imports_and_lists_an_unassigned_document_only_while_unlocked() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("DBS-July-2026.pdf");
    fs::write(&source_path, synthetic_pdf()).expect("write statement fixture");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("import statement");
    assert_eq!(imported.status, SourceDocumentImportStatus::Imported);

    let duplicate = runtime
        .import_selected_document(&source_path)
        .expect("deduplicate statement");
    assert_eq!(duplicate.document_id, imported.document_id);
    assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

    runtime
        .delete_source_document(&imported.document_id)
        .expect("delete encrypted source");
    assert!(
        source_path.exists(),
        "user-selected source must remain untouched"
    );
    let deleted = runtime
        .list_unassigned_source_documents()
        .expect("list deleted document");
    assert_eq!(deleted[0].file_state, "deleted");
    let confirmation = runtime
        .import_selected_document(&source_path)
        .expect("request restore confirmation");
    assert_eq!(
        confirmation.status,
        SourceDocumentImportStatus::RestoreConfirmationRequired
    );
    let receipt_id = confirmation
        .intake_item_id
        .as_deref()
        .expect("restore receipt id");
    let restored = runtime
        .confirm_restore_selected_document(&source_path, &imported.document_id, receipt_id)
        .expect("restore deleted source");
    assert_eq!(restored.document_id, imported.document_id);
    assert_eq!(restored.status, SourceDocumentImportStatus::Restored);

    let documents = runtime
        .list_unassigned_source_documents()
        .expect("list unassigned documents");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].document_id, imported.document_id);
    assert_eq!(documents[0].original_filename, "DBS-July-2026.pdf");
    assert_eq!(documents[0].mime_type, "application/pdf");
    assert_eq!(documents[0].byte_size, synthetic_pdf().len() as u64);
    assert_eq!(documents[0].file_state, "available");
    {
        let store = runtime.inner.store.lock().expect("runtime store");
        let persisted = store
            .as_ref()
            .expect("unlocked store")
            .list_unassigned_documents()
            .expect("inspect imported document");
        assert_eq!(persisted[0].money_source_id, None);
        assert_eq!(persisted[0].semantic_document_key, None);
    }

    runtime.lock().expect("lock Vault");
    assert_eq!(
        runtime
            .list_unassigned_source_documents()
            .expect_err("reject list while locked")
            .code(),
        "vault_locked"
    );
    assert_eq!(
        runtime
            .import_selected_document(&source_path)
            .expect_err("reject import while locked")
            .code(),
        "vault_locked"
    );
}

#[test]
fn rejects_unsupported_document_imports() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let unsupported = parent.path().join("statement.exe");
    fs::write(&unsupported, b"not a financial document").expect("write unsupported fixture");

    assert_eq!(
        runtime
            .import_selected_document(&unsupported)
            .expect_err("reject unsupported document")
            .code(),
        "unsupported_document"
    );
}

#[test]
fn renders_an_imported_pdf_in_memory_only_while_the_vault_is_unlocked() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("statement.pdf");
    fs::write(&source_path, synthetic_pdf()).expect("write PDF fixture");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::new(vault_root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("import PDF");
    let before = vault_entries(&vault_root);

    let rendered = runtime
        .render_source_document_page(&imported.document_id, 1)
        .expect("render PDF");

    assert_eq!(rendered.page_count, 1);
    assert_eq!(rendered.page_number, 1);
    assert!(!rendered.png_base64.is_empty());
    assert_eq!(vault_entries(&vault_root), before);
    runtime.lock().expect("lock Vault");
    assert_eq!(
        runtime
            .render_source_document_page(&imported.document_id, 1)
            .expect_err("reject rendering while locked")
            .code(),
        "vault_locked"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn imports_and_renders_an_image_in_memory_only() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("phone-statement.png");
    fs::write(&source_path, synthetic_png_fixture()).expect("write PNG fixture");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::new(vault_root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("import PNG");
    let before = vault_entries(&vault_root);

    let rendered = runtime
        .render_source_document_page(&imported.document_id, 1)
        .expect("render PNG");

    assert_eq!(rendered.page_count, 1);
    assert_eq!(rendered.page_number, 1);
    assert!(!rendered.png_base64.is_empty());
    assert_eq!(vault_entries(&vault_root), before);
    assert_eq!(
        runtime
            .render_source_document_page(&imported.document_id, 2)
            .expect_err("reject image page two")
            .code(),
        "invalid_document_request"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn rejects_an_invalid_image_before_storing_it() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("not-an-image.png");
    fs::write(&source_path, b"not a PNG").expect("write invalid PNG fixture");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::new(vault_root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let before = vault_entries(&vault_root);

    assert_eq!(
        runtime
            .import_selected_document(&source_path)
            .expect_err("reject invalid PNG")
            .code(),
        "import_failed"
    );
    assert_eq!(vault_entries(&vault_root), before);
}

#[test]
fn rejects_non_pdf_and_invalid_page_view_requests() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let csv_path = parent.path().join("statement.csv");
    fs::write(&csv_path, b"date,amount\n2026-07-19,42").expect("write CSV fixture");
    let pdf_path = parent.path().join("statement.pdf");
    fs::write(&pdf_path, synthetic_pdf()).expect("write PDF fixture");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let csv = runtime
        .import_selected_document(&csv_path)
        .expect("import CSV");
    let pdf = runtime
        .import_selected_document(&pdf_path)
        .expect("import PDF");

    assert_eq!(
        runtime
            .render_source_document_page(&csv.document_id, 1)
            .expect_err("reject CSV viewer")
            .code(),
        "viewer_unsupported"
    );
    assert_eq!(
        runtime
            .render_source_document_page(&pdf.document_id, 0)
            .expect_err("reject page zero")
            .code(),
        "invalid_document_request"
    );
    assert_eq!(
        runtime
            .render_source_document_page(&pdf.document_id, 2)
            .expect_err("reject out-of-range page")
            .code(),
        "invalid_document_request"
    );
    assert_eq!(
        runtime
            .render_source_document_page("missing-document", 1)
            .expect_err("reject missing document")
            .code(),
        "document_unavailable"
    );
}

#[test]
fn previews_only_bounded_csv_lines_while_the_vault_is_unlocked() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");

    let csv_path = parent.path().join("wise-export.csv");
    fs::write(
        &csv_path,
        b"date,amount\r\n2026-07-01,10.00\r\n2026-07-02,-4.25\n",
    )
    .expect("write CSV fixture");
    let imported = runtime
        .import_selected_document(&csv_path)
        .expect("import CSV");
    let preview = runtime
        .preview_source_document(&imported.document_id)
        .expect("preview CSV");
    assert_eq!(preview.line_count, 3);
    assert_eq!(preview.preview_lines, 3);
    assert_eq!(
        preview.preview_text,
        "date,amount\n2026-07-01,10.00\n2026-07-02,-4.25"
    );
    assert!(!preview.truncated);

    let complete_text = b"date,amount\n2026-07-01,10.00";
    let complete_preview = bounded_text_preview(complete_text);
    assert_eq!(complete_preview.preview_text.as_bytes(), complete_text);
    assert!(!complete_preview.truncated);

    let leading_blank_preview = bounded_text_preview(b"\n\ndate,amount");
    assert_eq!(leading_blank_preview.line_count, 3);
    assert_eq!(leading_blank_preview.preview_lines, 3);
    assert_eq!(leading_blank_preview.preview_text, "\n\ndate,amount");
    assert!(!leading_blank_preview.truncated);

    let mut big_content = String::from("date,amount\n");
    for row in 1..=300 {
        big_content.push_str(&format!("2026-07-01,{row}.00\n"));
    }
    let big_path = parent.path().join("big-export.csv");
    fs::write(&big_path, big_content).expect("write big CSV fixture");
    let big = runtime
        .import_selected_document(&big_path)
        .expect("import big CSV");
    let big_preview = runtime
        .preview_source_document(&big.document_id)
        .expect("preview big CSV");
    assert_eq!(big_preview.line_count, 301);
    assert_eq!(big_preview.preview_lines, PREVIEW_MAX_LINES as u64);
    assert!(big_preview.truncated);
    assert_eq!(big_preview.preview_text.lines().count(), PREVIEW_MAX_LINES);
    assert!(big_preview.preview_text.starts_with("date,amount\n"));

    let long_path = parent.path().join("long-line.csv");
    let long_content = format!("memo,{}\n2026-07-02,1.00\n", "é".repeat(PREVIEW_MAX_BYTES));
    fs::write(&long_path, long_content).expect("write long-line CSV fixture");
    let long = runtime
        .import_selected_document(&long_path)
        .expect("import long-line CSV");
    let long_preview = runtime
        .preview_source_document(&long.document_id)
        .expect("preview long-line CSV");
    assert_eq!(long_preview.line_count, 2);
    assert_eq!(long_preview.preview_lines, 1);
    assert!(long_preview.truncated);
    assert!(long_preview.preview_text.len() <= PREVIEW_MAX_BYTES);
    assert!(long_preview.preview_text.starts_with("memo,"));

    // The byte budget is never exceeded, even by a separator byte or a
    // zero-byte trailing line.
    let capped_path = parent.path().join("capped.csv");
    let capped_content = format!("{}\n\n", "x".repeat(PREVIEW_MAX_BYTES));
    fs::write(&capped_path, capped_content).expect("write capped CSV fixture");
    let capped = runtime
        .import_selected_document(&capped_path)
        .expect("import capped CSV");
    let capped_preview = runtime
        .preview_source_document(&capped.document_id)
        .expect("preview capped CSV");
    assert_eq!(capped_preview.line_count, 2);
    assert_eq!(capped_preview.preview_lines, 1);
    assert!(capped_preview.truncated);
    assert_eq!(capped_preview.preview_text.len(), PREVIEW_MAX_BYTES);

    let empty_path = parent.path().join("empty.csv");
    fs::write(&empty_path, b"").expect("write empty CSV fixture");
    let empty = runtime
        .import_selected_document(&empty_path)
        .expect("import empty CSV");
    let empty_preview = runtime
        .preview_source_document(&empty.document_id)
        .expect("preview empty CSV");
    assert_eq!(empty_preview.line_count, 0);
    assert_eq!(empty_preview.preview_lines, 0);
    assert!(!empty_preview.truncated);
    assert!(empty_preview.preview_text.is_empty());

    let pdf_path = parent.path().join("statement.pdf");
    fs::write(&pdf_path, synthetic_pdf()).expect("write PDF fixture");
    let pdf = runtime
        .import_selected_document(&pdf_path)
        .expect("import PDF");
    assert_eq!(
        runtime
            .preview_source_document(&pdf.document_id)
            .expect_err("PDF keeps the pixel viewer")
            .code(),
        "viewer_unsupported"
    );
    assert_eq!(
        runtime
            .preview_source_document("missing-document")
            .expect_err("reject missing document")
            .code(),
        "document_unavailable"
    );

    runtime.lock().expect("lock Vault");
    assert_eq!(
        runtime
            .preview_source_document(&imported.document_id)
            .expect_err("reject preview while locked")
            .code(),
        "vault_locked"
    );
}

#[test]
fn review_and_account_confirmation_models_reject_a_locked_vault() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));

    assert_eq!(
        runtime
            .list_review_items()
            .expect_err("reject review list while locked")
            .code(),
        "vault_locked"
    );
    assert_eq!(
        runtime
            .money_overview()
            .expect_err("reject overview while locked")
            .code(),
        "vault_locked"
    );
    assert_eq!(
        runtime
            .list_account_confirmation_prompts()
            .expect_err("reject confirmation list while locked")
            .code(),
        "vault_locked"
    );
    assert_eq!(
        runtime
            .decide_candidate_accounts(
                "source-dbs",
                "proposal-version",
                &[CandidateAccountDecisionInput {
                    account_id: "account-1".to_owned(),
                    action: CandidateAccountDecision::Accept,
                }],
            )
            .expect_err("reject confirmation while locked")
            .code(),
        "vault_locked"
    );
}

#[test]
fn applies_verified_mock_normalizer_routing_without_renderer_identity_input() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("synthetic.csv");
    fs::write(&source_path, synthetic_statement_csv()).expect("write statement fixture");
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
    assert_eq!(
        runtime.list_money_sources().expect("list Money Sources"),
        vec![MoneySourceSummary {
            display_name: "Synthetic Bank".to_owned(),
            money_source_id: "source-synthetic".to_owned(),
            source_type: "bank".to_owned(),
        }]
    );
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("capture statement");
    let input = runtime
        .normalization_input(&imported.document_id)
        .expect("extract synthetic CSV observations");
    let routed = runtime
        .apply_normalizer_result(&imported.document_id, &input, synthetic_normalizer_result())
        .expect("apply trusted routing");

    assert_eq!(
        routed.status,
        crate::database::SourceDocumentRoutingStatus::Routed
    );
    assert_eq!(routed.money_source_id.as_deref(), Some("source-synthetic"));
    assert_eq!(routed.account_ids.len(), 2);
    let routed_documents = runtime
        .list_source_documents("source-synthetic")
        .expect("list routed documents");
    assert_eq!(routed_documents.len(), 1);
    assert_eq!(routed_documents[0].document_id, imported.document_id);
    assert!(
        runtime
            .list_unassigned_source_documents()
            .expect("list pending")
            .is_empty()
    );
}

#[test]
fn persists_validated_records_and_reconciles_an_explicit_parser_re_run() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("synthetic.csv");
    fs::write(&source_path, synthetic_statement_csv()).expect("write statement fixture");
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
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("capture statement");
    let input = runtime
        .normalization_input(&imported.document_id)
        .expect("extract complete synthetic observations");

    let mut normalizer_result = synthetic_normalizer_result();
    let NormalizerResult::Classified { proposal, .. } = &mut normalizer_result else {
        panic!("synthetic result must be classified");
    };
    proposal.records[0].posting_status = Some("posted".to_owned());
    let routed = runtime
        .apply_normalizer_result(&imported.document_id, &input, normalizer_result)
        .expect("persist validated proposal");
    assert_eq!(
        routed.status,
        crate::database::SourceDocumentRoutingStatus::Routed
    );
    assert_eq!(
        structured_parse_test_state(&runtime, &imported.document_id),
        crate::database::StructuredParseTestState {
            balance_snapshots_without_amount: 4,
            ledger_events: 0,
            open_review_items: 0,
            parse_runs: 1,
            reconcile_status: Some("queued".to_owned()),
            records: 6,
            staged_records: 6,
        }
    );
    assert_eq!(
        structured_parse_posting_status(
            &runtime,
            &imported.document_id,
            "synthetic:record-checking-out",
        ),
        Some("posted".to_owned())
    );
    assert_eq!(
        structured_parse_posting_status(
            &runtime,
            &imported.document_id,
            "synthetic:record-savings-in",
        ),
        None
    );

    assert!(
        runtime
            .start_document_reconciliation(&imported.document_id)
            .expect("claim reconcile job")
    );
    runtime
        .fail_document_reconciliation(&imported.document_id, "reconcile_failed")
        .expect("record safe reconcile failure");
    let failed = structured_parse_test_state(&runtime, &imported.document_id);
    assert_eq!((failed.staged_records, failed.open_review_items), (6, 0));
    assert_eq!(failed.reconcile_status.as_deref(), Some("failed"));

    runtime
        .enqueue_source_document_reparse(&imported.document_id)
        .expect("enqueue explicit parser re-run");
    assert_eq!(
        runtime
            .enqueue_source_document_reparse(&imported.document_id)
            .expect_err("reject a second active parser re-run")
            .code(),
        "parse_already_running"
    );
    let reparse = runtime
        .queued_local_inbox_parse_documents()
        .expect("read explicit parser re-run")
        .pop()
        .expect("queued explicit parser re-run");
    let attempt = runtime
        .start_local_inbox_parse(&reparse)
        .expect("claim explicit parser re-run")
        .expect("explicit parser re-run claimed");
    runtime
        .apply_normalizer_result_for_job(&attempt.claim, &input, synthetic_normalizer_result())
        .expect("repeat same profile");
    assert_eq!(
        structured_parse_posting_status(
            &runtime,
            &imported.document_id,
            "synthetic:record-checking-out",
        ),
        Some("posted".to_owned())
    );
    assert_eq!(
        runtime
            .queued_document_reconciliations()
            .expect("requeue failed reconcile"),
        vec![imported.document_id.clone()]
    );
    assert!(
        runtime
            .start_document_reconciliation(&imported.document_id)
            .expect("claim recovery candidate")
    );
    expire_reconcile_lease_for_test(&runtime, &imported.document_id);
    assert_eq!(
        runtime
            .queued_document_reconciliations()
            .expect("recover expired reconcile job"),
        vec![imported.document_id.clone()]
    );
    assert!(
        runtime
            .start_document_reconciliation(&imported.document_id)
            .expect("claim recovered reconcile job")
    );
    runtime
        .reconcile_document(&imported.document_id)
        .expect("move staged records to review");

    assert_eq!(
        structured_parse_test_state(&runtime, &imported.document_id),
        crate::database::StructuredParseTestState {
            balance_snapshots_without_amount: 8,
            ledger_events: 0,
            open_review_items: 6,
            parse_runs: 2,
            reconcile_status: Some("succeeded".to_owned()),
            records: 12,
            staged_records: 0,
        }
    );
}

#[cfg(target_os = "macos")]
#[test]
fn accepts_a_review_only_dbs_profile_and_reconciles_its_records_to_review() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("dbs-statement.pdf");
    fs::write(&source_path, synthetic_provider_statement_pdf()).expect("write DBS PDF fixture");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .seed_money_source("source-dbs", "dbs", "DBS", "bank")
        .expect("seed DBS source");
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("capture DBS statement");
    let input = runtime
        .normalization_input(&imported.document_id)
        .expect("extract native DBS PDF observation");

    let routed = runtime
        .apply_normalizer_result(&imported.document_id, &input, dbs_bank_normalizer_result())
        .expect("apply review-only DBS result");

    assert_eq!(
        routed.status,
        crate::database::SourceDocumentRoutingStatus::Routed
    );
    let staged = structured_parse_test_state(&runtime, &imported.document_id);
    assert_eq!(
        (staged.parse_runs, staged.records, staged.staged_records),
        (1, 3, 3)
    );
    assert_eq!(staged.reconcile_status.as_deref(), Some("queued"));
    assert!(
        runtime
            .start_document_reconciliation(&imported.document_id)
            .expect("claim DBS reconcile job")
    );
    runtime
        .reconcile_document(&imported.document_id)
        .expect("move DBS records to Review");
    let reviewed = structured_parse_test_state(&runtime, &imported.document_id);
    assert_eq!((reviewed.parse_runs, reviewed.records), (1, 3));
    assert_eq!(reviewed.open_review_items, 3);
    assert_eq!(reviewed.ledger_events, 0);
}

#[cfg(target_os = "macos")]
#[test]
fn local_inbox_commits_a_cross_month_hsbc_to_dbs_card_repayment_after_restart() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
    let vault_root = parent.path().join("vault");
    let bookmarks = Arc::new(MemoryLocalInboxBookmarkStore::default());
    let runtime = VaultRuntime::with_secret_stores(
        vault_root.clone(),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        bookmarks.clone(),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .seed_money_source("source-dbs-card", "dbs", "DBS Card", "credit_card")
        .expect("seed DBS card source");
    runtime
        .seed_money_source("source-hsbc-bank", "hsbc", "HSBC Bank", "bank")
        .expect("seed HSBC bank source");
    runtime
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");
    assert!(bookmarks.load().expect("load stored bookmark").is_some());
    assert_eq!(
        runtime
            .local_inbox_status()
            .expect("read configured Inbox status")
            .access_state,
        LocalInboxAccessState::Enabled
    );
    runtime.lock().expect("lock Vault before restart");
    drop(runtime);

    let runtime = VaultRuntime::with_secret_stores(
        vault_root,
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        bookmarks,
    );
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("unlock restarted Vault");
    assert_eq!(
        runtime
            .local_inbox_status()
            .expect("restore Inbox bookmark after restart")
            .access_state,
        LocalInboxAccessState::Enabled
    );

    let inbox = inbox_root.join("Inbox");
    let hsbc_path = inbox.join("hsbc-repayment-2026-06.pdf");
    let dbs_path = inbox.join("dbs-card-repayment-2026-07.pdf");
    let hsbc_bytes = hsbc_repayment_statement_pdf();
    let dbs_bytes = dbs_card_repayment_statement_pdf();
    fs::write(&hsbc_path, &hsbc_bytes).expect("write HSBC repayment statement");
    fs::write(&dbs_path, &dbs_bytes).expect("write DBS card repayment statement");

    assert_eq!(
        runtime.rescan_local_inbox().expect("scan local Inbox"),
        LocalInboxScanSummary {
            already_present: 0,
            deferred: 0,
            imported: 2,
            suppressed: 0,
        }
    );
    assert_eq!(fs::read(&hsbc_path).expect("read HSBC source"), hsbc_bytes);
    assert_eq!(fs::read(&dbs_path).expect("read DBS source"), dbs_bytes);

    let mut parse_jobs = runtime
        .queued_local_inbox_parse_documents()
        .expect("queue both imported statements");
    parse_jobs.sort_by(|left, right| left.document_id.cmp(&right.document_id));
    assert_eq!(parse_jobs.len(), 2);
    let mut hsbc_document_id = None;
    let mut dbs_document_id = None;
    for job in &parse_jobs {
        let attempt = runtime
            .start_local_inbox_parse(job)
            .expect("claim local Inbox parse job")
            .expect("local Inbox parse job claimed");
        let input = runtime
            .normalization_input(&job.document_id)
            .expect("extract native PDF observation");
        assert_eq!(input.observations.len(), 1);
        let observation = &input.observations[0];
        assert_eq!(observation.kind, SourceObservationKind::NativeText);
        assert_eq!(observation.engine, "pdfkit");
        assert_eq!(observation.engine_version, "macos-page-string-v1");

        let routed = if observation.text.contains("provider=hsbc") {
            hsbc_document_id = Some(job.document_id.clone());
            runtime
                .apply_normalizer_result_for_job(
                    &attempt.claim,
                    &input,
                    hsbc_bank_repayment_normalizer_result(),
                )
                .expect("apply exact HSBC profile and proposal")
        } else if observation.text.contains("provider=dbs") {
            dbs_document_id = Some(job.document_id.clone());
            runtime
                .apply_normalizer_result_for_job(
                    &attempt.claim,
                    &input,
                    dbs_card_repayment_normalizer_result(),
                )
                .expect("apply exact DBS card profile and proposal")
        } else {
            panic!("unexpected local Inbox PDF package");
        };
        assert_eq!(
            routed.status,
            crate::database::SourceDocumentRoutingStatus::Routed
        );
    }
    let hsbc_document_id = hsbc_document_id.expect("identify HSBC statement");
    let dbs_document_id = dbs_document_id.expect("identify DBS statement");

    let mut reconcile_documents = runtime
        .queued_document_reconciliations()
        .expect("queue reconciliations after parse");
    reconcile_documents.sort();
    let mut expected_reconcile_documents = vec![hsbc_document_id.clone(), dbs_document_id.clone()];
    expected_reconcile_documents.sort();
    assert_eq!(reconcile_documents, expected_reconcile_documents);
    for document_id in &reconcile_documents {
        assert!(
            runtime
                .start_document_reconciliation(document_id)
                .expect("claim document reconciliation")
        );
        runtime
            .reconcile_document(document_id)
            .expect("move parsed statement records into Review");
    }
    assert!(
        runtime
            .list_recent_activity()
            .expect("read pre-commit Activity")
            .is_empty()
    );

    let prompts = runtime
        .list_account_confirmation_prompts()
        .expect("list exact candidate account prompts");
    assert_eq!(prompts.len(), 2);
    let dbs_prompt = prompts
        .iter()
        .find(|prompt| prompt.money_source_id == "source-dbs-card")
        .expect("DBS card confirmation prompt");
    assert_eq!(dbs_prompt.candidate_accounts.len(), 1);
    assert_eq!(dbs_prompt.candidate_accounts[0].account_type, "credit_card");
    assert_eq!(
        dbs_prompt.candidate_accounts[0].currency.as_deref(),
        Some("SGD")
    );
    let dbs_candidate_ids = dbs_prompt
        .candidate_accounts
        .iter()
        .map(|account| account.account_id.clone())
        .collect::<Vec<_>>();
    let dbs_proposal_version = dbs_prompt.proposal_version.clone();
    let hsbc_prompt = prompts
        .iter()
        .find(|prompt| prompt.money_source_id == "source-hsbc-bank")
        .expect("HSBC bank confirmation prompt");
    assert_eq!(hsbc_prompt.candidate_accounts.len(), 1);
    assert_eq!(
        hsbc_prompt.candidate_accounts[0].account_type,
        "deposit_account"
    );
    assert_eq!(
        hsbc_prompt.candidate_accounts[0].currency.as_deref(),
        Some("SGD")
    );
    let hsbc_candidate_ids = hsbc_prompt
        .candidate_accounts
        .iter()
        .map(|account| account.account_id.clone())
        .collect::<Vec<_>>();
    let hsbc_proposal_version = hsbc_prompt.proposal_version.clone();
    assert_eq!(
        runtime
            .decide_candidate_accounts(
                "source-dbs-card",
                &dbs_proposal_version,
                &dbs_candidate_ids
                    .iter()
                    .map(|account_id| CandidateAccountDecisionInput {
                        account_id: account_id.clone(),
                        action: CandidateAccountDecision::Accept,
                    })
                    .collect::<Vec<_>>(),
            )
            .expect("confirm exactly DBS card candidates")
            .status,
        crate::database::AccountConfirmationStatus::Confirmed
    );
    assert_eq!(
        runtime
            .decide_candidate_accounts(
                "source-hsbc-bank",
                &hsbc_proposal_version,
                &hsbc_candidate_ids
                    .iter()
                    .map(|account_id| CandidateAccountDecisionInput {
                        account_id: account_id.clone(),
                        action: CandidateAccountDecision::Accept,
                    })
                    .collect::<Vec<_>>(),
            )
            .expect("confirm exactly HSBC bank candidates")
            .status,
        crate::database::AccountConfirmationStatus::Confirmed
    );
    assert!(
        runtime
            .list_account_confirmation_prompts()
            .expect("candidate accounts are now confirmed")
            .is_empty()
    );

    let review_items = runtime.list_review_items().expect("list Review items");
    let repayment_items = review_items
        .iter()
        .filter(|item| item.event_type.as_deref() == Some("credit_card_repayment"))
        .collect::<Vec<_>>();
    assert_eq!(repayment_items.len(), 2);
    let dbs_repayment = repayment_items
        .iter()
        .find(|item| item.posted_on.as_deref() == Some("2026-07-01"))
        .expect("DBS card repayment review item");
    let hsbc_repayment = repayment_items
        .iter()
        .find(|item| item.posted_on.as_deref() == Some("2026-06-30"))
        .expect("historical HSBC repayment review item");
    let candidate_input = runtime
        .relationship_candidate_input(&dbs_repayment.review_item_id, dbs_repayment.record_version)
        .expect("discover historical relationship candidates")
        .expect("open DBS repayment review item");
    assert_eq!(candidate_input.event_type, "credit_card_repayment");
    assert_eq!(candidate_input.record.posted_on, "2026-07-01");
    assert_eq!(
        candidate_input
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<Vec<_>>(),
        vec![hsbc_repayment.record_id.as_str()]
    );
    let historical = candidate_input
        .candidates
        .first()
        .expect("HSBC candidate within repayment window");
    assert_eq!(historical.posted_on, "2026-06-30");
    let prepared = test_prepared_repayment_event(&candidate_input.record, historical);
    assert_eq!(prepared.event_type, "credit_card_repayment");
    assert_eq!(prepared.event_date, "2026-06-30");
    assert!(!prepared.spending);
    assert_eq!(
        runtime
            .accept_review_relationship(
                &dbs_repayment.review_item_id,
                dbs_repayment.record_version,
                &historical.id,
                hsbc_repayment.record_version,
                &prepared,
            )
            .expect("accept exact cross-month repayment")
            .status,
        ReviewMutationStatus::RelationshipAccepted
    );

    let job = runtime
        .enqueue_commit_review_batch(&[
            dbs_repayment.review_item_id.clone(),
            hsbc_repayment.review_item_id.clone(),
        ])
        .expect("enqueue explicit repayment review selection");
    assert_eq!(
        runtime
            .queued_review_job_ids()
            .expect("list queued Review job"),
        vec![job.job_id.clone()]
    );
    let claimed = runtime
        .claim_review_batch(&job.job_id, "local-inbox-test")
        .expect("claim Review batch")
        .expect("queued Review batch");
    let (groups, initial_outcomes) = runtime
        .prepare_commit_review_groups(&claimed)
        .expect("prepare accepted repayment group");
    assert!(initial_outcomes.is_empty());
    assert_eq!(groups.len(), 1);
    let prepared_commit =
        test_prepared_repayment_event(&groups[0].records[0], &groups[0].records[1]);
    let committed = runtime
        .commit_prepared_review_group(&claimed, &groups[0], &prepared_commit)
        .expect("commit prepared repayment group");
    assert_eq!(committed.status, ReviewBatchGroupStatus::Committed);
    assert_eq!(
        runtime
            .finish_review_batch(&claimed, &[committed])
            .expect("finish Review batch")
            .status,
        crate::database::ReviewJobStatus::Succeeded
    );

    let activity = runtime
        .list_recent_activity()
        .expect("list Recent Activity");
    assert_eq!(activity.len(), 1);
    assert_eq!(activity[0].event_type, "credit_card_repayment");
    assert!(!activity[0].spending);
    assert_eq!(activity[0].source_labels, vec!["DBS Card", "HSBC Bank"]);
    let remaining_review_items = runtime.list_review_items().expect("list remaining Review");
    assert_eq!(remaining_review_items.len(), 4);
    assert!(
        remaining_review_items
            .iter()
            .all(|item| item.event_type.as_deref() != Some("credit_card_repayment"))
    );

    assert_eq!(
        runtime
            .rescan_local_inbox()
            .expect("repeat local Inbox scan"),
        LocalInboxScanSummary {
            already_present: 0,
            deferred: 0,
            imported: 0,
            suppressed: 0,
        }
    );
    assert_eq!(
        fs::read(&hsbc_path).expect("read unchanged HSBC source"),
        hsbc_bytes
    );
    assert_eq!(
        fs::read(&dbs_path).expect("read unchanged DBS source"),
        dbs_bytes
    );
    assert_eq!(
        runtime
            .list_recent_activity()
            .expect("Activity remains idempotent")
            .len(),
        1
    );
}

#[test]
fn rejects_invalid_profiles_or_legacy_fingerprints_without_parse_persistence() {
    for mismatch in ["unknown_package", "proposal_mismatch", "legacy_fingerprint"] {
        let parent = tempfile::tempdir().expect("temporary app data");
        let source_path = parent.path().join("synthetic.csv");
        fs::write(&source_path, synthetic_statement_csv()).expect("write statement fixture");
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
        let imported = runtime
            .import_selected_document(&source_path)
            .expect("capture statement");
        let mut input = runtime
            .normalization_input(&imported.document_id)
            .expect("extract complete synthetic observations");
        let mut result = synthetic_normalizer_result();
        let NormalizerResult::Classified { profile, .. } = &mut result else {
            panic!("synthetic result must be classified");
        };
        if mismatch == "unknown_package" {
            profile.package_id = "unknown/provider@1".to_owned();
        } else if mismatch == "proposal_mismatch" {
            profile.document_type = "bank_statement".to_owned();
        } else if let Some(marker) = input
            .bundle
            .observations
            .iter_mut()
            .find(|observation| observation.text.contains("CANCAN_SYNTHETIC_STATEMENT_V1"))
        {
            marker.text = "untrusted synthetic statement".to_owned();
        } else {
            panic!("synthetic marker must be present");
        }

        let outcome = runtime
            .apply_normalizer_result(&imported.document_id, &input.bundle, result)
            .expect("fail closed for invalid profile");
        assert_eq!(
            outcome.status,
            crate::database::SourceDocumentRoutingStatus::NeedsAttention
        );
        let state = structured_parse_test_state(&runtime, &imported.document_id);
        assert_eq!(
            (state.parse_runs, state.records, state.open_review_items),
            (0, 0, 0)
        );
    }
}

#[test]
fn rejects_unknown_fields_in_the_normalization_profile_protocol() {
    let mut encoded = serde_json::to_value(synthetic_normalization_profile())
        .expect("serialize normalizer profile");
    encoded
        .as_object_mut()
        .expect("serialized profile")
        .insert("unexpected".to_owned(), serde_json::Value::Bool(true));

    assert!(serde_json::from_value::<NormalizerProfile>(encoded).is_err());
}

#[test]
fn rejects_invalid_normalizer_protocol_without_persisting_records_or_review() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("synthetic.csv");
    fs::write(&source_path, synthetic_statement_csv()).expect("write statement fixture");
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
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("capture statement");
    let input = runtime
        .normalization_input(&imported.document_id)
        .expect("extract complete synthetic observations");
    let mut invalid = synthetic_normalizer_result();
    let NormalizerResult::Classified { proposal, .. } = &mut invalid else {
        panic!("synthetic result must be classified");
    };
    proposal.records[0].validation.raw_grounded = false;

    let outcome = runtime
        .apply_normalizer_result(&imported.document_id, &input, invalid)
        .expect("reject invalid protocol safely");
    assert_eq!(
        outcome.status,
        crate::database::SourceDocumentRoutingStatus::NeedsAttention
    );
    let state = structured_parse_test_state(&runtime, &imported.document_id);
    assert_eq!(
        (state.parse_runs, state.records, state.open_review_items),
        (0, 0, 0)
    );
}

#[test]
fn rejects_invalid_normalizer_posting_status_without_persisting_records_or_review() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("synthetic.csv");
    fs::write(&source_path, synthetic_statement_csv()).expect("write statement fixture");
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
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("capture statement");
    let input = runtime
        .normalization_input(&imported.document_id)
        .expect("extract complete synthetic observations");
    let mut invalid = synthetic_normalizer_result();
    let NormalizerResult::Classified { proposal, .. } = &mut invalid else {
        panic!("synthetic result must be classified");
    };
    proposal.records[0].posting_status = Some("pending".to_owned());

    let outcome = runtime
        .apply_normalizer_result(&imported.document_id, &input, invalid)
        .expect("reject invalid protocol safely");
    assert_eq!(
        outcome.status,
        crate::database::SourceDocumentRoutingStatus::NeedsAttention
    );
    let state = structured_parse_test_state(&runtime, &imported.document_id);
    assert_eq!(
        (state.parse_runs, state.records, state.open_review_items),
        (0, 0, 0)
    );
}

fn structured_parse_test_state(
    runtime: &VaultRuntime,
    document_id: &str,
) -> crate::database::StructuredParseTestState {
    let store = runtime.store().expect("open store");
    store
        .as_ref()
        .expect("unlocked store")
        .structured_parse_test_state(document_id)
        .expect("read structured parse state")
}

fn structured_parse_posting_status(
    runtime: &VaultRuntime,
    document_id: &str,
    stable_record_key: &str,
) -> Option<String> {
    let store = runtime.store().expect("open store");
    store
        .as_ref()
        .expect("unlocked store")
        .structured_parse_posting_status(document_id, stable_record_key)
        .expect("read structured parse posting status")
}

fn expire_reconcile_lease_for_test(runtime: &VaultRuntime, document_id: &str) {
    let store = runtime.store().expect("open store");
    store
        .as_ref()
        .expect("unlocked store")
        .expire_reconcile_lease_for_test(document_id)
        .expect("expire reconcile lease");
}

fn synthetic_normalizer_result() -> NormalizerResult {
    NormalizerResult::Classified {
        profile: Box::new(synthetic_normalization_profile()),
        proposal: Box::new(NormalizerProposal {
            document: NormalizerDocument {
                document_type: "transfer_export".to_owned(),
                provider_key: "synthetic-bank".to_owned(),
                statement_id: Some("transfer-2026-07".to_owned()),
                statement_period: Some(NormalizerStatementPeriod {
                    from: Some("2026-07-01".to_owned()),
                    to: Some("2026-07-31".to_owned()),
                }),
            },
            accounts: vec![
                NormalizerAccount {
                    account_type: "deposit_account".to_owned(),
                    currency: Some("SGD".to_owned()),
                    masked_identifier: Some("••001".to_owned()),
                    proposal_account_id: "account-checking".to_owned(),
                    provider_account_id: Some("checking-001".to_owned()),
                },
                NormalizerAccount {
                    account_type: "deposit_account".to_owned(),
                    currency: Some("SGD".to_owned()),
                    masked_identifier: Some("••002".to_owned()),
                    proposal_account_id: "account-savings".to_owned(),
                    provider_account_id: Some("savings-002".to_owned()),
                },
            ],
            opening_snapshots: vec![
                synthetic_normalizer_record(
                    "record-checking-opening",
                    "account-checking",
                    "balance",
                    "balance_snapshot",
                    None,
                    None,
                    "1000.00",
                ),
                synthetic_normalizer_record(
                    "record-savings-opening",
                    "account-savings",
                    "balance",
                    "balance_snapshot",
                    None,
                    None,
                    "100.00",
                ),
            ],
            records: vec![
                synthetic_normalizer_record(
                    "record-checking-out",
                    "account-checking",
                    "transaction",
                    "same_currency_transfer",
                    Some("250.00"),
                    Some("-250.00"),
                    "750.00",
                ),
                synthetic_normalizer_record(
                    "record-savings-in",
                    "account-savings",
                    "transaction",
                    "same_currency_transfer",
                    Some("250.00"),
                    Some("250.00"),
                    "350.00",
                ),
            ],
            closing_snapshots: vec![
                synthetic_normalizer_record(
                    "record-checking-closing",
                    "account-checking",
                    "balance",
                    "balance_snapshot",
                    None,
                    None,
                    "750.00",
                ),
                synthetic_normalizer_record(
                    "record-savings-closing",
                    "account-savings",
                    "balance",
                    "balance_snapshot",
                    None,
                    None,
                    "350.00",
                ),
            ],
            status: "valid".to_owned(),
        }),
    }
}

fn synthetic_normalization_profile() -> NormalizerProfile {
    NormalizerProfile {
        id: "synthetic-bank-transfer-export-v1".to_owned(),
        provider_key: "synthetic-bank".to_owned(),
        document_type: "transfer_export".to_owned(),
        package_id: "synthetic/bank_transfer_export@1".to_owned(),
        package_version: "1.0.0".to_owned(),
        parser_version: "synthetic-bank-v1".to_owned(),
        skill_version: "synthetic-bank-v1".to_owned(),
        prompt_version: "synthetic-bank-v1".to_owned(),
        schema_version: "structured-proposal-v1".to_owned(),
        validator_version: "synthetic-bank-v1".to_owned(),
        normalizer_runtime: "single-pass-mock".to_owned(),
        tool_contract_version: "synthetic-bank-v1".to_owned(),
        input_strategy: "native-observations-v1".to_owned(),
        extraction_engines: vec![NormalizerProfileExtractionEngine {
            kind: NormalizerProfileExtractionKind::TableCell,
            engine: "rust-csv".to_owned(),
            version: "1.4.0".to_owned(),
        }],
        ocr_engines: Vec::new(),
        model_provider: "cancan-deterministic-mock".to_owned(),
        model: "fixture-v1".to_owned(),
        review_only: true,
    }
}

#[cfg(target_os = "macos")]
fn synthetic_pdf_normalizer_result() -> NormalizerResult {
    let mut result = synthetic_normalizer_result();
    let NormalizerResult::Classified { profile, .. } = &mut result else {
        panic!("synthetic result must be classified");
    };
    profile.extraction_engines = vec![NormalizerProfileExtractionEngine {
        kind: NormalizerProfileExtractionKind::NativeText,
        engine: "pdfkit".to_owned(),
        version: "macos-page-string-v1".to_owned(),
    }];
    result
}

#[cfg(target_os = "macos")]
fn dbs_bank_normalizer_result() -> NormalizerResult {
    NormalizerResult::Classified {
        profile: Box::new(dbs_bank_normalization_profile()),
        proposal: Box::new(NormalizerProposal {
            document: NormalizerDocument {
                document_type: "bank_statement".to_owned(),
                provider_key: "dbs".to_owned(),
                statement_id: Some("dbs-bank_statement-2026-07".to_owned()),
                statement_period: Some(NormalizerStatementPeriod {
                    from: Some("2026-07-01".to_owned()),
                    to: Some("2026-07-03".to_owned()),
                }),
            },
            accounts: vec![NormalizerAccount {
                account_type: "deposit_account".to_owned(),
                currency: Some("SGD".to_owned()),
                masked_identifier: Some("••6789".to_owned()),
                proposal_account_id: "dbs-account".to_owned(),
                provider_account_id: Some("DBS-123456789".to_owned()),
            }],
            opening_snapshots: vec![synthetic_normalizer_record(
                "dbs-opening",
                "dbs-account",
                "balance",
                "balance_snapshot",
                None,
                None,
                "100.00",
            )],
            records: vec![synthetic_normalizer_record(
                "dbs-posting",
                "dbs-account",
                "transaction",
                "same_currency_transfer",
                Some("20.00"),
                Some("-20.00"),
                "80.00",
            )],
            closing_snapshots: vec![synthetic_normalizer_record(
                "dbs-closing",
                "dbs-account",
                "balance",
                "balance_snapshot",
                None,
                None,
                "80.00",
            )],
            status: "valid".to_owned(),
        }),
    }
}

#[cfg(target_os = "macos")]
fn dbs_bank_normalization_profile() -> NormalizerProfile {
    provider_pdf_normalization_profile("dbs", "bank_statement", "dbs/bank_statement@1")
}

#[cfg(target_os = "macos")]
fn hsbc_bank_repayment_normalizer_result() -> NormalizerResult {
    NormalizerResult::Classified {
        profile: Box::new(provider_pdf_normalization_profile(
            "hsbc",
            "bank_statement",
            "hsbc/bank_statement@1",
        )),
        proposal: Box::new(NormalizerProposal {
            document: NormalizerDocument {
                document_type: "bank_statement".to_owned(),
                provider_key: "hsbc".to_owned(),
                statement_id: Some("hsbc-bank_statement-2026-06".to_owned()),
                statement_period: Some(NormalizerStatementPeriod {
                    from: Some("2026-06-29".to_owned()),
                    to: Some("2026-06-30".to_owned()),
                }),
            },
            accounts: vec![NormalizerAccount {
                account_type: "deposit_account".to_owned(),
                currency: Some("SGD".to_owned()),
                masked_identifier: Some("••6789".to_owned()),
                proposal_account_id: "hsbc-bank-account".to_owned(),
                provider_account_id: Some("HSBC-123456789".to_owned()),
            }],
            opening_snapshots: vec![provider_balance_normalizer_record(
                "hsbc-opening",
                "hsbc-bank-account",
                "2026-06-29",
                "100.00",
                "opening_balance",
                "hsbc:2026-06:opening",
                1,
            )],
            records: vec![provider_repayment_normalizer_record(
                ProviderRepaymentRecord {
                    proposal_record_id: "hsbc-dbs-card-payment",
                    proposal_account_id: "hsbc-bank-account",
                    posted_on: "2026-06-30",
                    description: "DBS CARD PAYMENT",
                    side: "debit",
                    amount: "20.00",
                    account_balance_delta: "-20.00",
                    balance_after: "80.00",
                    stable_record_key: "hsbc:2026-06:dbs-card-payment",
                    row: 2,
                },
            )],
            closing_snapshots: vec![provider_balance_normalizer_record(
                "hsbc-closing",
                "hsbc-bank-account",
                "2026-06-30",
                "80.00",
                "closing_balance",
                "hsbc:2026-06:closing",
                3,
            )],
            status: "valid".to_owned(),
        }),
    }
}

#[cfg(target_os = "macos")]
fn dbs_card_repayment_normalizer_result() -> NormalizerResult {
    NormalizerResult::Classified {
        profile: Box::new(provider_pdf_normalization_profile(
            "dbs",
            "credit_card_statement",
            "dbs/credit_card_statement@1",
        )),
        proposal: Box::new(NormalizerProposal {
            document: NormalizerDocument {
                document_type: "credit_card_statement".to_owned(),
                provider_key: "dbs".to_owned(),
                statement_id: Some("dbs-credit_card_statement-2026-07".to_owned()),
                statement_period: Some(NormalizerStatementPeriod {
                    from: Some("2026-07-01".to_owned()),
                    to: Some("2026-07-01".to_owned()),
                }),
            },
            accounts: vec![NormalizerAccount {
                account_type: "credit_card".to_owned(),
                currency: Some("SGD".to_owned()),
                masked_identifier: Some("••6789".to_owned()),
                proposal_account_id: "dbs-card-account".to_owned(),
                provider_account_id: Some("DBS-123456789".to_owned()),
            }],
            opening_snapshots: vec![provider_balance_normalizer_record(
                "dbs-card-opening",
                "dbs-card-account",
                "2026-07-01",
                "100.00",
                "opening_balance",
                "dbs-card:2026-07:opening",
                1,
            )],
            records: vec![provider_repayment_normalizer_record(
                ProviderRepaymentRecord {
                    proposal_record_id: "dbs-card-payment-thank-you",
                    proposal_account_id: "dbs-card-account",
                    posted_on: "2026-07-01",
                    description: "PAYMENT - THANK YOU",
                    side: "credit",
                    amount: "20.00",
                    account_balance_delta: "-20.00",
                    balance_after: "80.00",
                    stable_record_key: "dbs-card:2026-07:payment-thank-you",
                    row: 2,
                },
            )],
            closing_snapshots: vec![provider_balance_normalizer_record(
                "dbs-card-closing",
                "dbs-card-account",
                "2026-07-01",
                "80.00",
                "closing_balance",
                "dbs-card:2026-07:closing",
                3,
            )],
            status: "valid".to_owned(),
        }),
    }
}

#[cfg(target_os = "macos")]
fn provider_pdf_normalization_profile(
    provider_key: &str,
    document_type: &str,
    package_id: &str,
) -> NormalizerProfile {
    NormalizerProfile {
        id: format!(
            "mock:{package_id}:native-observations-v1:extract-native_text-pdfkit-macos-page-string-v1"
        ),
        provider_key: provider_key.to_owned(),
        document_type: document_type.to_owned(),
        package_id: package_id.to_owned(),
        package_version: "1.0.0".to_owned(),
        parser_version: "1.0.0".to_owned(),
        skill_version: "1.0.0".to_owned(),
        prompt_version: "1.0.0".to_owned(),
        schema_version: "1.0.0".to_owned(),
        validator_version: "1.0.0".to_owned(),
        normalizer_runtime: "single-pass-mock".to_owned(),
        tool_contract_version: "1.0.0".to_owned(),
        input_strategy: "native-observations-v1".to_owned(),
        extraction_engines: vec![NormalizerProfileExtractionEngine {
            kind: NormalizerProfileExtractionKind::NativeText,
            engine: "pdfkit".to_owned(),
            version: "macos-page-string-v1".to_owned(),
        }],
        ocr_engines: Vec::new(),
        model_provider: "cancan-deterministic-mock".to_owned(),
        model: "fixture-v1".to_owned(),
        review_only: true,
    }
}

#[cfg(target_os = "macos")]
fn provider_balance_normalizer_record(
    proposal_record_id: &str,
    proposal_account_id: &str,
    posted_on: &str,
    balance_after: &str,
    kind: &str,
    stable_record_key: &str,
    row: u8,
) -> NormalizerRecord {
    NormalizerRecord {
        account_balance_delta: None,
        amount: None,
        balance_after: Some(NormalizerMoney {
            currency: "SGD".to_owned(),
            value: balance_after.to_owned(),
        }),
        description_normalized: None,
        description_raw: None,
        event_type: None,
        instrument_symbol: None,
        posted_at: None,
        posted_on: Some(posted_on.to_owned()),
        posting_status: None,
        proposal_account_id: Some(proposal_account_id.to_owned()),
        proposal_record_id: proposal_record_id.to_owned(),
        provider_record_id: None,
        quantity: None,
        raw: serde_json::json!({
            "kind": kind,
            "date": posted_on,
            "balance": balance_after,
            "locator": { "row": row },
        }),
        record_type: "balance".to_owned(),
        stable_record_key: stable_record_key.to_owned(),
        statement_entry_side: None,
        transaction_on: None,
        validation: NormalizerRecordValidation {
            deterministic_validation_passed: true,
            raw_grounded: true,
            schema_valid: true,
        },
        valuation: None,
    }
}

#[cfg(target_os = "macos")]
struct ProviderRepaymentRecord<'a> {
    proposal_record_id: &'a str,
    proposal_account_id: &'a str,
    posted_on: &'a str,
    description: &'a str,
    side: &'a str,
    amount: &'a str,
    account_balance_delta: &'a str,
    balance_after: &'a str,
    stable_record_key: &'a str,
    row: u8,
}

#[cfg(target_os = "macos")]
fn provider_repayment_normalizer_record(input: ProviderRepaymentRecord<'_>) -> NormalizerRecord {
    let money = |value: &str| NormalizerMoney {
        currency: "SGD".to_owned(),
        value: value.to_owned(),
    };
    let raw = if input.side == "debit" {
        serde_json::json!({
            "kind": "posting",
            "date": input.posted_on,
            "description": input.description,
            "debit": input.amount,
            "balance": input.balance_after,
            "locator": { "row": input.row },
        })
    } else {
        serde_json::json!({
            "kind": "posting",
            "date": input.posted_on,
            "description": input.description,
            "credit": input.amount,
            "balance": input.balance_after,
            "locator": { "row": input.row },
        })
    };
    NormalizerRecord {
        account_balance_delta: Some(money(input.account_balance_delta)),
        amount: Some(money(input.amount)),
        balance_after: Some(money(input.balance_after)),
        description_normalized: Some(input.description.to_owned()),
        description_raw: Some(input.description.to_owned()),
        event_type: Some("credit_card_repayment".to_owned()),
        instrument_symbol: None,
        posted_at: None,
        posted_on: Some(input.posted_on.to_owned()),
        posting_status: Some("posted".to_owned()),
        proposal_account_id: Some(input.proposal_account_id.to_owned()),
        proposal_record_id: input.proposal_record_id.to_owned(),
        provider_record_id: None,
        quantity: None,
        raw,
        record_type: "transaction".to_owned(),
        stable_record_key: input.stable_record_key.to_owned(),
        statement_entry_side: Some(input.side.to_owned()),
        transaction_on: None,
        validation: NormalizerRecordValidation {
            deterministic_validation_passed: true,
            raw_grounded: true,
            schema_valid: true,
        },
        valuation: None,
    }
}

#[cfg(target_os = "macos")]
fn test_prepared_repayment_event(
    first: &CoreReviewRecord,
    second: &CoreReviewRecord,
) -> CorePreparedReviewEvent {
    let event_date = [first, second]
        .into_iter()
        .find(|record| record.account_type != "credit_card")
        .expect("repayment has one cash-like record")
        .posted_on
        .clone();
    let mut records = [first, second];
    records.sort_by(|left, right| left.id.cmp(&right.id));
    CorePreparedReviewEvent {
        event_class: "posting".to_owned(),
        event_date,
        event_type: "credit_card_repayment".to_owned(),
        legs: records
            .iter()
            .map(|record| crate::database::CoreReviewLeg {
                account_id: record.account_id.clone(),
                amount_value: record.account_balance_delta.clone(),
                currency: record.currency.clone(),
                instrument_id: record.instrument_id.clone(),
            })
            .collect(),
        source_record_ids: records.iter().map(|record| record.id.clone()).collect(),
        spending: false,
    }
}

fn synthetic_normalizer_record(
    proposal_record_id: &str,
    proposal_account_id: &str,
    record_type: &str,
    event_type: &str,
    amount: Option<&str>,
    account_balance_delta: Option<&str>,
    balance_after: &str,
) -> NormalizerRecord {
    let money = |value: &str| NormalizerMoney {
        currency: "SGD".to_owned(),
        value: value.to_owned(),
    };
    NormalizerRecord {
        account_balance_delta: account_balance_delta.map(money),
        amount: amount.map(money),
        balance_after: Some(money(balance_after)),
        description_normalized: None,
        description_raw: None,
        event_type: Some(event_type.to_owned()),
        instrument_symbol: None,
        posted_at: None,
        posted_on: Some("2026-07-01".to_owned()),
        posting_status: None,
        proposal_account_id: Some(proposal_account_id.to_owned()),
        proposal_record_id: proposal_record_id.to_owned(),
        provider_record_id: None,
        quantity: None,
        raw: serde_json::json!({ "proposalRecordId": proposal_record_id }),
        record_type: record_type.to_owned(),
        stable_record_key: format!("synthetic:{proposal_record_id}"),
        statement_entry_side: None,
        transaction_on: None,
        validation: NormalizerRecordValidation {
            deterministic_validation_passed: true,
            raw_grounded: true,
            schema_valid: true,
        },
        valuation: None,
    }
}

fn synthetic_statement_csv() -> String {
    [
        "balance,2026-06-30,checking-001,1000.00,SGD,CANCAN_SYNTHETIC_STATEMENT_V1",
        "2026-07-01,Transfer to savings,250.00,SGD,750.00,provider=synthetic-bank",
        "balance,2026-07-01,checking-001,750.00,SGD,statement_id=transfer-2026-07",
        "balance,2026-06-30,savings-002,100.00,SGD",
        "2026-07-01,Transfer from checking,250.00,SGD,350.00",
        "balance,2026-07-01,savings-002,350.00,SGD",
    ]
    .join("\n")
}

#[cfg(target_os = "macos")]
#[test]
fn extracts_native_pdf_and_csv_observations_without_creating_files() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let pdf_path = parent.path().join("statement.pdf");
    let csv_path = parent.path().join("statement.csv");
    fs::write(&pdf_path, synthetic_pdf()).expect("write text-layer PDF fixture");
    fs::write(&csv_path, b"date,memo\n2026-07-23,coffee\n").expect("write CSV fixture");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::new(vault_root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let pdf = runtime
        .import_selected_document(&pdf_path)
        .expect("import PDF");
    let csv = runtime
        .import_selected_document(&csv_path)
        .expect("import CSV");
    let vault_before = vault_entries(&vault_root);
    let mut temporary_entries_before = fs::read_dir(parent.path())
        .expect("read temporary directory")
        .map(|entry| entry.expect("read temporary entry").path())
        .collect::<Vec<_>>();
    temporary_entries_before.sort();

    let pdf_bundle = runtime
        .normalization_input(&pdf.document_id)
        .expect("extract native PDF observations");
    let csv_bundle = runtime
        .normalization_input(&csv.document_id)
        .expect("extract CSV observations");

    assert_eq!(pdf_bundle.observations.len(), 1);
    let pdf_observation = &pdf_bundle.observations[0];
    assert_eq!(
        pdf_observation.kind,
        crate::source_observations::SourceObservationKind::NativeText
    );
    assert_eq!(pdf_observation.page, Some(1));
    assert!(
        pdf_observation
            .text
            .contains("CANCAN_SYNTHETIC_STATEMENT_V1")
    );
    assert_eq!(
        pdf_observation.text_span.as_ref().map(|span| span.start),
        Some(0)
    );
    assert_eq!(
        pdf_observation.text_span.as_ref().map(|span| span.end),
        Some(pdf_observation.text.encode_utf16().count() as u64)
    );
    assert_eq!(pdf_observation.engine, "pdfkit");
    assert_eq!(pdf_observation.engine_version, "macos-page-string-v1");

    assert_eq!(csv_bundle.observations.len(), 4);
    assert_eq!(csv_bundle.observations[0].id, "csv-row-1-column-1");
    assert_eq!(csv_bundle.observations[3].text, "coffee");

    assert_eq!(vault_entries(&vault_root), vault_before);
    let mut temporary_entries_after = fs::read_dir(parent.path())
        .expect("read temporary directory")
        .map(|entry| entry.expect("read temporary entry").path())
        .collect::<Vec<_>>();
    temporary_entries_after.sort();
    assert_eq!(temporary_entries_after, temporary_entries_before);
}

#[cfg(target_os = "macos")]
pub(super) fn protected_text_pdf() -> Vec<u8> {
    use objc2::{AllocAnyThread, rc::Retained};
    use objc2_foundation::{NSData, NSDictionary, NSString};
    use objc2_pdf_kit::{
        PDFDocument, PDFDocumentOwnerPasswordOption, PDFDocumentUserPasswordOption,
    };

    let plaintext = synthetic_pdf_with_stream(
        "BT /F1 10 Tf 8 72 Td (CANCAN_SYNTHETIC_STATEMENT_V1 provider=synthetic-bank statement_id=transfer-2026-07 Caf\\351) Tj ET",
    );
    let data = NSData::with_bytes(&plaintext);
    let document = unsafe { PDFDocument::initWithData(PDFDocument::alloc(), &data) }
        .expect("open text-layer PDF fixture");
    let user_password = NSString::from_str("statement-password");
    let owner_password = NSString::from_str("owner-password");
    // PDFKit exports these immutable option-name constants for process lifetime.
    let option_keys = unsafe {
        [
            PDFDocumentUserPasswordOption,
            PDFDocumentOwnerPasswordOption,
        ]
    };
    let options = NSDictionary::from_slices(&option_keys, &[&*user_password, &*owner_password]);
    // Objective-C lightweight generics are erased at runtime; PDFKit's generated
    // signature uses an untyped NSDictionary even though these keys and values are strings.
    let options: Retained<NSDictionary> = unsafe { Retained::cast_unchecked(options) };
    unsafe { document.dataRepresentationWithOptions(&options) }
        .expect("encrypt text-layer PDF fixture")
        .to_vec()
}

fn synthetic_pdf() -> Vec<u8> {
    synthetic_pdf_with_stream("BT /F1 10 Tf 8 72 Td (CANCAN_SYNTHETIC_STATEMENT_V1) Tj ET")
}

#[cfg(target_os = "macos")]
fn synthetic_provider_statement_pdf() -> Vec<u8> {
    synthetic_pdf_with_stream(
        "BT /F1 10 Tf 8 72 Td (CANCAN_SYNTHETIC_PROVIDER_STATEMENT_V1 provider=dbs document_type=bank_statement package_id=dbs/bank_statement@1 statement_id=dbs-bank_statement-2026-07 DBS Statement of Account WITHDRAWAL DEPOSIT BALANCE Account number DBS-123456789 Statement currency SGD opening_balance 2026-07-01 100.00 posting 2026-07-02 GROCERIES 20.00 80.00 closing_balance 2026-07-03 80.00) Tj ET",
    )
}

#[cfg(target_os = "macos")]
fn hsbc_repayment_statement_pdf() -> Vec<u8> {
    synthetic_pdf_with_stream(
        "BT /F1 10 Tf 8 72 Td (CANCAN\\137SYNTHETIC\\137PROVIDER\\137STATEMENT\\137V1 provider=hsbc document\\137type=bank\\137statement package\\137id=hsbc/bank\\137statement@1 statement\\137id=hsbc-bank\\137statement-2026-06 HSBC ACCOUNT STATEMENT WITHDRAWAL DEPOSIT BALANCE Account number HSBC-123456789 Statement currency SGD opening\\137balance 2026-06-29 100.00 posting 2026-06-30 DBS CARD PAYMENT debit 20.00 balance 80.00 closing\\137balance 2026-06-30 80.00) Tj ET",
    )
}

#[cfg(target_os = "macos")]
fn dbs_card_repayment_statement_pdf() -> Vec<u8> {
    synthetic_pdf_with_stream(
        "BT /F1 10 Tf 8 72 Td (CANCAN\\137SYNTHETIC\\137PROVIDER\\137STATEMENT\\137V1 provider=dbs document\\137type=credit\\137card\\137statement package\\137id=dbs/credit\\137card\\137statement@1 statement\\137id=dbs-credit\\137card\\137statement-2026-07 DBS CREDIT LIMIT PAYMENT DUE DATE PREVIOUS BALANCE Account number DBS-123456789 Statement currency SGD opening\\137balance 2026-07-01 100.00 posting 2026-07-01 PAYMENT - THANK YOU credit 20.00 balance 80.00 closing\\137balance 2026-07-01 80.00) Tj ET",
    )
}

fn synthetic_pdf_with_stream(text: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 4096 96] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{text}\nendstream", text.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        write!(&mut pdf, "{} 0 obj\n{}\nendobj\n", index + 1, object).expect("write PDF object");
    }
    let xref = pdf.len();
    write!(
        &mut pdf,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    )
    .expect("write xref");
    for offset in offsets {
        writeln!(&mut pdf, "{offset:010} 00000 n ").expect("write xref entry");
    }
    write!(
        &mut pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("write trailer");
    pdf
}

fn vault_entries(root: &Path) -> Vec<PathBuf> {
    fn visit(root: &Path, path: &Path, entries: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(path).expect("read Vault directory") {
            let entry = entry.expect("read Vault entry");
            let entry_path = entry.path();
            entries.push(
                entry_path
                    .strip_prefix(root)
                    .expect("Vault-relative path")
                    .to_owned(),
            );
            if entry.file_type().expect("Vault entry type").is_dir() {
                visit(root, &entry_path, entries);
            }
        }
    }

    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort();
    entries
}
