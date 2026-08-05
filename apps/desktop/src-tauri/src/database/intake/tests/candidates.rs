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

    let root_confirmation = ConfirmMoneySourceCandidateInput {
        audit_id: "audit-root-confirm",
        candidate_id: "candidate-root-upper",
        display_name: "Bank root",
        expected_version: 1,
        proposed_money_source_id: "source-root",
        source_type: "bank",
    };
    let confirmed_root = store
        .confirm_money_source_candidate(&root_confirmation)
        .expect("root-scoped confirmation persists exact root identity");
    assert_eq!(confirmed_root.money_source_id, "source-root");
    let stored_root = store
        .connection
        .query_row(
            "SELECT provider_root_id FROM money_sources WHERE id = 'source-root'",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("read persisted root identity")
        .expect("root identity persisted");
    assert_eq!(stored_root, "Root:A", "root identity is exact, not folded");
    assert_eq!(
        store
            .confirm_money_source_candidate(&root_confirmation)
            .expect("root confirmation retry is idempotent"),
        confirmed_root
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM money_sources", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count sources after root confirmation"),
        1
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

#[test]
fn root_confirmation_never_reuses_singleton_source_and_keeps_roots_distinct() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    for (id, hash) in [
        ("document-singleton", 'b'),
        ("document-root-a", 'c'),
        ("document-root-b", 'd'),
    ] {
        insert_document(&store, id, hash);
    }
    for (candidate_id, document_id, scope) in [
        (
            "candidate-singleton",
            "document-singleton",
            MoneySourceCandidateScope::ProviderSingleton,
        ),
        (
            "candidate-root-a",
            "document-root-a",
            MoneySourceCandidateScope::ProviderRootId("Root:A"),
        ),
        (
            "candidate-root-b",
            "document-root-b",
            MoneySourceCandidateScope::ProviderRootId("Root:B"),
        ),
    ] {
        store
            .attach_money_source_candidate(&MoneySourceCandidateInput {
                candidate_id,
                document_id,
                provider_key: "bank",
                scope,
            })
            .expect("attach candidate");
    }

    // 1. Confirm the singleton first: creates a source with NULL provider_root_id.
    let confirmed_singleton = store
        .confirm_money_source_candidate(&ConfirmMoneySourceCandidateInput {
            audit_id: "audit-singleton",
            candidate_id: "candidate-singleton",
            display_name: "Bank",
            expected_version: 1,
            proposed_money_source_id: "source-singleton",
            source_type: "bank",
        })
        .expect("confirm singleton candidate");
    assert_eq!(confirmed_singleton.money_source_id, "source-singleton");

    // 2. Root-scoped confirmation under the same provider_key must create a NEW
    //    source, never reuse the singleton (provider_root_id IS NULL cannot match).
    let confirmed_root_a = store
        .confirm_money_source_candidate(&ConfirmMoneySourceCandidateInput {
            audit_id: "audit-root-a",
            candidate_id: "candidate-root-a",
            display_name: "Bank Root A",
            expected_version: 1,
            proposed_money_source_id: "source-root-a",
            source_type: "bank",
        })
        .expect("confirm root A");
    assert_eq!(
        confirmed_root_a.money_source_id, "source-root-a",
        "root confirmation must not reuse the singleton source"
    );

    // 3. A second distinct root under the same provider must create another source.
    let confirmed_root_b = store
        .confirm_money_source_candidate(&ConfirmMoneySourceCandidateInput {
            audit_id: "audit-root-b",
            candidate_id: "candidate-root-b",
            display_name: "Bank Root B",
            expected_version: 1,
            proposed_money_source_id: "source-root-b",
            source_type: "bank",
        })
        .expect("confirm root B");
    assert_eq!(confirmed_root_b.money_source_id, "source-root-b");

    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM money_sources", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count sources"),
        3
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM money_sources \
                 WHERE provider_key = 'bank' AND provider_root_id IS NOT NULL",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count root-scoped sources"),
        2
    );

    // 4. The partial unique index rejects a duplicate (provider_key, provider_root_id)
    //    pair, even through a raw insert — so a second source for the same root
    //    identity can never be created, by any path.
    assert!(
        store
            .connection
            .execute(
                "INSERT INTO money_sources(id, provider_key, provider_root_id, display_name, source_type) \
                 VALUES ('source-duplicate', 'bank', 'Root:A', 'Dup', 'bank')",
                [],
            )
            .is_err(),
        "provider_root_id uniqueness is enforced per provider"
    );
    // 5. Candidate identity uniqueness (composite index on money_source_candidates)
    //    also blocks a twin candidate for the same root identity — confirmation
    //    reuse is therefore only reachable for an already-confirmed candidate,
    //    which the idempotent early-return handles.
    assert!(
        store
            .connection
            .execute(
                "INSERT INTO money_source_candidates( \
                   id, candidate_key_version, provider_key, candidate_scope_kind, \
                   candidate_scope_value, status \
                 ) VALUES ( \
                   'candidate-root-a-twin', 1, 'bank', 'provider_root_id', 'Root:A', 'pending' \
                 )",
                [],
            )
            .is_err(),
        "a twin candidate for the same root identity is blocked by identity uniqueness"
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM money_sources", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count sources after uniqueness checks"),
        3
    );
}

#[test]
fn singleton_confirmation_does_not_reuse_root_scoped_source() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    for (id, hash) in [("document-singleton", 'b'), ("document-root", 'c')] {
        insert_document(&store, id, hash);
    }

    // Seed a root-scoped source directly (as if a prior root confirmation created it).
    store
        .connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, provider_root_id, display_name, source_type) VALUES ('source-root', 'bank', 'Root:A', 'Bank Root', 'bank')",
            [],
        )
        .expect("seed root source");

    // A singleton candidate for the same provider must NOT reuse the root-scoped source.
    store
        .attach_money_source_candidate(&MoneySourceCandidateInput {
            candidate_id: "candidate-singleton",
            document_id: "document-singleton",
            provider_key: "bank",
            scope: MoneySourceCandidateScope::ProviderSingleton,
        })
        .expect("attach singleton candidate");

    let confirmed = store
        .confirm_money_source_candidate(&ConfirmMoneySourceCandidateInput {
            audit_id: "audit-singleton",
            candidate_id: "candidate-singleton",
            display_name: "Bank",
            expected_version: 1,
            proposed_money_source_id: "source-singleton",
            source_type: "bank",
        })
        .expect("confirm singleton candidate");

    assert_eq!(confirmed.money_source_id, "source-singleton");
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM money_sources", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count sources"),
        2,
        "singleton confirmation must create a new source, not reuse the root-scoped one"
    );
}
