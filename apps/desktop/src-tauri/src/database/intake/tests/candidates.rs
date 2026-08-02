use super::*;

#[test]
fn candidate_transactions_upsert_park_confirm_and_retry_without_duplicate_sources() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    insert_document(&store, "document-first", '1');
    insert_document(&store, "document-second", '2');

    let first = store
        .attach_money_source_candidate(&MoneySourceCandidateInput {
            candidate_id: "candidate-first",
            document_id: "document-first",
            provider_key: "dbs",
            scope: MoneySourceCandidateScope::ProviderSingleton,
        })
        .expect("attach first document");
    let repeated = store
        .attach_money_source_candidate(&MoneySourceCandidateInput {
            candidate_id: "candidate-ignored",
            document_id: "document-second",
            provider_key: "dbs",
            scope: MoneySourceCandidateScope::ProviderSingleton,
        })
        .expect("reuse candidate for concurrent document");
    assert_eq!(first, repeated);
    assert_eq!(first.candidate_id, "candidate-first");
    assert_eq!(first.version, 1);

    let parked = store
        .keep_money_source_candidate_unassigned("candidate-first", 1)
        .expect("keep candidate unassigned");
    assert_eq!(parked.status, MoneySourceCandidateStatus::KeptUnassigned);
    assert_eq!(parked.version, 2);
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM source_documents \
                 WHERE money_source_candidate_id = 'candidate-first' \
                   AND attention_parked_reason = 'source_confirmation' \
                   AND attention_parked_at IS NOT NULL",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count parked evidence"),
        2
    );

    let confirmation = ConfirmMoneySourceCandidateInput {
        audit_id: "audit-confirm-source",
        candidate_id: "candidate-first",
        display_name: "DBS",
        expected_version: 2,
        proposed_money_source_id: "source-dbs",
        source_type: "bank",
    };
    assert!(
        store
            .confirm_money_source_candidate(&ConfirmMoneySourceCandidateInput {
                expected_version: 1,
                ..confirmation
            })
            .is_err(),
        "stale confirmation must write nothing"
    );
    let confirmed = store
        .confirm_money_source_candidate(&confirmation)
        .expect("confirm source candidate");
    assert_eq!(confirmed.money_source_id, "source-dbs");
    assert_eq!(confirmed.version, 3);
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM source_documents \
                 WHERE money_source_id = 'source-dbs' \
                   AND money_source_candidate_id IS NULL \
                   AND attention_parked_reason IS NULL \
                   AND attention_parked_at IS NULL",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count routed evidence"),
        2
    );

    let retried = store
        .confirm_money_source_candidate(&confirmation)
        .expect("retry confirmation idempotently");
    assert_eq!(retried, confirmed);
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
        store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE action = 'money_source_candidate_confirmed'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count confirmation audit entries"),
        1
    );
}

#[test]
fn candidate_identity_preserves_tagged_exact_provider_scope() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    for (id, hash) in [
        ("document-root-upper", '5'),
        ("document-root-lower", '6'),
        ("document-delimiter-left", '7'),
        ("document-delimiter-right", '8'),
        ("document-singleton", '9'),
        ("document-invalid-root", 'a'),
    ] {
        insert_document(&store, id, hash);
    }

    let identities = [
        (
            "candidate-root-upper",
            "document-root-upper",
            "bank",
            MoneySourceCandidateScope::ProviderRootId("Root:A"),
        ),
        (
            "candidate-root-lower",
            "document-root-lower",
            "bank",
            MoneySourceCandidateScope::ProviderRootId("root:a"),
        ),
        (
            "candidate-delimiter-left",
            "document-delimiter-left",
            "bank:a",
            MoneySourceCandidateScope::ProviderRootId("b"),
        ),
        (
            "candidate-delimiter-right",
            "document-delimiter-right",
            "bank",
            MoneySourceCandidateScope::ProviderRootId("a:b"),
        ),
        (
            "candidate-singleton",
            "document-singleton",
            "bank",
            MoneySourceCandidateScope::ProviderSingleton,
        ),
    ];
    for (candidate_id, document_id, provider_key, scope) in identities {
        let state = store
            .attach_money_source_candidate(&MoneySourceCandidateInput {
                candidate_id,
                document_id,
                provider_key,
                scope,
            })
            .expect("attach exact candidate identity");
        assert_eq!(state.candidate_id, candidate_id);
    }
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM money_source_candidates", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count exact identities"),
        5
    );

    assert!(
        store
            .confirm_money_source_candidate(&ConfirmMoneySourceCandidateInput {
                audit_id: "audit-root-must-fail-closed",
                candidate_id: "candidate-root-upper",
                display_name: "Bank root",
                expected_version: 1,
                proposed_money_source_id: "source-root",
                source_type: "bank",
            })
            .is_err(),
        "provider-root confirmation cannot fall back to provider-only identity"
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM money_sources", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count sources after fail-closed confirmation"),
        0
    );

    let oversized_root = "x".repeat(513);
    assert!(
        store
            .attach_money_source_candidate(&MoneySourceCandidateInput {
                candidate_id: "candidate-invalid-root",
                document_id: "document-invalid-root",
                provider_key: "bank",
                scope: MoneySourceCandidateScope::ProviderRootId(&oversized_root),
            })
            .is_err()
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM money_source_candidates", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count after rejected identity"),
        5
    );
}
