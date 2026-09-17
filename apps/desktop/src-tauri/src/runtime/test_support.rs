use super::*;

pub(super) fn statement_password_runtime(
    root: &Path,
    statement_passwords: Arc<dyn StatementPasswordStore>,
) -> VaultRuntime {
    let runtime = VaultRuntime::with_secret_stores(
        root.to_path_buf(),
        Arc::new(MemoryRememberedKeyStore::default()),
        statement_passwords,
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .seed_money_source("source-dbs", "dbs", "DBS", "bank")
        .expect("seed Money Source");
    runtime
}

pub(super) fn statement_password_state(
    runtime: &VaultRuntime,
) -> Option<crate::database::StatementPasswordState> {
    runtime
        .store()
        .expect("active store")
        .as_ref()
        .expect("unlocked store")
        .statement_password_state("source-dbs")
        .expect("statement password state")
}

/// The in-memory keychain fakes have no failure mode other than a poisoned
/// lock, which only happens after another test thread panicked.
pub(super) fn poisoned_keychain_lock() -> SecretStoreError {
    SecretStoreError::new("test keychain lock poisoned")
}

#[derive(Default)]
pub(super) struct MemoryRememberedKeyStore {
    secret: Mutex<Option<Vec<u8>>>,
}

impl RememberedKeyStore for MemoryRememberedKeyStore {
    fn delete(&self) -> Result<(), SecretStoreError> {
        *self.secret.lock().map_err(|_| poisoned_keychain_lock())? = None;
        Ok(())
    }

    fn is_present(&self) -> Result<bool, SecretStoreError> {
        Ok(self
            .secret
            .lock()
            .map_err(|_| poisoned_keychain_lock())?
            .is_some())
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        Ok(self
            .secret
            .lock()
            .map_err(|_| poisoned_keychain_lock())?
            .clone()
            .map(Zeroizing::new))
    }

    fn save(&self, secret: &[u8]) -> Result<(), SecretStoreError> {
        *self.secret.lock().map_err(|_| poisoned_keychain_lock())? = Some(secret.to_vec());
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct MemoryStatementPasswordStore {
    pub(super) fail_delete: AtomicBool,
    pub(super) fail_save: AtomicBool,
    secrets: Mutex<HashMap<String, Vec<u8>>>,
}

impl StatementPasswordStore for MemoryStatementPasswordStore {
    fn delete(&self, secret_ref: &str) -> Result<(), SecretStoreError> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(SecretStoreError::new("test keychain failure"));
        }
        self.secrets
            .lock()
            .map_err(|_| poisoned_keychain_lock())?
            .remove(secret_ref);
        Ok(())
    }

    fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        Ok(self
            .secrets
            .lock()
            .map_err(|_| poisoned_keychain_lock())?
            .get(secret_ref)
            .cloned()
            .map(Zeroizing::new))
    }

    fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), SecretStoreError> {
        if self.fail_save.load(Ordering::SeqCst) {
            return Err(SecretStoreError::new("test keychain failure"));
        }
        self.secrets
            .lock()
            .map_err(|_| poisoned_keychain_lock())?
            .insert(secret_ref.to_owned(), secret.to_vec());
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct MemoryLocalInboxBookmarkStore {
    bookmark: Mutex<Option<Vec<u8>>>,
}

impl LocalInboxBookmarkStore for MemoryLocalInboxBookmarkStore {
    fn delete(&self) -> Result<(), SecretStoreError> {
        *self.bookmark.lock().map_err(|_| poisoned_keychain_lock())? = None;
        Ok(())
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        Ok(self
            .bookmark
            .lock()
            .map_err(|_| poisoned_keychain_lock())?
            .clone()
            .map(Zeroizing::new))
    }

    fn save(&self, bookmark: &[u8]) -> Result<(), SecretStoreError> {
        *self.bookmark.lock().map_err(|_| poisoned_keychain_lock())? = Some(bookmark.to_vec());
        Ok(())
    }
}
