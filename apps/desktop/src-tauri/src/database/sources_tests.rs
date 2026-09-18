use super::*;

const KEY: [u8; KEY_LEN] = [0x53; KEY_LEN];

fn open_store(root: &Path) -> ManualImportStore {
    ManualImportStore::open(root, Zeroizing::new(KEY)).expect("open Vault")
}

/// One creation input. The audit id is explicit because the caller owns
/// generating it — the runtime uses a random identifier per call — and a reused
/// id would collide on `audit_log.id`.
fn create_input<'a>(
    audit_id: &'a str,
    money_source_id: &'a str,
    provider_key: &'a str,
    display_name: &'a str,
) -> CreateMoneySourceInput<'a> {
    CreateMoneySourceInput {
        audit_id,
        display_name,
        money_source_id,
        provider_key,
        source_type: "bank",
    }
}

fn audit_actions(store: &ManualImportStore, entity_id: &str) -> Vec<String> {
    let mut statement = store
        .connection
        .prepare("SELECT action FROM audit_log WHERE entity_id = ?1 ORDER BY created_at, id")
        .expect("prepare audit read");
    statement
        .query_map([entity_id], |row| row.get::<_, String>(0))
        .expect("read audit entries")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect audit entries")
}

#[test]
fn creates_one_provider_singleton_and_audits_it() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());

    let created = store
        .create_money_source(&create_input(
            "audit-source-dbs",
            "source-dbs",
            "dbs",
            "DBS",
        ))
        .expect("create Money Source");
    assert_eq!(
        created,
        MoneySourceView {
            display_name: "DBS".to_owned(),
            money_source_id: "source-dbs".to_owned(),
            provider_key: "dbs".to_owned(),
            source_type: "bank".to_owned(),
        }
    );
    assert_eq!(
        store
            .money_source("source-dbs")
            .expect("read Money Source")
            .as_ref(),
        Some(&created)
    );
    let provider_root: Option<String> = store
        .connection
        .query_row(
            "SELECT provider_root_id FROM money_sources WHERE id = 'source-dbs'",
            [],
            |row| row.get(0),
        )
        .expect("read provider root");
    assert_eq!(
        provider_root, None,
        "a user-created source is a provider singleton, never a claimed provider root"
    );
    assert_eq!(
        store
            .list_money_sources()
            .expect("list Money Sources")
            .into_iter()
            .map(|source| source.money_source_id)
            .collect::<Vec<_>>(),
        vec!["source-dbs".to_owned()]
    );
    assert_eq!(
        audit_actions(&store, "source-dbs"),
        vec!["money_source_created".to_owned()]
    );
}

#[test]
fn refuses_a_second_singleton_for_the_same_provider() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    store
        .create_money_source(&create_input(
            "audit-source-dbs",
            "source-dbs",
            "dbs",
            "DBS",
        ))
        .expect("create first Money Source");

    let error = store
        .create_money_source(&create_input(
            "audit-source-dbs-second",
            "source-dbs-second",
            "dbs",
            "DBS Joint",
        ))
        .expect_err("a second provider singleton would make routing ambiguous");
    assert_eq!(
        error.downcast_ref::<io::Error>().map(io::Error::kind),
        Some(io::ErrorKind::AlreadyExists)
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM money_sources", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count Money Sources"),
        1
    );
    assert_eq!(
        audit_actions(&store, "source-dbs-second"),
        Vec::<String>::new(),
        "a refused creation writes no audit entry"
    );
}

#[test]
fn keeps_one_singleton_per_provider() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    store
        .create_money_source(&create_input(
            "audit-source-dbs",
            "source-dbs",
            "dbs",
            "DBS",
        ))
        .expect("create DBS");
    store
        .create_money_source(&create_input(
            "audit-source-hsbc",
            "source-hsbc",
            "hsbc",
            "HSBC",
        ))
        .expect("create HSBC");

    assert_eq!(
        store
            .list_money_sources()
            .expect("list Money Sources")
            .into_iter()
            .map(|source| source.money_source_id)
            .collect::<Vec<_>>(),
        vec!["source-dbs".to_owned(), "source-hsbc".to_owned()]
    );
}

