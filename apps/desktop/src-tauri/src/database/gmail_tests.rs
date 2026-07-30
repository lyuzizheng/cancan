use super::*;

const KEY: [u8; KEY_LEN] = [0x47; KEY_LEN];

#[test]
fn upgrades_a_v9_vault_without_changing_existing_data() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let database_path = root.path().join(DATABASE_FILE_NAME);
    let mut connection = open_encrypted_database(
        &database_path,
        &KEY,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .expect("open encrypted database");
    apply_migration_set(&mut connection, &MIGRATIONS[..9]).expect("apply v9 schema");
    connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-existing', 'dbs', 'Existing DBS', 'bank')",
            [],
        )
        .expect("seed existing data");

    apply_migration_set(&mut connection, &MIGRATIONS[9..]).expect("apply Gmail migration");

    let display_name: String = connection
        .query_row(
            "SELECT display_name FROM money_sources WHERE id = 'source-existing'",
            [],
            |row| row.get(0),
        )
        .expect("existing data remains");
    assert_eq!(display_name, "Existing DBS");
    let schema_version: i64 = connection
        .query_row("SELECT max(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("read schema version");
    assert_eq!(schema_version, 10);
}

#[test]
fn keeps_one_stable_gmail_identity_per_mailbox() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store =
        ManualImportStore::open(root.path(), Zeroizing::new(KEY)).expect("open encrypted Vault");

    let first = store
        .begin_gmail_account_save(
            "gmail-account:first",
            "owner@example.com",
            "gmail-refresh-token:first",
        )
        .expect("begin first save");
    assert_eq!(first.status, GmailAccountStatus::PendingSave);
    store
        .mark_gmail_account_connected(&first.id)
        .expect("mark connected");

    let reconnected = store
        .begin_gmail_account_save(
            "gmail-account:different",
            "owner@example.com",
            "gmail-refresh-token:different",
        )
        .expect("begin reconnect");
    assert_eq!(reconnected.id, first.id);
    assert_eq!(reconnected.secret_storage_key, first.secret_storage_key);
    assert_eq!(reconnected.status, GmailAccountStatus::PendingSave);
    let account_count: i64 = store
        .connection
        .query_row("SELECT count(*) FROM gmail_accounts", [], |row| row.get(0))
        .expect("count Gmail accounts");
    assert_eq!(account_count, 1);
    store
        .mark_gmail_account_disconnected(&first.id, "pending_save")
        .expect("recover interrupted save");
    assert_eq!(
        store
            .gmail_account_state("owner@example.com")
            .expect("read retained identity")
            .expect("retained account")
            .status,
        GmailAccountStatus::Disconnected
    );
}
