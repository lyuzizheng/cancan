//! The opt-in background-intake notification: the setting that turns it on, the
//! redacted summary it delivers, and the click that reopens the window on the
//! batch's Tasks row.
//!
//! Delivery is a platform seam ([`IntakeNotificationDelivery`]) because the
//! decision logic is testable on its own: what a completed batch makes of the
//! setting, the permission, and window visibility is asserted in tests with a
//! recorder, while macOS only has to show banners.

use super::*;
use std::sync::atomic::Ordering;

/// The device-local record of the toggle, one magic-prefixed byte, matching
/// the other device-local status files in the Vault root.
const INTAKE_NOTIFICATION_STATUS_MAGIC: &[u8; 8] = b"CCNOTE01";
const INTAKE_NOTIFICATION_STATUS_FILE_NAME: &str = "intake-notifications.status";

/// Stable per batch, so a retry after a failed delivery replaces the banner
/// instead of stacking a second copy of the same result on the Notification
/// Center.
pub(super) const INTAKE_NOTIFICATION_IDENTIFIER_PREFIX: &str = "intake-batch:";
const INTAKE_NOTIFICATION_TITLE: &str = "CanCan";

/// The Notifications pane of System Settings on macOS 13 and later.
#[cfg(target_os = "macos")]
const NOTIFICATION_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.Notifications-Settings.extension";

/// What macOS last decided about CanCan's authorization to post notifications.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum IntakeNotificationPermission {
    NotDetermined,
    Authorized,
    Denied,
}

/// The Settings surface's read model: the opt-in toggle and the system's answer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct IntakeNotificationSettings {
    pub(crate) enabled: bool,
    pub(crate) permission: IntakeNotificationPermission,
}

/// One redacted batch notification: counts only, never a filename, provider,
/// account, or amount.
pub(crate) struct IntakeNotification {
    pub(crate) identifier: String,
    pub(crate) title: String,
    pub(crate) body: String,
}

/// The platform seam. The production implementation talks to
/// `UNUserNotificationCenter`; tests inject a recorder so the setting,
/// permission, and one-delivery-per-batch rules stay deterministic.
pub(super) trait IntakeNotificationDelivery: Send + Sync {
    fn permission(&self) -> IntakeNotificationPermission;
    /// Asks the system once, at the moment the user turns the setting on.
    fn request_permission(&self) -> IntakeNotificationPermission;
    fn deliver(&self, notification: &IntakeNotification) -> Result<(), RuntimeError>;
}

/// The delivery for this platform. Only macOS posts system notifications; the
/// background window lifecycle is macOS-only in this phase.
pub(super) fn system_intake_notification_delivery() -> Arc<dyn IntakeNotificationDelivery> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(notifications_macos::MacIntakeNotificationDelivery)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Arc::new(InertIntakeNotificationDelivery)
    }
}

/// Delivery for platforms (and tests) that never post a system notification:
/// nothing is ever authorized, so no batch is ever eligible.
#[cfg(not(target_os = "macos"))]
pub(super) struct InertIntakeNotificationDelivery;

#[cfg(not(target_os = "macos"))]
impl IntakeNotificationDelivery for InertIntakeNotificationDelivery {
    fn permission(&self) -> IntakeNotificationPermission {
        IntakeNotificationPermission::NotDetermined
    }

    fn request_permission(&self) -> IntakeNotificationPermission {
        IntakeNotificationPermission::NotDetermined
    }

    fn deliver(&self, _notification: &IntakeNotification) -> Result<(), RuntimeError> {
        Err(RuntimeError::new("intake_notifications_unavailable"))
    }
}

/// Connects the platform's click handler before any notification can be
/// delivered, so a click that arrives before the window exists still routes.
pub(crate) fn install_intake_notifications(app: &AppHandle, runtime: &VaultRuntime) {
    #[cfg(target_os = "macos")]
    {
        notifications_macos::install(app, runtime);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, runtime);
    }
}

impl VaultRuntime {
    pub(crate) fn intake_notification_settings(&self) -> IntakeNotificationSettings {
        IntakeNotificationSettings {
            enabled: self.intake_notifications_enabled(),
            permission: self.inner.intake_notifications.permission(),
        }
    }

