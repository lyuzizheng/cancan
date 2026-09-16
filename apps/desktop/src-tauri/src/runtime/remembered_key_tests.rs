use super::test_support::{MemoryRememberedKeyStore, poisoned_keychain_lock};
use super::*;

struct FailingRememberedKeyStore;

impl RememberedKeyStore for FailingRememberedKeyStore {
    fn delete(&self) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }

    fn is_present(&self) -> Result<bool, SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        Ok(None)
    }

    fn save(&self, _secret: &[u8]) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }
}

struct PresenceOnlyRememberedKeyStore;

impl RememberedKeyStore for PresenceOnlyRememberedKeyStore {
    fn delete(&self) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }

    fn is_present(&self) -> Result<bool, SecretStoreError> {
        Ok(true)
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        panic!("status must not load the remembered secret")
    }

    fn save(&self, _secret: &[u8]) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }
}

struct MalformedDeleteFailingRememberedKeyStore;

impl RememberedKeyStore for MalformedDeleteFailingRememberedKeyStore {
    fn delete(&self) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }

    fn is_present(&self) -> Result<bool, SecretStoreError> {
        Ok(true)
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        Ok(Some(Zeroizing::new(vec![0x55; KEY_LEN - 1])))
    }

    fn save(&self, _secret: &[u8]) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }
}

/// Simulates a Touch ID item the OS invalidated after the enrolled fingerprint
/// set changed: `load` errors, the marker is still present, and `delete`
/// cleans both items up.
struct InvalidatedRememberedKeyStore {
    deleted: Mutex<bool>,
}

impl InvalidatedRememberedKeyStore {
    fn new() -> Self {
        Self {
            deleted: Mutex::new(false),
        }
    }
}

impl RememberedKeyStore for InvalidatedRememberedKeyStore {
    fn delete(&self) -> Result<(), SecretStoreError> {
        *self.deleted.lock().map_err(|_| poisoned_keychain_lock())? = true;
        Ok(())
    }

    fn is_present(&self) -> Result<bool, SecretStoreError> {
        Ok(!*self.deleted.lock().map_err(|_| poisoned_keychain_lock())?)
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }

    fn invalidated(&self) -> bool {
        true
    }

    fn save(&self, _secret: &[u8]) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }
}

/// Simulates a transient read failure (user cancelled the Touch ID prompt or
/// authentication is temporarily unavailable): `load` errors, the marker is
/// still present, and the item is not invalidated, so the offer must survive.
struct TransientFailureRememberedKeyStore;

impl RememberedKeyStore for TransientFailureRememberedKeyStore {
    fn delete(&self) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }

    fn is_present(&self) -> Result<bool, SecretStoreError> {
        Ok(true)
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }

    fn save(&self, _secret: &[u8]) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::new("test keychain failure"))
    }
}

#[test]
fn clears_an_os_invalidated_touch_id_item_and_keeps_password_fallback() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let remembered_keys = Arc::new(MemoryRememberedKeyStore::default());
    let setup = VaultRuntime::with_remembered_keys(root.clone(), remembered_keys);
    setup
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    setup.remember_on_this_mac().expect("remember on this Mac");
    setup.test_support_lock().expect("lock Vault");
    drop(setup);

    let invalidated = Arc::new(InvalidatedRememberedKeyStore::new());
    let runtime = VaultRuntime::with_remembered_keys(root.clone(), invalidated.clone());
    assert_eq!(
        runtime.access_status().expect("status before unlock"),
        VaultAccessStatus {
            recovery_configured: false,
            remembered_on_this_mac: Some(true),
            status: VaultStatus::Locked,
        }
    );

    // The invalidated item errors on load; the runtime clears both items so the
    // stale Touch ID offer disappears, then the Vault password still unlocks.
    assert_eq!(
        runtime
            .unlock_with_keychain()
            .expect_err("invalidated Touch ID item")
            .code(),
        "remembered_unlock_unavailable"
    );
    assert!(*invalidated.deleted.lock().expect("deleted flag"));
    assert_eq!(
        runtime.access_status().expect("status after cleanup"),
        VaultAccessStatus {
            recovery_configured: false,
            remembered_on_this_mac: Some(false),
            status: VaultStatus::Locked,
        }
    );
    assert_eq!(
        runtime
            .unlock(b"synthetic-vault-password")
            .expect("password fallback"),
        VaultStatus::Unlocked
    );
}

#[test]
fn keeps_the_touch_id_offer_when_the_read_fails_transiently() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let remembered_keys = Arc::new(MemoryRememberedKeyStore::default());
    let setup = VaultRuntime::with_remembered_keys(root.clone(), remembered_keys);
    setup
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    setup.remember_on_this_mac().expect("remember on this Mac");
    setup.test_support_lock().expect("lock Vault");
    drop(setup);

    // A transient failure (user cancel / authentication unavailable) errors on
    // load without being invalidated, so the marker must survive and the retry
    // stays offered.
    let runtime =
        VaultRuntime::with_remembered_keys(root, Arc::new(TransientFailureRememberedKeyStore));
    assert_eq!(
        runtime
            .unlock_with_keychain()
            .expect_err("transient Touch ID failure")
            .code(),
        "remembered_unlock_failed"
    );
    assert_eq!(
        runtime.access_status().expect("marker kept"),
        VaultAccessStatus {
            recovery_configured: false,
            remembered_on_this_mac: Some(true),
            status: VaultStatus::Locked,
        }
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
    setup.test_support_lock().expect("lock Vault");
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
    runtime.test_support_lock().expect("lock Vault");
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
    setup.test_support_lock().expect("lock Vault");
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
    runtime.test_support_lock().expect("lock Vault");
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
