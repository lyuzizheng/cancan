use super::tests::{MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore};
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