    /// The opt-in toggle. Turning it on is the only moment CanCan asks macOS for
    /// permission; turning it off never revokes anything, it just stops
    /// deliveries. A refused permission leaves the setting on but ineligible:
    /// the user asked for notifications and macOS said no, so the switch keeps
    /// the user's intent while nothing is delivered and nothing errors.
    pub(crate) fn set_intake_notifications_enabled(
        &self,
        enabled: bool,
    ) -> Result<IntakeNotificationSettings, RuntimeError> {
        if enabled {
            self.inner.intake_notifications.request_permission();
        }
        write_intake_notification_status(&self.inner.root, enabled).map_err(|error| {
            runtime_failure(
                self,
                "intake_notification_status_write",
                "intake_notification_setting_failed",
                &error,
            )
        })?;
        self.inner
            .intake_notifications_enabled
            .store(enabled, Ordering::SeqCst);
        Ok(self.intake_notification_settings())
    }

    fn intake_notifications_enabled(&self) -> bool {
        self.inner
            .intake_notifications_enabled
            .load(Ordering::SeqCst)
    }

    /// Whether a completed batch right now would actually reach the user's
    /// Notification Center: opted in, authorized, and no visible window that
    /// already shows the result.
    pub(super) fn intake_notification_eligible(&self) -> bool {
        self.intake_notifications_enabled()
            && self.inner.intake_notifications.permission()
                == IntakeNotificationPermission::Authorized
    }

    /// Delivers every notification a completed batch still owes, oldest first.
    /// A batch is marked emitted only after the system accepted its request, so
    /// a failed delivery retries under the same identifier at the next pass
    /// instead of being lost.
    pub(super) fn deliver_pending_intake_notifications(&self) -> Result<(), RuntimeError> {
        let pending = {
            let store_guard = self.store()?;
            let store = store_guard
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            store.pending_intake_notifications().map_store_error(
                store,
                "pending_intake_notifications",
                "tasks_read_failed",
            )?
        };
        for entry in pending {
            let notification = IntakeNotification {
                identifier: intake_notification_identifier(&entry.batch_id),
                title: INTAKE_NOTIFICATION_TITLE.to_string(),
                body: intake_notification_body(entry.ready, entry.needs_action),
            };
            if let Err(error) = self.inner.intake_notifications.deliver(&notification) {
                // The batch stays pending; the next pass retries it under the
                // same identifier. Delivery is best-effort and never an error
                // on a path the user is watching.
                eprintln!(
                    "intake notification delivery failed: {} ({})",
                    error.code, entry.batch_id
                );
                return Ok(());
            }
            let mut store_guard = self.store()?;
            let store = store_guard
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            store
                .mark_intake_notification_emitted(&entry.batch_id)
                .map_store_error(
                    store,
                    "mark_intake_notification_emitted",
                    "tasks_write_failed",
                )?;
        }
        Ok(())
    }

    /// Remembers the batch a notification click asked to open. The route
    /// survives until the renderer pulls it, including across the window
    /// rebuild the click triggers.
    pub(super) fn record_background_intake_route(&self, batch_id: &str) {
        if let Ok(mut route) = self.inner.pending_intake_route.lock() {
            *route = Some(batch_id.to_owned());
        }
    }

    /// Takes the pending route as the Tasks row the user should land on: the
    /// most urgent row of the batch that asked for the window. Taking it
    /// clears it, so one click opens one row once — a later rebuild cannot
    /// replay a stale route. A route that no longer resolves (the row was
    /// resolved while the window was closed) simply opens nothing.
    ///
    /// The route is only dropped once it has been resolved: a pull that cannot
    /// read the projection (the Vault is locked) leaves the click pending for
    /// the pull that can, instead of swallowing the click.
    pub(crate) fn take_background_intake_route(&self) -> Result<Option<TaskRow>, RuntimeError> {
        let pending = self
            .inner
            .pending_intake_route
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))?
            .clone();
        let Some(batch_id) = pending else {
            return Ok(None);
        };
        let row = {
            let store_guard = self.store()?;
            let store = store_guard
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            let rows = store.derive_batch_task_rows(&batch_id).map_store_error(
                store,
                "derive_batch_task_rows",
                "list_tasks_failed",
            )?;
            rows.into_iter().map(raw_task_to_row).min_by(|left, right| {
                task_row_urgency(left)
                    .cmp(&task_row_urgency(right))
                    .then_with(|| right.timestamp.cmp(&left.timestamp))
            })
        };
        let mut route = self
            .inner
            .pending_intake_route
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))?;
        if route.as_deref() == Some(batch_id.as_str()) {
            *route = None;
        }
        Ok(row)
    }
}

