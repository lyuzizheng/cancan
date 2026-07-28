use super::*;

const KEY: [u8; KEY_LEN] = [0x91; KEY_LEN];

fn open_store(root: &Path) -> ManualImportStore {
    let store = ManualImportStore::open(root, Zeroizing::new(KEY)).expect("open encrypted Vault");
    store
        .connection
        .execute(
            "INSERT OR IGNORE INTO money_sources( \
               id, provider_key, display_name, source_type \
             ) VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
            [],
        )
        .expect("seed money source");
    store
}

fn seed_candidate_account(
    store: &ManualImportStore,
    account_id: &str,
    money_source_id: &str,
    display_name: &str,
    account_type: &str,
    masked_identifier: Option<&str>,
    currency: Option<&str>,
) {
    store
        .connection
        .execute(
            "INSERT INTO accounts( \
               id, money_source_id, provider_key, provider_account_id, account_type, \
               display_name, masked_identifier, currency, status, raw_identity_json \
             ) VALUES (?1, ?2, 'synthetic', ?3, ?4, ?5, ?6, ?7, 'candidate', ?8)",
            params![
                account_id,
                money_source_id,
                format!("private-{account_id}"),
                account_type,
                display_name,
                masked_identifier,
                currency,
                format!(r#"{{"providerAccountId":"private-{account_id}"}}"#),
            ],
        )
        .expect("seed candidate account");
}

#[test]
fn lists_pending_account_confirmations_without_private_identity_fields() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store = open_store(root.path());
    store
        .seed_money_source("source-alpha", "alpha", "Alpha Bank", "bank")
        .expect("seed source");
    seed_candidate_account(
        &store,
        "candidate-dbs",
        "source-dbs",
        "Everyday",
        "deposit_account",
        Some("••001"),
        Some("SGD"),
    );
    seed_candidate_account(
        &store,
        "candidate-alpha",
        "source-alpha",
        "Savings",
        "deposit_account",
        None,
        Some("USD"),
    );
    seed_candidate_account(
        &store,
        "confirmed-dbs",
        "source-dbs",
        "Confirmed",
        "deposit_account",
        None,
        Some("SGD"),
    );
    store
        .connection
        .execute(
            "UPDATE accounts SET status = 'confirmed' WHERE id = 'confirmed-dbs'",
            [],
        )
        .expect("confirm fixture account");

    let prompts = store
        .list_account_confirmation_prompts()
        .expect("list pending confirmations");

    assert_eq!(prompts.len(), 2);
    assert_eq!(
        prompts[0].candidate_accounts,
        vec![AccountConfirmationCandidate {
            account_id: "candidate-alpha".to_owned(),
            account_type: "deposit_account".to_owned(),
            currency: Some("USD".to_owned()),
            display_name: "Savings".to_owned(),
            masked_identifier: None,
        }]
    );
    assert_eq!(prompts[0].dismissed_accounts, vec![]);
    assert_eq!(prompts[0].display_name, "Alpha Bank");
    assert_eq!(prompts[0].money_source_id, "source-alpha");
    assert_eq!(
        prompts[1].candidate_accounts,
        vec![AccountConfirmationCandidate {
            account_id: "candidate-dbs".to_owned(),
            account_type: "deposit_account".to_owned(),
            currency: Some("SGD".to_owned()),
            display_name: "Everyday".to_owned(),
            masked_identifier: Some("••001".to_owned()),
        }]
    );
    assert_eq!(prompts[1].dismissed_accounts, vec![]);
    assert_eq!(prompts[1].display_name, "DBS");
    assert_eq!(prompts[1].money_source_id, "source-dbs");
    assert!(
        prompts
            .iter()
            .all(|prompt| prompt.proposal_version.len() == 64)
    );
    assert_eq!(
        prompts
            .iter()
            .map(|prompt| prompt.proposal_version.clone())
            .collect::<Vec<_>>(),
        store
            .list_account_confirmation_prompts()
            .expect("relist deterministic proposals")
            .into_iter()
            .map(|prompt| prompt.proposal_version)
            .collect::<Vec<_>>(),
    );
    let serialized = serde_json::to_string(&prompts).expect("serialize safe prompts");
    assert!(!serialized.contains("providerAccountId"));
    assert!(!serialized.contains("providerKey"));
    assert!(!serialized.contains("rawIdentityJson"));
    assert!(!serialized.contains("private-candidate-dbs"));
}

