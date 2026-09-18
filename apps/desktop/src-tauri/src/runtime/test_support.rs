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

/// Records what would reach the user's Notification Center, so the
/// delivery-eligibility rules are asserted without touching the system
/// notification center.
pub(super) struct RecordingIntakeNotificationDelivery {
    permission: Mutex<IntakeNotificationPermission>,
    permission_requests: std::sync::atomic::AtomicUsize,
    failed_deliveries: std::sync::atomic::AtomicUsize,
    delivered: Mutex<Vec<(String, String, String)>>,
}

impl RecordingIntakeNotificationDelivery {
    pub(super) fn new(permission: IntakeNotificationPermission) -> Self {
        Self {
            permission: Mutex::new(permission),
            permission_requests: std::sync::atomic::AtomicUsize::new(0),
            failed_deliveries: std::sync::atomic::AtomicUsize::new(0),
            delivered: Mutex::new(Vec::new()),
        }
    }

    /// Refuses the next `count` deliveries, the way a notification center that
    /// cannot accept the request would.
    pub(super) fn fail_next_deliveries(&self, count: usize) {
        self.failed_deliveries
            .store(count, std::sync::atomic::Ordering::SeqCst);
    }

    pub(super) fn permission_requests(&self) -> usize {
        self.permission_requests
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// The delivered `(identifier, title, body)` of every accepted request.
    pub(super) fn delivered(&self) -> Vec<(String, String, String)> {
        self.delivered
            .lock()
            .expect("delivery recorder lock")
            .clone()
    }
}

impl IntakeNotificationDelivery for RecordingIntakeNotificationDelivery {
    fn permission(&self) -> IntakeNotificationPermission {
        *self.permission.lock().expect("permission lock")
    }

    fn request_permission(&self) -> IntakeNotificationPermission {
        self.permission_requests
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut permission = self.permission.lock().expect("permission lock");
        if *permission == IntakeNotificationPermission::NotDetermined {
            // The system asks the user once; the recorder answers the way an
            // accepted prompt would.
            *permission = IntakeNotificationPermission::Authorized;
        }
        *permission
    }

    fn deliver(&self, notification: &IntakeNotification) -> Result<(), RuntimeError> {
        if self
            .failed_deliveries
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |remaining| remaining.checked_sub(1),
            )
            .is_ok()
        {
            return Err(RuntimeError::new("intake_notification_test_refused"));
        }
        self.delivered
            .lock()
            .expect("delivery recorder lock")
            .push((
                notification.identifier.clone(),
                notification.title.clone(),
                notification.body.clone(),
            ));
        Ok(())
    }
}

/// A Vault whose notification delivery is a recorder instead of the system.
/// The Vault itself is left uncreated, so a test can either create it or open
/// an existing one.
pub(super) fn notification_runtime(
    root: &Path,
    delivery: Arc<RecordingIntakeNotificationDelivery>,
) -> VaultRuntime {
    VaultRuntime::with_all_secret_stores(
        root.to_path_buf(),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
        Arc::new(KeychainGmailRefreshTokenStore::production()),
        delivery,
    )
}
