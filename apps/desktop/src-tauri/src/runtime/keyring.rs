use super::*;

#[cfg(target_os = "macos")]
use security_framework::{
    access_control::{ProtectionMode, SecAccessControl},
    passwords::{
        AccessControlOptions, PasswordOptions, delete_generic_password_options, generic_password,
        set_generic_password_options,
    },
};

pub(super) trait RememberedKeyStore: Send + Sync {
    fn delete(&self) -> Result<(), ()>;
    fn is_present(&self) -> Result<bool, ()>;
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()>;
    /// True when the operating system has permanently invalidated the stored
    /// secret (for example, the enrolled Touch ID fingerprint set changed), so
    /// the presence marker and any stale secret should be cleaned up instead of
    /// offering an unlock that can never succeed.
    fn invalidated(&self) -> bool {
        false
    }
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
    fn item_options(&self, account: &str, use_protected: bool) -> PasswordOptions {
        let mut options = PasswordOptions::new_generic_password(&self.service, account);
        if use_protected {
            options.use_protected_keychain();
        }
        options
    }

    #[cfg(target_os = "macos")]
    fn read_item(&self, account: &str) -> Result<Option<Vec<u8>>, security_framework::base::Error> {
        let options = self.item_options(account, true);
        match generic_password(options) {
            Ok(secret) => Ok(Some(secret)),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(None),
            Err(error) => Err(error),
        }
    }

    #[cfg(target_os = "macos")]
    fn delete_item(&self, account: &str, use_protected: bool) -> Result<(), ()> {
        let options = self.item_options(account, use_protected);
        match delete_generic_password_options(options) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(()),
            Err(_) => Err(()),
        }
    }

    #[cfg(target_os = "macos")]
    fn write_item(&self, account: &str, value: &[u8], access_control: usize) -> Result<(), ()> {
        let mut options = self.item_options(account, true);
        options.set_access_synchronized(Some(false));
        let access = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            access_control,
        )
        .map_err(|_| ())?;
        options.set_access_control(access);
        set_generic_password_options(value, options).map_err(|_| ())
    }
}

impl RememberedKeyStore for KeychainRememberedKeyStore {
    #[cfg(target_os = "macos")]
    fn delete(&self) -> Result<(), ()> {
        let protected = [
            self.delete_item(&self.account, true),
            self.delete_item(&self.marker_account, true),
        ];
        // The legacy file-based-keychain item is a best-effort cleanup; an OS
        // error there must not fail a forget that already removed the
        // Touch ID-protected items.
        let _ = self.delete_item(&self.account, false);
        if protected.iter().any(|result| result.is_err()) {
            Err(())
        } else {
            Ok(())
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn delete(&self) -> Result<(), ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn is_present(&self) -> Result<bool, ()> {
        self.read_item(&self.marker_account)
            .map(|secret| secret.is_some())
            .map_err(|_| ())
    }

    #[cfg(not(target_os = "macos"))]
    fn is_present(&self) -> Result<bool, ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn invalidated(&self) -> bool {
        match self.read_item(&self.account) {
            // The read failing because the item is gone means the marker was
            // left behind after cleanup; treat it as invalidated so the marker
            // is removed and the UI stops offering Touch ID.
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => true,
            Err(error) => error.code() == KEYCHAIN_ITEM_INVALIDATED_STATUS,
            Ok(_) => false,
        }
    }

    #[cfg(target_os = "macos")]
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        self.read_item(&self.account)
            .map(|secret| secret.map(Zeroizing::new))
            .map_err(|_| ())
    }

    #[cfg(not(target_os = "macos"))]
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn save(&self, secret: &[u8]) -> Result<(), ()> {
        const MARKER_VALUE: &[u8] = &[1];

        let _ = self.delete();
        self.write_item(
            &self.marker_account,
            MARKER_VALUE,
            AccessControlOptions::empty().bits(),
        )?;
        self.write_item(&self.account, secret, self.access_control)
            .inspect_err(|_| {
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