#[test]
fn confirms_the_exact_candidate_set_once() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_candidate_account(
        &store,
        "candidate-one",
        "source-dbs",
        "Everyday",
        "deposit_account",
        Some("••001"),
        Some("SGD"),
    );
    seed_candidate_account(
        &store,
        "candidate-two",
        "source-dbs",
        "Savings",
        "deposit_account",
        Some("••002"),
        Some("SGD"),
    );
    let prompt = store
        .list_account_confirmation_prompts()
        .expect("list candidate proposal")
        .pop()
        .expect("candidate prompt");
    let expected = ["candidate-two".to_owned(), "candidate-one".to_owned()];

    assert_eq!(
        store
            .decide_candidate_accounts(
                "source-dbs",
                &prompt.proposal_version,
                &expected
                    .iter()
                    .cloned()
                    .map(|account_id| CandidateAccountDecisionInput {
                        account_id,
                        action: CandidateAccountDecision::Accept,
                    })
                    .collect::<Vec<_>>(),
                "audit-confirm-accounts",
            )
            .expect("confirm exact candidate set"),
        AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Confirmed,
        }
    );
    let statuses = store
        .connection
        .prepare("SELECT status FROM accounts WHERE id IN (?1, ?2) ORDER BY id")
        .expect("prepare account status query")
        .query_map(["candidate-one", "candidate-two"], |row| {
            row.get::<_, String>(0)
        })
        .expect("query account statuses")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect account statuses");
    assert_eq!(statuses, vec!["confirmed", "confirmed"]);
    let audit_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log \
             WHERE entity_id IN ('candidate-one', 'candidate-two') \
               AND action = 'candidate_account_accepted'",
            [],
            |row| row.get(0),
        )
        .expect("count confirmation audits");
    assert_eq!(audit_count, 2);
    let source_refs = store
        .connection
        .prepare(
            "SELECT source_ref FROM audit_log WHERE entity_id IN ('candidate-one', 'candidate-two') \
             ORDER BY entity_id",
        )
        .expect("prepare confirmation audit source query")
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .expect("query confirmation audit sources")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect confirmation audit sources");
    assert_eq!(
        source_refs,
        vec![
            Some(format!("source-dbs:proposal:{}", prompt.proposal_version)),
            Some(format!("source-dbs:proposal:{}", prompt.proposal_version)),
        ]
    );

    assert_eq!(
        store
            .decide_candidate_accounts(
                "source-dbs",
                &prompt.proposal_version,
                &expected
                    .iter()
                    .cloned()
                    .map(|account_id| CandidateAccountDecisionInput {
                        account_id,
                        action: CandidateAccountDecision::Accept,
                    })
                    .collect::<Vec<_>>(),
                "audit-repeat",
            )
            .expect("repeat confirmation"),
        AccountConfirmationOutcome {
            status: AccountConfirmationStatus::AlreadyConfirmed,
        }
    );
    let repeated_audit_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log \
             WHERE entity_id IN ('candidate-one', 'candidate-two') \
               AND action = 'candidate_account_accepted'",
            [],
            |row| row.get(0),
        )
        .expect("count repeated confirmation audits");
    assert_eq!(repeated_audit_count, 2);
}

#[test]
fn rejects_a_stale_candidate_confirmation_without_writing() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_candidate_account(
        &store,
        "candidate-one",
        "source-dbs",
        "Everyday",
        "deposit_account",
        Some("••001"),
        Some("SGD"),
    );
    seed_candidate_account(
        &store,
        "candidate-two",
        "source-dbs",
        "Savings",
        "deposit_account",
        Some("••002"),
        Some("SGD"),
    );
    let prompt = store
        .list_account_confirmation_prompts()
        .expect("list confirmation prompt")
        .pop()
        .expect("candidate prompt");
    let expected = prompt
        .candidate_accounts
        .iter()
        .map(|candidate| candidate.account_id.clone())
        .collect::<Vec<_>>();
    seed_candidate_account(
        &store,
        "candidate-new",
        "source-dbs",
        "New account",
        "deposit_account",
        Some("••003"),
        Some("SGD"),
    );

    assert_eq!(
        store
            .decide_candidate_accounts(
                "source-dbs",
                &prompt.proposal_version,
                &expected
                    .iter()
                    .cloned()
                    .map(|account_id| CandidateAccountDecisionInput {
                        account_id,
                        action: CandidateAccountDecision::Accept,
                    })
                    .collect::<Vec<_>>(),
                "audit-stale",
            )
            .expect("reject stale confirmation"),
        AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Conflict,
        }
    );
    let candidate_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM accounts \
             WHERE money_source_id = 'source-dbs' AND status = 'candidate'",
            [],
            |row| row.get(0),
        )
        .expect("count unchanged candidates");
    let audit_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log \
             WHERE entity_id = 'source-dbs' AND action = 'candidate_accounts_confirmed'",
            [],
            |row| row.get(0),
        )
        .expect("count confirmation audits");
    assert_eq!(candidate_count, 3);
    assert_eq!(audit_count, 0);
}

