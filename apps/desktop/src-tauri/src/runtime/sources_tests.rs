use super::test_support::{
    MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore, MemoryStatementPasswordStore,
};
use super::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::{fs, path::Path};

/// The one statement body every case captures.
const STATEMENT_CSV: &[u8] = b"date,amount\n2026-07-01,10.00\n";

fn unlocked_runtime(root: &Path) -> VaultRuntime {
    let runtime = VaultRuntime::with_secret_stores(
        root.to_path_buf(),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
}

/// Captures one statement and routes it to the given provider through the
/// trusted classification path, so the source owns a real document and account.
fn route_document(runtime: &VaultRuntime, path: &Path, provider_key: &str) -> String {
    let imported = runtime
        .import_selected_document(path)
        .expect("capture document");
    let mut store = runtime.store().expect("open store");
    let store = store.as_mut().expect("unlocked store");
    store
        .apply_trusted_classification(&TrustedDocumentClassification {
            accounts: &[TrustedAccountCandidate {
                account_id: "account-dbs-checking",
                account_type: "checking",
                currency: Some("SGD"),
                display_name: "DBS Checking",
                masked_identifier: Some("••1234"),
                provider_account_id: Some("provider-account-1234"),
            }],
            audit_id: "audit-test-classification",
            document_id: &imported.document_id,
            document_type: Some("statement"),
            provider_key,
            provider_root_id: None,
            semantic_document_key: "dbs:test-statement-1",
            statement_period_from: Some("2026-07-01"),
            statement_period_to: Some("2026-07-31"),
        })
        .expect("route document");
    imported.document_id
}

fn write_statement(parent: &Path) -> std::path::PathBuf {
    let statement = parent.join("statement.csv");
    fs::write(&statement, STATEMENT_CSV).expect("write fixture");
    statement
}

/// The renderer-facing detail payload: the contract this command exists for.
fn detail_payload(runtime: &VaultRuntime, money_source_id: &str) -> serde_json::Value {
    serde_json::to_value(
        runtime
            .money_source_detail(money_source_id)
            .expect("read source detail"),
    )
    .expect("serialize source detail")
}

#[test]
fn creates_a_money_source_from_the_supported_provider_catalog() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = unlocked_runtime(&parent.path().join("vault"));

    let created = runtime
        .create_money_source("dbs", None)
        .expect("create Money Source");
    assert_eq!(created.display_name, "DBS");
    assert_eq!(created.provider_key, "dbs");
    assert_eq!(created.source_type, "bank");
    assert!(!created.money_source_id.is_empty());
    assert_eq!(
        runtime
            .list_money_sources()
            .expect("list Money Sources")
            .into_iter()
            .map(|source| source.money_source_id)
            .collect::<Vec<_>>(),
        vec![created.money_source_id.clone()]
    );

    let named = runtime
        .create_money_source("hsbc", Some("  HSBC Premier  "))
        .expect("create named Money Source");
    assert_eq!(named.display_name, "HSBC Premier");
    assert_eq!(named.provider_key, "hsbc");
    assert_eq!(named.source_type, "bank");
}

#[test]
fn refuses_unsupported_providers_duplicates_and_blank_names() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = unlocked_runtime(&parent.path().join("vault"));
    runtime
        .create_money_source("dbs", None)
        .expect("create Money Source");

    assert_eq!(
        runtime
            .create_money_source("uob", None)
            .expect_err("a provider outside the catalog must be refused")
            .code(),
        "unsupported_source_provider"
    );
    assert_eq!(
        runtime
            .create_money_source("dbs", Some("DBS Joint"))
            .expect_err("a provider keeps one configured source")
            .code(),
        "source_provider_already_configured"
    );
    assert_eq!(
        runtime
            .create_money_source("hsbc", Some("   "))
            .expect_err("a blank display name must be refused")
            .code(),
        "invalid_source_request"
    );
    assert_eq!(
        runtime
            .create_money_source("hsbc", Some(&"N".repeat(257)))
            .expect_err("an unbounded display name must be refused")
            .code(),
        "invalid_source_request"
    );
    assert_eq!(
        runtime
            .list_money_sources()
            .expect("list Money Sources")
            .len(),
        1
    );
}

