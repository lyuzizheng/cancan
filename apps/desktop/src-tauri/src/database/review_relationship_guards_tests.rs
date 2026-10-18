use super::tests::{
    open_store, prepared_repayment, prepared_transfer, seed_review_repayment, seed_review_transfer,
};
use super::*;

/// A forged invalid proposal built against the staged pair it describes.
type InvalidShapeBuilder = Box<dyn Fn(&mut ManualImportStore) -> CorePreparedReviewEvent>;

/// Enqueues, claims, and prepares the one accepted pair under test, so every
/// commit-side case starts from the same prepared baseline: one group and no
/// outcomes.
fn prepare_single_group(
    store: &mut ManualImportStore,
    review_item_ids: [&str; 2],
) -> (ClaimedReviewBatch, CommitReviewGroup) {
    let job = store
        .enqueue_commit_review_batch(&review_item_ids.map(str::to_owned))
        .expect("enqueue selected relationship");
    let claimed = store
        .claim_review_batch(&job.job_id, "test-worker")
        .expect("claim batch")
        .expect("queued job");
    let (mut groups, outcomes) = store
        .prepare_commit_review_groups(&claimed)
        .expect("prepare selected relationship");
    assert!(outcomes.is_empty(), "{outcomes:?}");
    assert_eq!(groups.len(), 1, "one accepted pair prepares one group");
    (claimed, groups.remove(0))
}

/// Everything a successful prepared-group commit writes: the ledger event,
/// its legs, and its match edges, the two committed record statuses, the
/// resolved review items, the committed relationship, and the commit audit
/// entry. Reading it before and after a refused commit proves the failed
/// attempt left no write of any kind behind.
#[derive(Debug, PartialEq)]
struct CommitWriteSurface {
    ledger_events: i64,
    ledger_legs: i64,
    match_edges: i64,
    committed_records: i64,
    resolved_review_items: i64,
    relationship_status: String,
    commit_audits: i64,
}