#[test]
fn renames_only_the_display_name_and_audits_it() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    store
        .create_money_source(&create_input(
            "audit-source-dbs",
            "source-dbs",
            "dbs",
            "DBS",
        ))
        .expect("create Money Source");

    let renamed = store
        .rename_money_source("source-dbs", "DBS Joint", "audit-rename-source")
        .expect("rename Money Source");
    assert_eq!(
        renamed,
        MoneySourceView {
            display_name: "DBS Joint".to_owned(),
            money_source_id: "source-dbs".to_owned(),
            provider_key: "dbs".to_owned(),
            source_type: "bank".to_owned(),
        }
    );
    let identity: (String, String) = store
        .connection
        .query_row(
            "SELECT provider_key, COALESCE(provider_root_id, '') FROM money_sources WHERE id = ?1",
            ["source-dbs"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read identity");
    assert_eq!(identity, ("dbs".to_owned(), String::new()));
    let rename_reference: String = store
        .connection
        .query_row(
            "SELECT source_ref FROM audit_log \
             WHERE entity_id = 'source-dbs' AND action = 'money_source_renamed'",
            [],
            |row| row.get(0),
        )
        .expect("read rename source reference");
    assert_eq!(
        rename_reference, "DBS -> DBS Joint",
        "the previous name stays recoverable in the evidence trail"
    );
    // Both entries land in the same second, so compare as a set: the audit log
    // orders equal timestamps by id and the test must not pin that tie-break.
    let mut actions = audit_actions(&store, "source-dbs");
    actions.sort();
    assert_eq!(
        actions,
        vec![
            "money_source_created".to_owned(),
            "money_source_renamed".to_owned(),
        ]
    );
}

#[test]
fn refuses_to_rename_an_unknown_source() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());

    let error = store
        .rename_money_source("source-missing", "DBS", "audit-rename-source")
        .expect_err("an unknown Money Source cannot be renamed");
    assert_eq!(
        error.downcast_ref::<io::Error>().map(io::Error::kind),
        Some(io::ErrorKind::NotFound)
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM audit_log", [], |row| row
                .get::<_, i64>(0))
            .expect("count audit entries"),
        0
    );
}

#[test]
fn rejects_malformed_creation_and_rename_input() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());

    for input in [
        create_input("audit-source-dbs", "source-dbs", "dbs", ""),
        create_input("audit-source-dbs", "source-dbs", "", "DBS"),
        create_input("audit-malformed", "", "dbs", "DBS"),
    ] {
        assert!(
            store.create_money_source(&input).is_err(),
            "malformed creation input must be refused"
        );
    }
    store
        .create_money_source(&create_input(
            "audit-source-dbs",
            "source-dbs",
            "dbs",
            "DBS",
        ))
        .expect("create Money Source");
    assert!(
        store
            .rename_money_source("source-dbs", "", "audit-rename-source")
            .is_err(),
        "a blank display name must be refused"
    );
    assert_eq!(
        store
            .money_source("source-dbs")
            .expect("read Money Source")
            .expect("Money Source exists")
            .display_name,
        "DBS"
    );
}

#[test]
fn lists_only_provider_singletons_with_their_source_id() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    assert!(
        store
            .provider_singletons()
            .expect("read singletons")
            .is_empty(),
        "an unconfigured Vault configures no provider"
    );
    store
        .create_money_source(&create_input(
            "audit-source-dbs",
            "source-dbs",
            "dbs",
            "DBS",
        ))
        .expect("create DBS");
    // A source claimed for a provider root is not the provider singleton: the
    // create gate would still accept one for `dbs`, so the picker must not
    // report the provider as configured.
    store
        .connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, provider_root_id, display_name, source_type) \
             VALUES ('source-dbs-root', 'dbs', 'root-1', 'DBS Joint Account', 'bank')",
            [],
        )
        .expect("seed root-scoped source");

    assert_eq!(
        store.provider_singletons().expect("read singletons"),
        HashMap::from([("dbs".to_owned(), "source-dbs".to_owned())])
    );
}

#[test]
fn reports_the_saved_statement_password_state_per_source() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    store
        .create_money_source(&create_input(
            "audit-source-dbs",
            "source-dbs",
            "dbs",
            "DBS",
        ))
        .expect("create Money Source");
    store
        .create_money_source(&create_input(
            "audit-source-hsbc",
            "source-hsbc",
            "hsbc",
            "HSBC",
        ))
        .expect("create second Money Source");

    assert!(
        !store
            .has_saved_statement_password("source-dbs")
            .expect("read password state")
    );
    store
        .connection
        .execute(
            "INSERT INTO statement_secret_refs(id, money_source_id, secret_storage_key, status) \
             VALUES ('secret-dbs', 'source-dbs', 'money-source:source-dbs', 'pending_save')",
            [],
        )
        .expect("store pending password reference");
    assert!(
        !store
            .has_saved_statement_password("source-dbs")
            .expect("read pending password state"),
        "a pending save is not a stored password"
    );
    store
        .connection
        .execute(
            "UPDATE statement_secret_refs SET status = 'saved' WHERE id = 'secret-dbs'",
            [],
        )
        .expect("confirm stored password");
    assert!(
        store
            .has_saved_statement_password("source-dbs")
            .expect("read saved password state")
    );
    assert!(
        !store
            .has_saved_statement_password("source-hsbc")
            .expect("read other source password state")
    );
}
