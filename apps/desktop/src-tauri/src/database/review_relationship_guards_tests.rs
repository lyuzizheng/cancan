use super::tests::{
    open_store, prepared_repayment, prepared_transfer, seed_review_repayment, seed_review_transfer,
};
use super::*;

/// A forged invalid proposal built against the staged pair it describes.
type InvalidShapeBuilder = Box<dyn Fn(&mut ManualImportStore) -> CorePreparedReviewEvent>;

/// Q06: the host enforces the full relationship contract at acceptance and
/// commit — shared currency, distinct accounts, per-type window, valid
/// account kinds, and exact non-zero equal magnitude — instead of trusting a
/// proposal that bypasses the sidecar. Six invalid shapes are forged against
/// staged records that echo them, so the commit echo check passes and only
/// the guard under test decides; every case is refused at accept with no
/// partial `review_relationships` row.
#[test]
fn refuses_invalid_relationships_at_acceptance_without_partial_writes() {
    // Each shape runs against the staged pair it describes, so accept reads
    // current sides whose full attributes the forged proposal violates.
    let shapes: Vec<(&str, InvalidShapeBuilder)> = vec![
        // Cross-currency pair: one leg spends USD while the staged records
        // and the other leg stay SGD; equal magnitudes must not pass.
        (
            "mixed currency",
            Box::new(|store| {
                store
                    .connection
                    .execute(
                        "UPDATE external_records SET currency = 'USD' \
                             WHERE id = 'record-dbs-savings'",
                        [],
                    )
                    .expect("stage cross-currency candidate");
                let mut event = prepared_transfer("250.00");
                event.legs[0].currency = "USD".to_owned();
                event
            }),
        ),
        // Same-account pair: both legs point at the HSBC account.
        (
            "same account",
            Box::new(|store| {
                store
                    .connection
                    .execute(
                        "UPDATE external_records SET account_id = 'account-hsbc-cash' \
                             WHERE id = 'record-dbs-savings'",
                        [],
                    )
                    .expect("stage same-account candidate");
                let mut event = prepared_transfer("250.00");
                event.legs[0].account_id = "account-hsbc-cash".to_owned();
                event
            }),
        ),
        // Over-window pair: the candidate posts ten days after the anchor,
        // beyond the three-day transfer window.
        (
            "over window",
            Box::new(|store| {
                store
                    .connection
                    .execute(
                        "UPDATE external_records SET posted_on = '2026-08-12' \
                             WHERE id = 'record-dbs-savings'",
                        [],
                    )
                    .expect("stage over-window candidate");
                prepared_transfer("250.00")
            }),
        ),
        // Zero pair: no money moves.
        (
            "zero magnitude",
            Box::new(|store| {
                store
                    .connection
                    .execute(
                        "UPDATE external_records SET account_balance_delta = '0.00' \
                             WHERE id = 'record-dbs-savings'",
                        [],
                    )
                    .expect("stage zero-magnitude candidate");
                prepared_transfer("0.00")
            }),
        ),
        // Unequal magnitudes that only differ past a naive zero-strip.
        ("unequal magnitude", Box::new(|_| prepared_transfer("10"))),
        // Wrong signs for a transfer: both sides decrease.
        ("same sign", Box::new(|_| prepared_transfer("-250.00"))),
    ];
    for (name, build) in shapes {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_transfer(&mut store, "250.00");
        let event = build(&mut store);
        let refused = store
            .accept_review_relationship(
                "review-record-hsbc-transfer",
                1,
                "record-dbs-savings",
                1,
                &event,
            )
            .expect("refuse the invalid relationship at acceptance");
        assert_eq!(
            (refused.status, refused.reason),
            (
                ReviewMutationStatus::Conflict,
                Some("invalid_relationship_request")
            ),
            "{name}",
        );
        let relationships: i64 = store
            .connection
            .query_row("SELECT count(*) FROM review_relationships", [], |row| {
                row.get(0)
            })
            .expect("count relationships");
        assert_eq!(relationships, 0, "{name}: no partial relationship write");
        let audits: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log WHERE action = 'review_relationship_accepted'",
                [],
                |row| row.get(0),
            )
            .expect("count acceptance audits");
        assert_eq!(audits, 0, "{name}: no acceptance audit without a row");
    }
}

/// Q06: repayment kinds are enforced on the stored sides. A forged repayment
/// between two deposit accounts (no card side) is refused at acceptance even
/// though both legs decrease with equal magnitude.
#[test]
fn refuses_a_repayment_without_a_card_and_cash_like_pair_at_acceptance() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_transfer(&mut store, "250.00");
    // Repurpose the transfer pair as two deposit accounts with two decreases.
    store
        .connection
        .execute(
            "UPDATE external_records SET event_type = 'credit_card_repayment', \
                   account_balance_delta = '-250.00'",
            [],
        )
        .expect("stage two-deposit repayment sides");
    let mut event = prepared_transfer("-250.00");
    event.event_type = "credit_card_repayment".to_owned();
    let refused = store
        .accept_review_relationship(
            "review-record-hsbc-transfer",
            1,
            "record-dbs-savings",
            1,
            &event,
        )
        .expect("refuse the kind-invalid repayment at acceptance");
    assert_eq!(
        (refused.status, refused.reason),
        (
            ReviewMutationStatus::Conflict,
            Some("invalid_relationship_request")
        ),
    );
    let relationships: i64 = store
        .connection
        .query_row("SELECT count(*) FROM review_relationships", [], |row| {
            row.get(0)
        })
        .expect("count relationships");
    assert_eq!(relationships, 0, "no partial relationship write");
}