#[test]
fn renames_a_money_source_without_touching_its_evidence() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = unlocked_runtime(&parent.path().join("vault"));
    let created = runtime
        .create_money_source("dbs", None)
        .expect("create Money Source");
    let document_id = route_document(&runtime, &write_statement(parent.path()), "dbs");
    assert_eq!(
        runtime
            .list_source_documents(&created.money_source_id)
            .expect("list source documents")
            .len(),
        1
    );

    let renamed = runtime
        .edit_money_source(&created.money_source_id, "  DBS Joint  ")
        .expect("rename Money Source");
    assert_eq!(renamed.display_name, "DBS Joint");
    assert_eq!(renamed.money_source_id, created.money_source_id);
    assert_eq!(renamed.source_type, created.source_type);
    assert_eq!(
        runtime
            .list_source_documents(&created.money_source_id)
            .expect("documents stay attached")
            .into_iter()
            .map(|document| document.document_id)
            .collect::<Vec<_>>(),
        vec![document_id]
    );

    assert_eq!(
        runtime
            .edit_money_source("source-missing", "DBS")
            .expect_err("an unknown Money Source cannot be renamed")
            .code(),
        "source_not_found"
    );
    assert_eq!(
        runtime
            .edit_money_source(&created.money_source_id, "  ")
            .expect_err("a blank display name must be refused")
            .code(),
        "invalid_source_request"
    );
    assert_eq!(
        runtime
            .edit_money_source("", "DBS")
            .expect_err("an empty Money Source id must be refused")
            .code(),
        "invalid_source_request"
    );
}

#[test]
fn source_detail_projects_metadata_documents_and_actions() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = unlocked_runtime(&parent.path().join("vault"));
    let created = runtime
        .create_money_source("dbs", None)
        .expect("create Money Source");
    assert_eq!(
        detail_payload(&runtime, &created.money_source_id),
        serde_json::json!({
            "actions": {
                "canEnterStatementPassword": false,
                "hasSavedStatementPassword": false,
            },
            "displayName": "DBS",
            "documents": [],
            "moneySourceId": created.money_source_id,
            "providerKey": "dbs",
            "sourceType": "bank",
        })
    );

    let document_id = route_document(&runtime, &write_statement(parent.path()), "dbs");
    let job = runtime
        .queued_local_inbox_parse_documents()
        .expect("read parse job")
        .pop()
        .expect("queued parse job");
    let attempt = runtime
        .start_local_inbox_parse(&job)
        .expect("claim parse job")
        .expect("parse job claimed");
    runtime
        .block_local_inbox_parse(&attempt, "password_required")
        .expect("block parse pending password");

    let detail = detail_payload(&runtime, &created.money_source_id);
    let mut document = detail["documents"][0]
        .as_object()
        .expect("document object")
        .clone();
    assert!(
        document["receivedAt"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "a captured document carries its receipt time"
    );
    document.remove("receivedAt");
    assert_eq!(
        serde_json::Value::Object(document),
        serde_json::json!({
            "attentionReason": "password_required",
            "byteSize": STATEMENT_CSV.len(),
            "documentId": document_id,
            "documentStatus": "needs_attention",
            "fileState": "available",
            "mimeType": "text/csv",
            "originalFilename": "statement.csv",
        }),
        "the detail describes the waiting document the same way the document list does"
    );
    assert_eq!(detail["displayName"], "DBS");
    assert_eq!(detail["providerKey"], "dbs");
    assert_eq!(detail["sourceType"], "bank");
    assert_eq!(detail["moneySourceId"], created.money_source_id);
    assert_eq!(
        detail["actions"],
        serde_json::json!({
            "canEnterStatementPassword": true,
            "hasSavedStatementPassword": false,
        })
    );
    assert_eq!(
        runtime
            .money_source_detail("source-missing")
            .expect_err("an unknown Money Source has no detail")
            .code(),
        "source_not_found"
    );
    assert_eq!(
        runtime
            .money_source_detail("")
            .expect_err("an empty Money Source id must be refused")
            .code(),
        "invalid_source_request"
    );
}

