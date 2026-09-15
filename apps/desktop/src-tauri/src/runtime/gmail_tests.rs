use super::*;
use crate::database::{GmailAccountState, GmailAccountStatus};
use sha2::{Digest, Sha256};

#[derive(Default)]
struct MemoryGmailRefreshTokenStore {
    fail_delete_refs: Mutex<HashSet<String>>,
    fail_save: AtomicBool,
    secrets: Mutex<HashMap<String, Vec<u8>>>,
}

impl GmailRefreshTokenStore for MemoryGmailRefreshTokenStore {
    fn delete(&self, secret_ref: &str) -> Result<(), ()> {
        if self
            .fail_delete_refs
            .lock()
            .map_err(|_| ())?
            .contains(secret_ref)
        {
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

fn gmail_runtime(
    root: &Path,
    gmail_refresh_tokens: Arc<MemoryGmailRefreshTokenStore>,
) -> VaultRuntime {
    let runtime = VaultRuntime::with_all_secret_stores(
        root.to_path_buf(),
        Arc::new(super::tests::MemoryRememberedKeyStore::default()),
        Arc::new(super::tests::MemoryStatementPasswordStore::default()),
        Arc::new(super::tests::MemoryLocalInboxBookmarkStore::default()),
        gmail_refresh_tokens,
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
}

fn gmail_state(runtime: &VaultRuntime, mailbox: &str) -> GmailAccountState {
    runtime
        .store()
        .expect("active store")
        .as_ref()
        .expect("unlocked store")
        .gmail_account_state(mailbox)
        .expect("read Gmail state")
        .expect("Gmail account exists")
}

#[test]
fn connects_reconnects_and_isolates_gmail_mailboxes() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let tokens = Arc::new(MemoryGmailRefreshTokenStore::default());
    let runtime = gmail_runtime(&parent.path().join("vault"), tokens.clone());

    runtime
        .persist_gmail_mailbox(" Owner@Example.com ", b"first-token")
        .expect("connect first mailbox");
    let first = gmail_state(&runtime, "owner@example.com");
    assert_eq!(first.status, GmailAccountStatus::Connected);
    assert!(!first.id.contains("owner@example.com"));
    assert!(!first.secret_storage_key.contains("owner@example.com"));

    runtime
        .persist_gmail_mailbox("owner@example.com", b"replacement-token")
        .expect("reconnect first mailbox");
    let reconnected = gmail_state(&runtime, "owner@example.com");
    assert_eq!(reconnected.id, first.id);
    assert_eq!(reconnected.secret_storage_key, first.secret_storage_key);
    assert_eq!(
        tokens
            .load(&first.secret_storage_key)
            .expect("load replacement token")
            .expect("replacement token exists")
            .as_slice(),
        b"replacement-token"
    );

    runtime
        .persist_gmail_mailbox("second@example.com", b"second-token")
        .expect("connect second mailbox");
    let second = gmail_state(&runtime, "second@example.com");
    assert_ne!(second.id, first.id);
    assert_ne!(second.secret_storage_key, first.secret_storage_key);
}

#[test]
fn keeps_failed_cross_store_transitions_recoverable() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let tokens = Arc::new(MemoryGmailRefreshTokenStore::default());
    let runtime = gmail_runtime(&parent.path().join("vault"), tokens.clone());

    tokens.fail_save.store(true, Ordering::SeqCst);
    assert_eq!(
        runtime
            .persist_gmail_mailbox("owner@example.com", b"token")
            .expect_err("surface Keychain save failure")
            .code(),
        "gmail_connection_save_failed"
    );
    assert_eq!(
        gmail_state(&runtime, "owner@example.com").status,
        GmailAccountStatus::PendingSave
    );

    tokens.fail_save.store(false, Ordering::SeqCst);
    runtime
        .persist_gmail_mailbox("owner@example.com", b"token")
        .expect("recover and connect");
    let owner = gmail_state(&runtime, "owner@example.com");
    tokens
        .fail_delete_refs
        .lock()
        .expect("configure delete failure")
        .insert(owner.secret_storage_key.clone());
    assert_eq!(
        runtime
            .disconnect_gmail_mailbox("owner@example.com")
            .expect_err("surface Keychain delete failure")
            .code(),
        "gmail_connection_remove_failed"
    );
    assert_eq!(
        gmail_state(&runtime, "owner@example.com").status,
        GmailAccountStatus::PendingDelete
    );

    runtime
        .persist_gmail_mailbox("second@example.com", b"second-token")
        .expect("unrelated mailbox remains available");
    assert_eq!(
        gmail_state(&runtime, "second@example.com").status,
        GmailAccountStatus::Connected
    );
    assert_eq!(
        gmail_state(&runtime, "owner@example.com").status,
        GmailAccountStatus::PendingDelete
    );

    tokens
        .fail_delete_refs
        .lock()
        .expect("clear delete failure")
        .remove(&owner.secret_storage_key);
    runtime
        .disconnect_gmail_mailbox("owner@example.com")
        .expect("recover interrupted delete");
    assert_eq!(
        gmail_state(&runtime, "owner@example.com").status,
        GmailAccountStatus::Disconnected
    );
}

#[test]
fn unlock_reconciles_tokens_written_before_a_crash() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let tokens = Arc::new(MemoryGmailRefreshTokenStore::default());
    let runtime = gmail_runtime(&parent.path().join("vault"), tokens.clone());
    let mailbox = "owner@example.com";
    let digest = format!("{:x}", Sha256::digest(mailbox.as_bytes()));
    let secret_ref = format!("gmail-refresh-token:{digest}");

    runtime
        .store()
        .expect("active store")
        .as_ref()
        .expect("unlocked store")
        .begin_gmail_account_save(&format!("gmail-account:{digest}"), mailbox, &secret_ref)
        .expect("persist pending save");
    tokens
        .save(&secret_ref, b"unverified-token")
        .expect("simulate Keychain write");
    runtime.test_support_lock().expect("simulate process lock");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("unlock and reconcile");

    assert_eq!(
        gmail_state(&runtime, mailbox).status,
        GmailAccountStatus::Disconnected
    );
    assert!(
        tokens
            .load(&secret_ref)
            .expect("load discarded token")
            .is_none()
    );
}

#[test]
fn unlock_keeps_a_failed_gmail_reconciliation_mailbox_scoped() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let tokens = Arc::new(MemoryGmailRefreshTokenStore::default());
    let runtime = gmail_runtime(&parent.path().join("vault"), tokens.clone());
    runtime
        .persist_gmail_mailbox("owner@example.com", b"token")
        .expect("connect mailbox");
    let account = gmail_state(&runtime, "owner@example.com");
    runtime
        .store()
        .expect("active store")
        .as_ref()
        .expect("unlocked store")
        .begin_gmail_account_delete(&account.id)
        .expect("persist pending delete");
    tokens
        .fail_delete_refs
        .lock()
        .expect("configure delete failure")
        .insert(account.secret_storage_key.clone());
    runtime.test_support_lock().expect("lock Vault");