impl CommitWriteSurface {
    fn read(store: &ManualImportStore) -> Self {
        store
            .connection
            .query_row(
                "SELECT (SELECT count(*) FROM ledger_events), \
                        (SELECT count(*) FROM ledger_legs), \
                        (SELECT count(*) FROM match_edges), \
                        (SELECT count(*) FROM external_records WHERE status = 'committed'), \
                        (SELECT count(*) FROM review_items WHERE status = 'resolved'), \
                        (SELECT status FROM review_relationships), \
                        (SELECT count(*) FROM audit_log WHERE action = 'review_batch_committed')",
                [],
                |row| {
                    Ok(Self {
                        ledger_events: row.get(0)?,
                        ledger_legs: row.get(1)?,
                        match_edges: row.get(2)?,
                        committed_records: row.get(3)?,
                        resolved_review_items: row.get(4)?,
                        relationship_status: row.get(5)?,
                        commit_audits: row.get(6)?,
                    })
                },
            )
            .expect("read the commit write surface")
    }
}

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
        // and the other leg stay SGD; equal magnitudes must not pass. The USD
        // fiat instrument is seeded so the accept loaders resolve the staged
        // side and only the shared-currency guard decides.
        (
            "mixed currency",
            Box::new(|store| {
                store
                    .connection
                    .execute(
                        "INSERT INTO instruments(id, instrument_type, symbol, currency, display_name) \
                         VALUES ('instrument-usd', 'fiat_currency', 'USD', 'USD', 'US Dollar')",
                        [],
                    )
                    .expect("seed USD instrument");
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
                event.legs[0].instrument_id = "instrument-usd".to_owned();
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
    let (claimed, group) = prepare_single_group(
        &mut store,
        ["review-record-hsbc-transfer", "review-record-dbs-savings"],
    );
    let mut forged = prepared_transfer("250.00");
    forged.legs[0].account_id = "account-hsbc-cash".to_owned();
    let refused = store
        .commit_prepared_review_group(&claimed, &group, &forged)
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

/// Q06: the commit re-checks the per-type window on the stored sides, so a
/// transfer that was accepted while valid but whose candidate date moved
/// beyond the three-day window before commit stays in Review with no write.
/// The legs remain valid — distinct accounts, one shared currency, exact
/// non-zero equal magnitudes with opposite signs — and they carry no date, so
/// only the commit-side relationship guard can refuse this drift.
#[test]
fn refuses_a_relationship_whose_side_left_the_transfer_window_before_commit() {
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
        .expect("accept the in-window transfer");
    assert_eq!(
        (accepted.status, accepted.reason),
        (ReviewMutationStatus::RelationshipAccepted, None),
    );
    // Ten days after the anchor, beyond the three-day transfer window; the
    // accounts, deltas, signs, and currency stay exactly as accepted.
    store
        .connection
        .execute(
            "UPDATE external_records SET posted_on = '2026-08-12' \
             WHERE id = 'record-dbs-savings'",
            [],
        )
        .expect("move the staged side out of the transfer window");
    let (claimed, group) = prepare_single_group(
        &mut store,
        ["review-record-hsbc-transfer", "review-record-dbs-savings"],
    );
    let baseline = CommitWriteSurface::read(&store);
    assert_eq!(
        baseline.relationship_status, "accepted",
        "the prepared baseline is the accepted pair"
    );
    let refused = store
        .commit_prepared_review_group(&claimed, &group, &prepared_transfer("250.00"))
        .expect("refuse the out-of-window commit");
    assert_eq!(
        (refused.status, refused.reason.as_deref()),
        (
            ReviewBatchGroupStatus::StillNeedsReview,
            Some("relationship_amounts_unbalanced")
        ),
    );
    assert_eq!(
        CommitWriteSurface::read(&store),
        baseline,
        "an out-of-window commit writes nothing"
    );
}

/// Q06: the commit re-checks the repayment kind shape on the stored sides, so
/// a repayment that was accepted while valid but whose card account was
/// reclassified out of `credit_card` before commit stays in Review with no
/// write. Both legs keep two distinct accounts, two equal decreases, and one
/// shared currency, so only the commit-side relationship guard can refuse
/// this drift.
#[test]
fn refuses_a_relationship_whose_side_left_the_repayment_kind_contract_before_commit() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    let accepted = store
        .accept_review_relationship(
            "review-record-hsbc-cash",
            1,
            "record-dbs-card",
            1,
            &prepared_repayment(),
        )
        .expect("accept the valid repayment");
    assert_eq!(
        (accepted.status, accepted.reason),
        (ReviewMutationStatus::RelationshipAccepted, None),
    );
    // Both sides become cash-like once the card side is reclassified; the
    // accounts, deltas, signs, and currency stay exactly as accepted.
    store
        .connection
        .execute(
            "UPDATE accounts SET account_type = 'deposit_account' \
             WHERE id = 'account-dbs-card'",
            [],
        )
        .expect("reclassify the card side out of the repayment contract");
    let (claimed, group) = prepare_single_group(
        &mut store,
        ["review-record-hsbc-cash", "review-record-dbs-card"],
    );
    let baseline = CommitWriteSurface::read(&store);
    assert_eq!(
        baseline.relationship_status, "accepted",
        "the prepared baseline is the accepted pair"
    );
    let refused = store
        .commit_prepared_review_group(&claimed, &group, &prepared_repayment())
        .expect("refuse the kind-invalid commit");
    assert_eq!(
        (refused.status, refused.reason.as_deref()),
        (
            ReviewBatchGroupStatus::StillNeedsReview,
            Some("relationship_amounts_unbalanced")
        ),
    );
    assert_eq!(
        CommitWriteSurface::read(&store),
        baseline,
        "a kind-invalid commit writes nothing"
    );
}

/// Q06 (BRAWUKA-903 follow-up): the acceptance echo binds the stored fiat
/// instrument, mirroring the commit's `event_matches_group`. A proposal whose
/// leg carries a wrong instrument but the right account, currency, and amount
/// is refused at acceptance with `invalid_relationship_request` — not stranded
/// as `accepted` and refused later at commit with `core_preflight_failed`.
#[test]
fn refuses_a_proposal_whose_leg_carries_the_wrong_instrument() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_transfer(&mut store, "250.00");
    let mut forged = prepared_transfer("250.00");
    forged.legs[0].instrument_id = "instrument-stranger".to_owned();
    let refused = store
        .accept_review_relationship(
            "review-record-hsbc-transfer",
            1,
            "record-dbs-savings",
            1,
            &forged,
        )
        .expect("refuse the instrument-mismatched proposal");
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