/// The picker's one source of truth: the host catalog, with the provider
/// singleton already configured for each entry.
#[test]
fn lists_the_supported_provider_catalog_with_its_configured_source() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = unlocked_runtime(&parent.path().join("vault"));
    let catalog = serde_json::to_value(
        runtime
            .list_supported_money_source_providers()
            .expect("read provider catalog"),
    )
    .expect("serialize provider catalog");
    assert_eq!(
        catalog,
        serde_json::json!([
            {
                "configuredMoneySourceId": null,
                "displayName": "DBS",
                "providerKey": "dbs",
                "sourceType": "bank",
            },
            {
                "configuredMoneySourceId": null,
                "displayName": "HSBC",
                "providerKey": "hsbc",
                "sourceType": "bank",
            },
        ])
    );

    let created = runtime
        .create_money_source("hsbc", Some("HSBC Premier"))
        .expect("create Money Source");
    let catalog = serde_json::to_value(
        runtime
            .list_supported_money_source_providers()
            .expect("read configured provider catalog"),
    )
    .expect("serialize configured provider catalog");
    assert_eq!(
        catalog
            .as_array()
            .expect("catalog array")
            .iter()
            .map(|provider| (
                provider["providerKey"].as_str().expect("provider key"),
                provider["configuredMoneySourceId"].clone(),
            ))
            .collect::<Vec<_>>(),
        vec![
            ("dbs", serde_json::Value::Null),
            ("hsbc", serde_json::json!(created.money_source_id)),
        ],
        "a configured provider reports the source the picker can open, not a flag"
    );
    assert_eq!(
        catalog
            .as_array()
            .expect("catalog array")
            .iter()
            .find(|provider| provider["providerKey"] == "hsbc")
            .expect("HSBC entry")["displayName"],
        "HSBC",
        "the catalog names the provider, not the user's label for the source"
    );
}

/// The source-detail contract is the renderer's window onto evidence: it
/// carries display metadata and user-facing status only, never the stored hash,
/// locator, or secret material behind a document.
#[test]
fn source_detail_never_exposes_hashes_locators_or_secrets() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = unlocked_runtime(&parent.path().join("vault"));
    let created = runtime
        .create_money_source("dbs", None)
        .expect("create Money Source");
    route_document(&runtime, &write_statement(parent.path()), "dbs");
    let stored_sha = format!("{:x}", Sha256::digest(STATEMENT_CSV));
    let stored_locator = {
        let store = runtime.store().expect("open store");
        let store = store.as_ref().expect("unlocked store");
        store
            .list_documents(&created.money_source_id)
            .expect("list documents")
            .into_iter()
            .map(|document| document.encrypted_locator.expect("stored locator"))
            .collect::<Vec<_>>()
    };
    assert_eq!(stored_locator.len(), 1);

    let payload = detail_payload(&runtime, &created.money_source_id);
    let serialized = payload.to_string();
    assert!(
        !serialized.contains(&stored_sha),
        "the stored hash must not reach the renderer"
    );
    assert!(
        !serialized.contains(&stored_locator[0]),
        "the encrypted locator must not reach the renderer"
    );
    let mut keys = payload["documents"][0]
        .as_object()
        .expect("document object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "attentionReason",
            "byteSize",
            "documentId",
            "documentStatus",
            "fileState",
            "mimeType",
            "originalFilename",
            "receivedAt",
        ],
        "the document projection is fixed: a new field has to revisit this contract"
    );
}