    runtime
        .unlock(b"synthetic-vault-password")
        .expect("Gmail failure does not block Vault unlock");
    assert_eq!(
        gmail_state(&runtime, "owner@example.com").status,
        GmailAccountStatus::PendingDelete
    );

    tokens
        .fail_delete_refs
        .lock()
        .expect("clear delete failure")
        .remove(&account.secret_storage_key);
    runtime
        .disconnect_gmail_mailbox("owner@example.com")
        .expect("retry mailbox cleanup");
    assert_eq!(
        gmail_state(&runtime, "owner@example.com").status,
        GmailAccountStatus::Disconnected
    );
}

#[test]
fn rejects_gmail_connection_changes_while_locked() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let tokens = Arc::new(MemoryGmailRefreshTokenStore::default());
    let runtime = gmail_runtime(&parent.path().join("vault"), tokens);
    runtime.test_support_lock().expect("lock Vault");

    assert_eq!(
        runtime
            .persist_gmail_mailbox("owner@example.com", b"token")
            .expect_err("reject connect while locked")
            .code(),
        "vault_locked"
    );
    assert_eq!(
        runtime
            .disconnect_gmail_mailbox("owner@example.com")
            .expect_err("reject disconnect while locked")
            .code(),
        "vault_locked"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn production_gmail_refresh_token_store_replaces_and_removes_secret() {
    struct Cleanup {
        secret_ref: String,
        store: KeychainGmailRefreshTokenStore,
    }

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.store.delete(&self.secret_ref);
        }
    }

    let service = format!(
        "{GMAIL_REFRESH_TOKEN_KEYCHAIN_SERVICE}.test.{}",
        candidate_name()
    );
    let secret_ref = random_identifier("gmail-refresh-token-test");
    let store = KeychainGmailRefreshTokenStore::new(service);
    store
        .delete(&secret_ref)
        .expect("remove pre-existing test entry");
    let _cleanup = Cleanup {
        secret_ref: secret_ref.clone(),
        store: store.clone(),
    };

    store
        .save(&secret_ref, b"first-synthetic-refresh-token")
        .expect("save Gmail refresh token");
    store
        .save(&secret_ref, b"updated-synthetic-refresh-token")
        .expect("replace Gmail refresh token");
    assert_eq!(
        store
            .load(&secret_ref)
            .expect("load Gmail refresh token")
            .expect("saved Gmail refresh token exists")
            .as_slice(),
        b"updated-synthetic-refresh-token"
    );
    store
        .delete(&secret_ref)
        .expect("delete Gmail refresh token");
    assert!(
        store
            .load(&secret_ref)
            .expect("load deleted Gmail refresh token")
            .is_none()
    );
}
