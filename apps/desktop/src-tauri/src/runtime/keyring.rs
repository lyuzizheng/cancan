use super::*;

#[cfg(target_os = "macos")]
use security_framework::{
    access_control::{ProtectionMode, SecAccessControl},
    passwords::{
        AccessControlOptions, PasswordOptions, delete_generic_password,
        delete_generic_password_options, generic_password, set_generic_password_options,
    },
};

pub(super) trait RememberedKeyStore: Send + Sync {
    fn delete(&self) -> Result<(), ()>;
    fn is_present(&self) -> Result<bool, ()>;
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()>;
    fn save(&self, secret: &[u8]) -> Result<(), ()>;
}

pub(super) trait StatementPasswordStore: Send + Sync {
    fn delete(&self, secret_ref: &str) -> Result<(), ()>;
    fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ()>;
    fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), ()>;
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "refresh-token save and load are wired by the next connector execution checkpoint"
    )
)]
pub(super) trait GmailRefreshTokenStore: Send + Sync {
    fn delete(&self, secret_ref: &str) -> Result<(), ()>;
    fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ()>;
    fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), ()>;
}

pub(super) trait LocalInboxBookmarkStore: Send + Sync {
    fn delete(&self) -> Result<(), ()>;
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()>;
    fn save(&self, bookmark: &[u8]) -> Result<(), ()>;
}

#[derive(Clone)]
pub(super) struct KeychainRememberedKeyStore {
    access_control: usize,
    account: String,
    marker_account: String,
    service: String,
}

impl KeychainRememberedKeyStore {
    #[cfg(target_os = "macos")]
    pub(super) fn production() -> Self {
        Self::with_access_control(
            KEYCHAIN_SERVICE,
            KEYCHAIN_ACCOUNT,
            AccessControlOptions::BIOMETRY_CURRENT_SET.bits(),
        )
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn production() -> Self {
        Self::with_access_control(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, 0)
    }

    fn with_access_control(
        service: impl Into<String>,
        account: impl Into<String>,
        access_control: usize,
    ) -> Self {
        let account = account.into();
        let service = service.into();
        let marker_account = format!("{account}-presence");
        Self {
            access_control,
            account,
            marker_account,
            service,
        }
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(super) fn new_without_biometry(
        service: impl Into<String>,
        account: impl Into<String>,
    ) -> Self {
        Self::with_access_control(service, account, 0)
    }

    #[cfg(target_os = "macos")]
    fn delete_options(options: PasswordOptions) -> Result<(), ()> {
        match delete_generic_password_options(options) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(()),
            Err(_) => Err(()),
        }
    }

    #[cfg(target_os = "macos")]
    fn load_marker(&self) -> Result<Vec<u8>, security_framework::base::Error> {
        let mut options =
            PasswordOptions::new_generic_password(&self.service, &self.marker_account);
        options.use_protected_keychain();
        generic_password(options)
    }

    #[cfg(target_os = "macos")]
    fn load_secret(&self) -> Result<Option<Vec<u8>>, ()> {
        let mut options = PasswordOptions::new_generic_password(&self.service, &self.account);
        options.use_protected_keychain();
        match generic_password(options) {
            Ok(secret) => Ok(Some(secret)),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(None),
            Err(_) => Err(()),
        }
    }

    #[cfg(target_os = "macos")]
    fn save_marker(&self) -> Result<(), ()> {
        let mut options =
            PasswordOptions::new_generic_password(&self.service, &self.marker_account);
        options.use_protected_keychain();
        options.set_access_synchronized(Some(false));
        let access = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            AccessControlOptions::empty().bits(),
        )
        .map_err(|_| ())?;
        options.set_access_control(access);
        set_generic_password_options(&[1], options).map_err(|_| ())
    }

    #[cfg(target_os = "macos")]
    fn save_secret(&self, secret: &[u8]) -> Result<(), ()> {
        let mut options = PasswordOptions::new_generic_password(&self.service, &self.account);
        options.use_protected_keychain();
        options.set_access_synchronized(Some(false));
        let access = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            self.access_control,
        )
        .map_err(|_| ())?;
        options.set_access_control(access);
        set_generic_password_options(secret, options).map_err(|_| ())
    }
}

impl RememberedKeyStore for KeychainRememberedKeyStore {
    #[cfg(target_os = "macos")]
    fn delete(&self) -> Result<(), ()> {
        let mut options = PasswordOptions::new_generic_password(&self.service, &self.account);
        options.use_protected_keychain();
        let protected_deleted = Self::delete_options(options);

        let mut options =
            PasswordOptions::new_generic_password(&self.service, &self.marker_account);
        options.use_protected_keychain();
        let marker_deleted = Self::delete_options(options);

        // Clean up any pre-biometry item stored in the legacy file-based keychain.
        let legacy_deleted = match delete_generic_password(&self.service, &self.account) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(()),
            Err(_) => Err(()),
        };

