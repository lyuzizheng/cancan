use super::*;

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
    account: String,
    service: String,
}

impl KeychainRememberedKeyStore {
    pub(super) fn production() -> Self {
        Self::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
    }

    pub(super) fn new(service: impl Into<String>, account: impl Into<String>) -> Self {
        Self {
            account: account.into(),
            service: service.into(),
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

    #[cfg(target_os = "macos")]
    fn item_query(&self) -> Result<ItemSearchOptions, ()> {
        let keychain =
            SecKeychain::default_for_domain(SecPreferencesDomain::User).map_err(|_| ())?;
        let mut query = ItemSearchOptions::new();
        query
            .keychains(&[keychain])
            .class(ItemClass::generic_password())
            .service(&self.service)
            .account(&self.account)
            .limit(1);
        Ok(query)
    }
}

impl RememberedKeyStore for KeychainRememberedKeyStore {
    #[cfg(target_os = "macos")]
    fn delete(&self) -> Result<(), ()> {
        match self.item_query()?.delete() {
            Ok(()) => Ok(()),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(()),
            Err(_) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn delete(&self) -> Result<(), ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn is_present(&self) -> Result<bool, ()> {
        match self.item_query()?.search() {
            Ok(_) => Ok(true),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(false),
            Err(_) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn is_present(&self) -> Result<bool, ()> {
        Err(())
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        match self.entry()?.get_secret() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(()),
        }
    }

    fn save(&self, secret: &[u8]) -> Result<(), ()> {
        self.entry()?.set_secret(secret).map_err(|_| ())
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
