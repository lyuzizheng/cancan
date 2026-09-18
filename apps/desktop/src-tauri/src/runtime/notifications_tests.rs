use super::*;
use crate::runtime::tasks_test_support::{
    finish_pipeline_to_ready, import, runtime_fixture, write_source,
};
use crate::runtime::test_support::{RecordingIntakeNotificationDelivery, notification_runtime};
use std::sync::Arc;

/// A runtime whose toggle is on and whose permission the recorder granted, the
/// state a user reaches by turning the setting on and accepting the prompt.
fn delivery_enabled(
    parent: &std::path::Path,
) -> (VaultRuntime, Arc<RecordingIntakeNotificationDelivery>) {
    let delivery = Arc::new(RecordingIntakeNotificationDelivery::new(
        IntakeNotificationPermission::NotDetermined,
    ));
    let runtime = notification_runtime(&parent.join("vault"), delivery.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .seed_money_source("source-dbs", "dbs", "DBS", "bank")
        .expect("seed Money Source");
    runtime
        .set_intake_notifications_enabled(true)
        .expect("enable background intake notifications");
    (runtime, delivery)
}

fn batch_states(runtime: &VaultRuntime) -> Vec<(String, Option<String>, String)> {
    let store_guard = runtime.store().expect("open store");
    let store = store_guard.as_ref().expect("unlocked store");
    store
        .intake_batch_completion_for_test()
        .expect("batch states")
}

#[test]
fn the_two_shapes_the_spec_names_and_their_plurals() {
    assert_eq!(
        intake_notification_body(2, 1),
        "CanCan processed 3 files. 2 are ready and 1 needs action."
    );
    assert_eq!(
        intake_notification_body(0, 1),
        "A statement needs action. Open CanCan to continue."
    );
    assert_eq!(
        intake_notification_body(1, 0),
        "CanCan processed 1 file. 1 is ready."
    );
    assert_eq!(
        intake_notification_body(0, 3),
        "CanCan processed 3 files. 3 need action."
    );
    assert_eq!(
        intake_notification_body(3, 2),
        "CanCan processed 5 files. 3 are ready and 2 need action."
    );
}

#[test]
fn the_toggle_asks_permission_only_while_turning_on() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let delivery = Arc::new(RecordingIntakeNotificationDelivery::new(
        IntakeNotificationPermission::NotDetermined,
    ));
    let runtime = notification_runtime(&parent.path().join("vault"), delivery.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");

    let settings = runtime.intake_notification_settings();
    assert!(!settings.enabled);
    assert_eq!(
        settings.permission,
        IntakeNotificationPermission::NotDetermined
    );

    let enabled = runtime
        .set_intake_notifications_enabled(true)
        .expect("enable notifications");
    assert!(enabled.enabled);
    assert_eq!(enabled.permission, IntakeNotificationPermission::Authorized);
    assert_eq!(delivery.permission_requests(), 1);

    // macOS only ever asks once; turning the setting off never asks again.
    let disabled = runtime
        .set_intake_notifications_enabled(false)
        .expect("disable notifications");
    assert!(!disabled.enabled);
    assert_eq!(delivery.permission_requests(), 1);
}

#[test]
fn the_toggle_survives_a_restart_and_starts_off_in_a_new_vault() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let root = parent.path().join("vault");
    let (runtime, _delivery) = delivery_enabled(parent.path());
    drop(runtime);

    let restarted = notification_runtime(
        &root,
        Arc::new(RecordingIntakeNotificationDelivery::new(
            IntakeNotificationPermission::Authorized,
        )),
    );
    restarted
        .unlock(b"synthetic-vault-password")
        .expect("unlock the Vault again");
    assert!(restarted.intake_notification_settings().enabled);

    let other = tempfile::tempdir().expect("temporary app data");
    let fresh = notification_runtime(
        &other.path().join("vault"),
        Arc::new(RecordingIntakeNotificationDelivery::new(
            IntakeNotificationPermission::NotDetermined,
        )),
    );
    fresh
        .create(b"synthetic-vault-password")
        .expect("create a second Vault");
    assert!(
        !fresh.intake_notification_settings().enabled,
        "a Vault without the record defaults to Off"
    );
}

#[test]
fn a_background_batch_is_delivered_once_as_a_redacted_summary() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let (runtime, delivery) = delivery_enabled(parent.path());
    let source = write_source(
        parent.path(),
        "dbs-statement.pdf",
        b"%PDF-1.4\nsynthetic ready",
    );
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);

    runtime
        .seal_completed_intake_batches(false)
        .expect("seal completed batches");

    let delivered = delivery.delivered();
    assert_eq!(delivered.len(), 1);
    let (identifier, title, body) = &delivered[0];
    assert_eq!(title, "CanCan");
    assert_eq!(body, "CanCan processed 1 file. 1 is ready.");
    assert!(!body.contains("dbs-statement.pdf"));
    assert!(!body.contains("dbs"));
    assert!(identifier.starts_with("intake-batch:"));

    let states = batch_states(&runtime);
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].2, "emitted");

    // The batch is already delivered, so a later pass has nothing to send.
    runtime
        .seal_completed_intake_batches(false)
        .expect("seal completed batches again");
    assert_eq!(delivery.delivered().len(), 1);
}

#[test]
fn a_visible_window_suppresses_the_batch_without_delivery() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let (runtime, delivery) = delivery_enabled(parent.path());
    let source = write_source(parent.path(), "ready.pdf", b"%PDF-1.4\nsynthetic ready");
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);

    runtime
        .seal_completed_intake_batches(true)
        .expect("seal with a visible window");

    assert!(delivery.delivered().is_empty());
    assert_eq!(batch_states(&runtime)[0].2, "suppressed");

    // Completion is monotonic: a later pass cannot resurrect the batch.
    runtime
        .seal_completed_intake_batches(false)
        .expect("seal again");
    assert!(delivery.delivered().is_empty());
}