fn intake_notification_identifier(batch_id: &str) -> String {
    format!("{INTAKE_NOTIFICATION_IDENTIFIER_PREFIX}{batch_id}")
}

/// Redacted, count-only body text (spec 0017). The two shapes the spec names
/// are the two single-outcome cases; the mixed and plural cases follow the same
/// grammar so the text stays truthful whatever the batch held.
pub(crate) fn intake_notification_body(ready: usize, needs_action: usize) -> String {
    let processed = ready + needs_action;
    match (ready, needs_action) {
        (0, 0) => format!("CanCan processed {}.", file_count(processed)),
        (0, 1) if processed == 1 => {
            "A statement needs action. Open CanCan to continue.".to_string()
        }
        (0, needs_action) => format!(
            "CanCan processed {}. {}.",
            file_count(processed),
            needs_action_clause(needs_action)
        ),
        (ready, 0) => format!(
            "CanCan processed {}. {}.",
            file_count(processed),
            ready_clause(ready)
        ),
        (ready, needs_action) => format!(
            "CanCan processed {}. {} and {}.",
            file_count(processed),
            ready_clause(ready),
            needs_action_clause(needs_action)
        ),
    }
}

fn file_count(count: usize) -> String {
    if count == 1 {
        "1 file".to_string()
    } else {
        format!("{count} files")
    }
}

fn ready_clause(ready: usize) -> String {
    if ready == 1 {
        "1 is ready".to_string()
    } else {
        format!("{ready} are ready")
    }
}

fn needs_action_clause(needs_action: usize) -> String {
    if needs_action == 1 {
        "1 needs action".to_string()
    } else {
        format!("{needs_action} need action")
    }
}

/// The toggle's device-local record. It is written next to the Vault so it
/// survives restarts without the Vault being unlocked; the byte is only ever
/// read back at startup, where an absent or malformed record means Off.
fn write_intake_notification_status(root: &Path, enabled: bool) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(INTAKE_NOTIFICATION_STATUS_MAGIC.len() + 1);
    bytes.extend_from_slice(INTAKE_NOTIFICATION_STATUS_MAGIC);
    bytes.push(u8::from(enabled));
    write_atomic(&root.join(INTAKE_NOTIFICATION_STATUS_FILE_NAME), &bytes)
}

pub(super) fn read_intake_notification_status(root: &Path) -> bool {
    let Ok(status) = fs::read(root.join(INTAKE_NOTIFICATION_STATUS_FILE_NAME)) else {
        return false;
    };
    status.len() == INTAKE_NOTIFICATION_STATUS_MAGIC.len() + 1
        && &status[..INTAKE_NOTIFICATION_STATUS_MAGIC.len()] == INTAKE_NOTIFICATION_STATUS_MAGIC
        && status[INTAKE_NOTIFICATION_STATUS_MAGIC.len()] == 1
}

#[tauri::command]
pub(crate) async fn intake_notification_settings(
    runtime: State<'_, VaultRuntime>,
) -> Result<IntakeNotificationSettings, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || Ok(runtime.intake_notification_settings())).await
}

#[tauri::command]
pub(crate) async fn set_intake_notifications_enabled(
    enabled: bool,
    runtime: State<'_, VaultRuntime>,
) -> Result<IntakeNotificationSettings, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.set_intake_notifications_enabled(enabled)).await
}

/// Pulled by the renderer once its window is live: the Tasks row a background
/// notification click asked to open, if any.
#[tauri::command]
pub(crate) async fn take_background_intake_route(
    runtime: State<'_, VaultRuntime>,
) -> Result<Option<TaskRow>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.take_background_intake_route()).await
}

/// The one escape hatch a refusal leaves: the Notifications pane, where the
/// user can grant what only they can grant. macOS owns that pane; on other
/// platforms there is no permission to change and the command says so instead
/// of pretending to open something.
#[tauri::command]
pub(crate) async fn open_notification_settings(app: AppHandle) -> Result<(), VaultCommandError> {
    #[cfg(target_os = "macos")]
    {
        #[allow(
            deprecated,
            reason = "the same narrow shell opener the Gmail authorization flow uses until the desktop shell adopts tauri-plugin-opener"
        )]
        app.shell()
            .open(NOTIFICATION_SETTINGS_URL, None)
            .map_err(|error| {
                eprintln!("opening notification settings failed: {error}");
                VaultCommandError::from(RuntimeError::new("system_settings_unavailable"))
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err(RuntimeError::new("system_settings_unavailable").into())
    }
}