#[test]
fn rejects_a_changed_candidate_proposal_with_the_same_account_ids() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_candidate_account(
        &store,
        "candidate-one",
        "source-dbs",
        "Everyday",
        "deposit_account",
        Some("••001"),
        Some("SGD"),
    );
    let prompt = store
        .list_account_confirmation_prompts()
        .expect("list candidate proposal")
        .pop()
        .expect("candidate prompt");
    store
        .connection
        .execute(
            "UPDATE accounts SET masked_identifier = '••002' WHERE id = 'candidate-one'",
            [],
        )
        .expect("change candidate proposal field");
    let current = store
        .list_account_confirmation_prompts()
        .expect("read changed candidate proposal")
        .pop()
        .expect("changed candidate prompt");
    assert_eq!(
        current
            .candidate_accounts
            .iter()
            .map(|candidate| candidate.account_id.as_str())
            .collect::<Vec<_>>(),
        vec!["candidate-one"]
    );
    assert_ne!(current.proposal_version, prompt.proposal_version);

    assert_eq!(
        store
            .decide_candidate_accounts(
                "source-dbs",
                &prompt.proposal_version,
                &[CandidateAccountDecisionInput {
                    account_id: "candidate-one".to_owned(),
                    action: CandidateAccountDecision::Accept,
                }],
                "audit-stale-proposal",
            )
            .expect("reject changed proposal"),
        AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Conflict,
        }
    );
    let status: String = store
        .connection
        .query_row(
            "SELECT status FROM accounts WHERE id = 'candidate-one'",
            [],
            |row| row.get(0),
        )
        .expect("read unchanged candidate status");
    assert_eq!(status, "candidate");
    let audit_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log WHERE id = 'audit-stale-proposal:candidate-one'",
            [],
            |row| row.get(0),
        )
        .expect("count stale-proposal audits");
    assert_eq!(audit_count, 0);
}

#[test]
fn dismisses_and_restores_a_candidate_account_with_audited_state() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_candidate_account(
        &store,
        "candidate-dismissed",
        "source-dbs",
        "Everyday",
        "deposit_account",
        Some("••001"),
        Some("SGD"),
    );

    let proposal_version = store
        .list_account_confirmation_prompts()
        .expect("read candidate proposal")[0]
        .proposal_version
        .clone();
    assert_eq!(
        store
            .decide_candidate_accounts(
                "source-dbs",
                &proposal_version,
                &[CandidateAccountDecisionInput {
                    account_id: "candidate-dismissed".to_owned(),
                    action: CandidateAccountDecision::Dismiss,
                }],
                "audit-dismiss",
            )
            .expect("dismiss candidate"),
        AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Updated,
        }
    );
    assert_eq!(
        store
            .list_account_confirmation_prompts()
            .expect("read dismissed prompt")[0]
            .dismissed_accounts[0]
            .account_id,
        "candidate-dismissed"
    );

    assert_eq!(
        store
            .restore_dismissed_candidate_account("candidate-dismissed", "audit-restore")
            .expect("restore candidate"),
        AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Restored,
        }
    );
    assert_eq!(
        store
            .list_account_confirmation_prompts()
            .expect("read restored prompt")[0]
            .candidate_accounts[0]
            .account_id,
        "candidate-dismissed"
    );
    let actions = store
        .connection
        .prepare("SELECT action FROM audit_log WHERE entity_id = ?1 ORDER BY id")
        .expect("prepare audit query")
        .query_map(["candidate-dismissed"], |row| row.get::<_, String>(0))
        .expect("query audit actions")
        .collect::<Result<Vec<_>, _>>()
        .expect("read audit actions");
    assert_eq!(
        actions,
        vec![
            "candidate_account_dismissed".to_owned(),
            "candidate_account_restored".to_owned(),
        ]
    );
    assert_eq!(
        store
            .restore_dismissed_candidate_account("candidate-dismissed", "audit-repeat")
            .expect("reject duplicate restore"),
        AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Conflict,
        }
    );
}