#[test]
fn a_denied_permission_reports_the_setting_without_error_or_delivery() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let delivery = Arc::new(RecordingIntakeNotificationDelivery::new(
        IntakeNotificationPermission::Denied,
    ));
    let runtime = notification_runtime(&parent.path().join("vault"), delivery.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .seed_money_source("source-dbs", "dbs", "DBS", "bank")
        .expect("seed Money Source");

    let settings = runtime
        .set_intake_notifications_enabled(true)
        .expect("enabling a denied permission is not an error");
    assert!(settings.enabled);
    assert_eq!(settings.permission, IntakeNotificationPermission::Denied);

    let source = write_source(parent.path(), "ready.pdf", b"%PDF-1.4\nsynthetic ready");
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);
    runtime
        .seal_completed_intake_batches(false)
        .expect("seal with a denied permission");

    assert!(delivery.delivered().is_empty());
    assert_eq!(batch_states(&runtime)[0].2, "suppressed");
}

#[test]
fn a_batch_with_the_toggle_off_is_suppressed_rather_than_left_pending() {
    let (parent, runtime) = runtime_fixture();
    let source = write_source(parent.path(), "ready.pdf", b"%PDF-1.4\nsynthetic ready");
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);

    runtime
        .seal_completed_intake_batches(false)
        .expect("seal with notifications off");

    assert_eq!(batch_states(&runtime)[0].2, "suppressed");
}

#[test]
fn a_refused_delivery_keeps_the_batch_pending_until_it_lands() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let (runtime, delivery) = delivery_enabled(parent.path());
    let source = write_source(parent.path(), "ready.pdf", b"%PDF-1.4\nsynthetic ready");
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);

    delivery.fail_next_deliveries(1);
    runtime
        .seal_completed_intake_batches(false)
        .expect("seal while delivery is refused");
    assert!(delivery.delivered().is_empty());
    assert_eq!(batch_states(&runtime)[0].2, "pending");

    runtime
        .seal_completed_intake_batches(false)
        .expect("seal again");
    let delivered = delivery.delivered();
    assert_eq!(delivered.len(), 1, "the retry delivers the owed batch once");
    assert_eq!(delivered[0].2, "CanCan processed 1 file. 1 is ready.");
    assert_eq!(batch_states(&runtime)[0].2, "emitted");
}

#[test]
fn a_click_routes_to_the_batch_row_and_is_taken_once() {
    let (parent, runtime) = runtime_fixture();
    let source = write_source(parent.path(), "first.pdf", b"%PDF-1.4\nsynthetic first");
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    // The receipt row only renders inside the RecentlyCompleted retention
    // window, so it is stamped with the store's own clock.
    let finalized_at = store.current_timestamp().expect("current timestamp");
    store
        .insert_visible_receipt_batch_for_test(
            "batch-receipt",
            "item-receipt",
            "receipt.pdf",
            &finalized_at,
        )
        .expect("insert visible rejection");
    drop(store_guard);

    runtime.record_background_intake_route("batch-receipt");

    let row = runtime
        .take_background_intake_route()
        .expect("take route")
        .expect("the batch has a row");
    assert_eq!(row.consequence, TaskConsequence::FileNotAdded);
    assert_eq!(
        row.destination,
        TaskDestination::Receipt {
            intake_item_id: "item-receipt".to_string(),
        },
        "the route lands on the batch that asked for the window, not another batch's row"
    );

    // One click opens one row once; a later rebuild cannot replay it.
    assert!(
        runtime
            .take_background_intake_route()
            .expect("take again")
            .is_none()
    );
}

#[test]
fn a_click_for_a_resolved_batch_opens_nothing() {
    let (parent, runtime) = runtime_fixture();
    let _ = parent;
    runtime.record_background_intake_route("batch-that-does-not-exist");
    assert!(
        runtime
            .take_background_intake_route()
            .expect("take route")
            .is_none()
    );
}

#[test]
fn a_route_survives_a_locked_vault_until_the_pull_can_resolve_it() {
    let (parent, runtime) = runtime_fixture();
    let source = write_source(parent.path(), "first.pdf", b"%PDF-1.4\nsynthetic first");
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);
    let batch_id = batch_states(&runtime)
        .into_iter()
        .map(|(batch_id, ..)| batch_id)
        .next()
        .expect("the import sealed a batch");

    runtime.record_background_intake_route(&batch_id);
    runtime.test_support_lock().expect("lock Vault");

    // A window that is still on the lock gate pulls before it can read the
    // projection; that pull must not swallow the click.
    assert_eq!(
        runtime
            .take_background_intake_route()
            .expect_err("locked pull")
            .code,
        "vault_locked"
    );

    runtime
        .unlock(b"synthetic-vault-password")
        .expect("unlock Vault");
    let row = runtime
        .take_background_intake_route()
        .expect("take route")
        .expect("the click is still pending");
    let batch_rows = {
        let store_guard = runtime.store().expect("open store");
        let store = store_guard.as_ref().expect("unlocked store");
        store
            .derive_batch_task_rows(&batch_id)
            .expect("batch rows")
            .into_iter()
            .map(raw_task_to_row)
            .collect::<Vec<_>>()
    };
    assert!(
        batch_rows.contains(&row),
        "the pull opens one of the clicked batch's rows: {row:?} not in {batch_rows:?}"
    );
}