        protected_deleted.and(marker_deleted).and(legacy_deleted)
    }

    #[cfg(not(target_os = "macos"))]
    fn delete(&self) -> Result<(), ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn is_present(&self) -> Result<bool, ()> {
        match self.load_marker() {
            Ok(_) => Ok(true),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(false),
            Err(_) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn is_present(&self) -> Result<bool, ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        match self.load_secret() {
            Ok(Some(secret)) => Ok(Some(Zeroizing::new(secret))),
            Ok(None) => Ok(None),
            Err(()) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn save(&self, secret: &[u8]) -> Result<(), ()> {
        let _ = self.delete();
        self.save_marker().inspect_err(|_| {
            let _ = self.delete();
        })?;
        self.save_secret(secret).inspect_err(|_| {
            let _ = self.delete();
        })
    }

    #[cfg(not(target_os = "macos"))]
    fn save(&self, _secret: &[u8]) -> Result<(), ()> {
        Err(())
    }
}

#[derive(Clone)]
pub(super) struct KeychainStatementPasswordStore {
    service: String,
}

impl KeychainStatementPasswordStore {
    pub(super) fn production() -> Self {
        Self::new(STATEMENT_PASSWORD_KEYCHAIN_SERVICE)
    }

    pub(super) fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    #[cfg(target_os = "macos")]
    fn entry(&self, secret_ref: &str) -> Result<KeyringEntry, ()> {
        KeychainCredential::build(MacKeychainDomain::User, &self.service, secret_ref)
            .map_err(|_| ())
    }

    #[cfg(not(target_os = "macos"))]
    fn entry(&self, _secret_ref: &str) -> Result<KeyringEntry, ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn item_query(&self, secret_ref: &str) -> Result<ItemSearchOptions, ()> {
        let keychain =
            SecKeychain::default_for_domain(SecPreferencesDomain::User).map_err(|_| ())?;
        let mut query = ItemSearchOptions::new();
        query
            .keychains(&[keychain])
            .class(ItemClass::generic_password())
            .service(&self.service)
            .account(secret_ref)
            .limit(1);
        Ok(query)
    }
}

impl StatementPasswordStore for KeychainStatementPasswordStore {
    #[cfg(target_os = "macos")]
    fn delete(&self, secret_ref: &str) -> Result<(), ()> {
        match self.item_query(secret_ref)?.delete() {
            Ok(()) => Ok(()),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(()),
            Err(_) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn delete(&self, _secret_ref: &str) -> Result<(), ()> {
        Err(())
    }

    fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        match self.entry(secret_ref)?.get_secret() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(()),
        }
    }

    fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), ()> {
        self.entry(secret_ref)?.set_secret(secret).map_err(|_| ())
    }
}

#[derive(Clone)]
pub(super) struct KeychainGmailRefreshTokenStore {
    service: String,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "refresh-token save and load are wired by the next connector execution checkpoint"
    )
)]
impl KeychainGmailRefreshTokenStore {
    pub(super) fn production() -> Self {
        Self::new(GMAIL_REFRESH_TOKEN_KEYCHAIN_SERVICE)
    }

    pub(super) fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    #[cfg(target_os = "macos")]
    fn entry(&self, secret_ref: &str) -> Result<KeyringEntry, ()> {
        KeychainCredential::build(MacKeychainDomain::User, &self.service, secret_ref)
            .map_err(|_| ())
    }

    #[cfg(not(target_os = "macos"))]
    fn entry(&self, _secret_ref: &str) -> Result<KeyringEntry, ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn item_query(&self, secret_ref: &str) -> Result<ItemSearchOptions, ()> {
        let keychain =
            SecKeychain::default_for_domain(SecPreferencesDomain::User).map_err(|_| ())?;
        let mut query = ItemSearchOptions::new();
        query
            .keychains(&[keychain])
            .class(ItemClass::generic_password())
            .service(&self.service)
            .account(secret_ref)
            .limit(1);
        Ok(query)
    }
}

impl GmailRefreshTokenStore for KeychainGmailRefreshTokenStore {
    #[cfg(target_os = "macos")]
    fn delete(&self, secret_ref: &str) -> Result<(), ()> {
        match self.item_query(secret_ref)?.delete() {
            Ok(()) => Ok(()),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(()),
            Err(_) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn delete(&self, _secret_ref: &str) -> Result<(), ()> {
        Err(())
    }

    fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        match self.entry(secret_ref)?.get_secret() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(()),
        }
    }

    fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), ()> {
        self.entry(secret_ref)?.set_secret(secret).map_err(|_| ())
    }
}

#[derive(Clone)]
pub(super) struct KeychainLocalInboxBookmarkStore {
    account: String,
    service: String,
}

impl KeychainLocalInboxBookmarkStore {
    pub(super) fn production() -> Self {
        Self {
            account: LOCAL_INBOX_BOOKMARK_ACCOUNT.to_owned(),
            service: LOCAL_INBOX_BOOKMARK_KEYCHAIN_SERVICE.to_owned(),
        }
    }

    #[cfg(target_os = "macos")]
    fn entry(&self) -> Result<KeyringEntry, ()> {
        KeychainCredential::build(MacKeychainDomain::User, &self.service, &self.account)
            .map_err(|_| ())
    }

    #[cfg(not(target_os = "macos"))]
    fn entry(&self) -> Result<KeyringEntry, ()> {
        Err(())
    }
}

impl LocalInboxBookmarkStore for KeychainLocalInboxBookmarkStore {
    fn delete(&self) -> Result<(), ()> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(_) => Err(()),
        }
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        match self.entry()?.get_secret() {
            Ok(bookmark) => Ok(Some(Zeroizing::new(bookmark))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(()),
        }
    }

    fn save(&self, bookmark: &[u8]) -> Result<(), ()> {
        self.entry()?.set_secret(bookmark).map_err(|_| ())
    }
}