/// Q06: an over-window repayment is refused at acceptance. The seeded pair is
/// one day apart and valid; moving the card side eight days out exceeds the
/// seven-day repayment window while magnitude, signs, currency, and kinds
/// stay exact.
#[test]
fn refuses_an_over_window_repayment_at_acceptance() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    store
        .connection
        .execute(
            "UPDATE external_records SET posted_on = '2026-07-08' \
             WHERE id = 'record-dbs-card'",
            [],
        )
        .expect("stage over-window repayment side");
    let refused = store
        .accept_review_relationship(
            "review-record-hsbc-cash",
            1,
            "record-dbs-card",
            1,
            &prepared_repayment(),
        )
        .expect("refuse the over-window repayment at acceptance");
    assert_eq!(
        (refused.status, refused.reason),
        (
            ReviewMutationStatus::Conflict,
            Some("invalid_relationship_request")
        ),
    );
    let relationships: i64 = store
        .connection
        .query_row("SELECT count(*) FROM review_relationships", [], |row| {
            row.get(0)
        })
        .expect("count relationships");
    assert_eq!(relationships, 0, "no partial relationship write");
}

/// Q06: the commit re-checks the stored sides, so a relationship that was
/// accepted while valid but whose staged side moved out of contract before
/// commit (here: the candidate's account flips onto the anchor's account
/// after acceptance) stays in Review with no ledger write. The forged
/// proposal echoes the moved side, so only the host guard decides.
#[test]
fn refuses_a_relationship_that_left_contract_between_accept_and_commit() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_transfer(&mut store, "250.00");
    let accepted = store
        .accept_review_relationship(
            "review-record-hsbc-transfer",
            1,
            "record-dbs-savings",
            1,
            &prepared_transfer("250.00"),
        )
        .expect("accept the valid transfer");
    assert_eq!(
        (accepted.status, accepted.reason),
        (ReviewMutationStatus::RelationshipAccepted, None),
    );
    // The staged side moves out of contract after acceptance.
    store
        .connection
        .execute(
            "UPDATE external_records SET account_id = 'account-hsbc-cash' \
             WHERE id = 'record-dbs-savings'",
            [],
        )
        .expect("move the staged side out of contract");
    let job = store
        .enqueue_commit_review_batch(&[
            "review-record-hsbc-transfer".to_owned(),
            "review-record-dbs-savings".to_owned(),
        ])
        .expect("enqueue selected relationship");
    let claimed = store
        .claim_review_batch(&job.job_id, "test-worker")
        .expect("claim batch")
        .expect("queued job");
    let (groups, outcomes) = store
        .prepare_commit_review_groups(&claimed)
        .expect("prepare selected relationship");
    assert!(outcomes.is_empty(), "{outcomes:?}");
    assert_eq!(groups.len(), 1);
    let mut forged = prepared_transfer("250.00");
    forged.legs[0].account_id = "account-hsbc-cash".to_owned();
    let refused = store
        .commit_prepared_review_group(&claimed, &groups[0], &forged)
        .expect("refuse the out-of-contract commit");
    assert_eq!(refused.status, ReviewBatchGroupStatus::StillNeedsReview);
    assert_eq!(
        refused.reason.as_deref(),
        Some("relationship_amounts_unbalanced")
    );
    let (events, status): (i64, String) = store
        .connection
        .query_row(
            "SELECT (SELECT count(*) FROM ledger_events), \
                    (SELECT status FROM review_relationships LIMIT 1)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("count events and read relationship status");
    assert_eq!(
        events, 0,
        "an out-of-contract commit writes no ledger event"
    );
    assert_eq!(
        status, "accepted",
        "the accepted row stays for a valid retry"
    );
}

/// Q06: legs that disagree with the stored sides are refused at acceptance.
/// The staged pair is a valid transfer, but the forged proposal swaps in a
/// stranger account on one leg; magnitude and signs still balance.
#[test]
fn refuses_a_proposal_whose_legs_disagree_with_the_stored_sides() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_transfer(&mut store, "250.00");
    let mut forged = prepared_transfer("250.00");
    forged.legs[0].account_id = "account-stranger".to_owned();
    let refused = store
        .accept_review_relationship(
            "review-record-hsbc-transfer",
            1,
            "record-dbs-savings",
            1,
            &forged,
        )
        .expect("refuse the legs-mismatched proposal");
    assert_eq!(
        (refused.status, refused.reason),
        (
            ReviewMutationStatus::Conflict,
            Some("invalid_relationship_request")
        ),
    );
    let relationships: i64 = store
        .connection
        .query_row("SELECT count(*) FROM review_relationships", [], |row| {
            row.get(0)
        })
        .expect("count relationships");
    assert_eq!(relationships, 0, "no partial relationship write");
}
