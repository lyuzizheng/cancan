use super::*;

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires a code-signed macOS app with keychain-access-groups entitlement and interactive authentication is not available in unit tests"]
fn keychain_store_round_trips_binary_secret() {
    struct Cleanup(KeychainRememberedKeyStore);

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete();
        }
    }

    let service = format!("{KEYCHAIN_SERVICE}.test.{}", candidate_name());
    let store =
        KeychainRememberedKeyStore::new_without_biometry(service.clone(), KEYCHAIN_ACCOUNT);
    store.delete().expect("remove pre-existing test entry");
    let _cleanup = Cleanup(store.clone());
    assert!(!store.is_present().expect("test entry starts absent"));

    let secret = [0_u8, 1, 2, 0, 4, 5, 6, 7];
    store.save(&secret).expect("save binary Keychain secret");
    assert!(store.is_present().expect("test entry is present"));

    let restarted =
        KeychainRememberedKeyStore::new_without_biometry(service, KEYCHAIN_ACCOUNT);
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
