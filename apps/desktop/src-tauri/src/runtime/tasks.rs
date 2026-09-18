use super::*;
use crate::database::tasks::{RawTask, RawTaskKind};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum TaskFilter {
    CommandCenter,
    Full,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum TaskGroup {
    NeedsAction,
    InProgress,
    RecentlyCompleted,
    Parked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum TaskConsequence {
    Processing,
    PasswordNeeded,
    NewSourceDetected,
    NeedsReview,
    RestoreSourceFile,
    InboxFileCouldNotBeAdded,
    ImportInterrupted,
    NeedsAttention,
    FileNotAdded,
    AlreadyInCancan,
    SameStatementContent,
    SourceFileRestored,
    SourceFileLeftDeleted,
    Ready,
    SourceUnassigned,
    PasswordParked,
    InboxFileParked,
    SaveRecoveryFile,
    SetupReminderPostponed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
// Tagged discriminated union: the renderer switches on `destination.kind` and
// narrows per variant. `rename_all_fields = "camelCase"` camelCases each
// variant's fields in the generated TypeScript contract.
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum TaskDestination {
    Document {
        document_id: String,
    },
    Password {
        document_id: String,
        money_source_id: String,
    },
    SourceConfirmation {
        money_source_candidate_id: String,
    },
    ReviewGroup {
        document_id: String,
    },
    Receipt {
        intake_item_id: String,
    },
    InboxIssue {
        intake_item_id: String,
    },
    RecoverySetup,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct TaskRow {
    pub(crate) row_key: String,
    pub(crate) group: TaskGroup,
    pub(crate) title: String,
    pub(crate) consequence: TaskConsequence,
    pub(crate) timestamp: String,
    pub(crate) destination: TaskDestination,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct Tasks {
    pub(crate) needs_action_count: usize,
    pub(crate) rows: Vec<TaskRow>,
}

impl VaultRuntime {
    pub(crate) fn list_tasks(&self, filter: TaskFilter) -> Result<Tasks, RuntimeError> {
        let store_guard = self.store()?;
        let store = store_guard
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let raw = store
            .derive_task_rows(filter == TaskFilter::Full)
            .map_store_error(store, "derive_task_rows", "list_tasks_failed")?;
        let rows: Vec<TaskRow> = raw.into_iter().map(raw_task_to_row).collect();

        // A document can surface in more than one derivation (e.g. an active
        // reparse alongside a failed reconcile, both keyed `task:document:{id}`).
        // Keep one row per row_key, preferring the most urgent group and then the
        // more recent timestamp, so the renderer never receives duplicate keys.
        let mut best_by_row_key: Vec<(String, TaskRow)> = Vec::new();
        for row in rows {
            match best_by_row_key
                .iter_mut()
                .find(|(key, _)| *key == row.row_key)
            {
                Some((_, existing)) => {
                    if task_row_urgency(&row) > task_row_urgency(existing)
                        || (task_row_urgency(&row) == task_row_urgency(existing)
                            && row.timestamp > existing.timestamp)
                    {
                        *existing = row;
                    }
                }
                None => best_by_row_key.push((row.row_key.clone(), row)),
            }
        }
        let mut rows: Vec<TaskRow> = best_by_row_key.into_iter().map(|(_, row)| row).collect();

        // Recovery setup row, keyed off the reminder:
        //   - reminder in the future  -> postponed row (Parked), even if a
        //     recovery file has already been saved
        //   - reminder in the past    -> "set up recovery file" again
        //   - no reminder, not configured -> "set up recovery file"
        //   - no reminder, configured -> no row
        let recovery_row = match self.recovery_reminder() {
            Some(remind) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| RuntimeError::new("clock_error"))?
                    .as_secs();
                if remind > now {
                    let timestamp = store.format_unix_timestamp(remind).map_store_error(
                        store,
                        "format_unix_timestamp",
                        "list_tasks_failed",
                    )?;
                    Some((
                        TaskGroup::Parked,
                        TaskConsequence::SetupReminderPostponed,
                        "Recovery setup reminder postponed".to_string(),
                        timestamp,
                    ))
                } else {
                    let timestamp = store.current_timestamp().map_store_error(
                        store,
                        "current_timestamp",
                        "list_tasks_failed",
                    )?;
                    Some((
                        TaskGroup::NeedsAction,
                        TaskConsequence::SaveRecoveryFile,
                        "Set up recovery file".to_string(),
                        timestamp,
                    ))
                }
            }
            None if !self.recovery_configured() => {
                let timestamp = store.current_timestamp().map_store_error(
                    store,
                    "current_timestamp",
                    "list_tasks_failed",
                )?;
                Some((
                    TaskGroup::NeedsAction,
                    TaskConsequence::SaveRecoveryFile,
                    "Set up recovery file".to_string(),
                    timestamp,
                ))
            }
            None => None,
        };
        if let Some((group, consequence, title, timestamp)) = recovery_row {
            rows.push(TaskRow {
                row_key: "task:setup".to_string(),
                group,
                title,
                consequence,
                timestamp,
                destination: TaskDestination::RecoverySetup,
            });
        }

        let mut needs_action = Vec::new();
        let mut in_progress = Vec::new();
        let mut recently_completed = Vec::new();
        let mut parked = Vec::new();
        for row in rows {
            match row.group {
                TaskGroup::NeedsAction => needs_action.push(row),
                TaskGroup::InProgress => in_progress.push(row),
                TaskGroup::RecentlyCompleted => recently_completed.push(row),
                TaskGroup::Parked => parked.push(row),
            }
        }
        needs_action.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        in_progress.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recently_completed.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        parked.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let needs_action_count = needs_action.len();

        let display_rows: Vec<TaskRow> = match filter {
            TaskFilter::CommandCenter => needs_action
                .into_iter()
                .chain(in_progress)
                .chain(recently_completed)
                .take(5)
                .collect(),
            TaskFilter::Full => needs_action
                .into_iter()
                .chain(in_progress)
                .chain(recently_completed)
                .chain(parked)
                .collect(),
        };

        Ok(Tasks {
            needs_action_count,
            rows: display_rows,
        })
    }

    /// Completes sealed intake batches and decides their notification state.
    ///
    /// This is the write half of the Tasks projection and is deliberately not
    /// part of [`Self::list_tasks`]: the read command must not open a write
    /// transaction. It runs wherever intake actually advances — after each
    /// pipeline pump pass and when the Vault opens, so completion stays
    /// monotonic and restart-reconciled.
    pub(crate) fn seal_completed_intake_batches(&self) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store.reconcile_sealed_batches().map_store_error(
            store,
            "reconcile_sealed_batches",
            "seal_batches_failed",
        )
    }
}

/// Lower value = more urgent. NeedsAction beats InProgress beats
/// RecentlyCompleted beats Parked.
fn task_row_urgency(row: &TaskRow) -> u8 {
    match row.group {
        TaskGroup::NeedsAction => 0,
        TaskGroup::InProgress => 1,
        TaskGroup::RecentlyCompleted => 2,
        TaskGroup::Parked => 3,
    }
}

fn raw_task_to_row(raw: RawTask) -> TaskRow {
    let (row_key, group, consequence, destination) = match raw.kind {
        RawTaskKind::Processing { document_id } => (
            format!("task:document:{document_id}"),
            TaskGroup::InProgress,
            TaskConsequence::Processing,
            TaskDestination::Document { document_id },
        ),
        RawTaskKind::PasswordNeeded {
            document_id,
            money_source_id,
        } => (
            format!("task:password:{document_id}"),
            TaskGroup::NeedsAction,
            TaskConsequence::PasswordNeeded,
            TaskDestination::Password {
                document_id,
                money_source_id,
            },
        ),
        RawTaskKind::NewSource { candidate_id } => (
            format!("task:new_source:{candidate_id}"),
            TaskGroup::NeedsAction,
            TaskConsequence::NewSourceDetected,
            TaskDestination::SourceConfirmation {
                money_source_candidate_id: candidate_id,
            },
        ),
        RawTaskKind::NeedsReview { document_id } => (
            format!("task:review:{document_id}"),
            TaskGroup::NeedsAction,
            TaskConsequence::NeedsReview,
            TaskDestination::ReviewGroup { document_id },
        ),
        RawTaskKind::RestoreSourceFile {
            intake_item_id,
            document_id,
        } => (
            format!("task:restore:{intake_item_id}"),
            TaskGroup::NeedsAction,
            TaskConsequence::RestoreSourceFile,
            TaskDestination::Document { document_id },
        ),
        RawTaskKind::InboxFileCouldNotBeAdded { intake_item_id } => (
            format!("task:inbox:{intake_item_id}"),
            TaskGroup::NeedsAction,
            TaskConsequence::InboxFileCouldNotBeAdded,
            TaskDestination::InboxIssue { intake_item_id },
        ),
        RawTaskKind::ImportInterrupted { intake_item_id } => (
            format!("task:inbox:{intake_item_id}"),
            TaskGroup::NeedsAction,
            TaskConsequence::ImportInterrupted,
            TaskDestination::InboxIssue { intake_item_id },
        ),
        RawTaskKind::NeedsAttention { document_id } => (
            format!("task:document:{document_id}"),
            TaskGroup::NeedsAction,
            TaskConsequence::NeedsAttention,
            TaskDestination::Document { document_id },
        ),
        RawTaskKind::FileNotAdded { intake_item_id } => (
            format!("task:receipt:{intake_item_id}"),
            TaskGroup::RecentlyCompleted,
            TaskConsequence::FileNotAdded,
            TaskDestination::Receipt { intake_item_id },
        ),
        RawTaskKind::AlreadyInCancan { intake_item_id } => (
            format!("task:receipt:{intake_item_id}"),
            TaskGroup::RecentlyCompleted,
            TaskConsequence::AlreadyInCancan,
            TaskDestination::Receipt { intake_item_id },
        ),
        RawTaskKind::SameStatementContent { intake_item_id } => (
            format!("task:receipt:{intake_item_id}"),
            TaskGroup::RecentlyCompleted,
            TaskConsequence::SameStatementContent,
            TaskDestination::Receipt { intake_item_id },
        ),
        RawTaskKind::SourceFileRestored { intake_item_id } => (
            format!("task:receipt:{intake_item_id}"),
            TaskGroup::RecentlyCompleted,
            TaskConsequence::SourceFileRestored,
            TaskDestination::Receipt { intake_item_id },
        ),
        RawTaskKind::SourceFileLeftDeleted { intake_item_id } => (
            format!("task:receipt:{intake_item_id}"),
            TaskGroup::RecentlyCompleted,
            TaskConsequence::SourceFileLeftDeleted,
            TaskDestination::Receipt { intake_item_id },
        ),
        RawTaskKind::Ready { intake_item_id } => (
            format!("task:receipt:{intake_item_id}"),
            TaskGroup::RecentlyCompleted,
            TaskConsequence::Ready,
            TaskDestination::Receipt { intake_item_id },
        ),
        RawTaskKind::SourceUnassigned { candidate_id } => (
            format!("task:new_source:{candidate_id}"),
            TaskGroup::Parked,
            TaskConsequence::SourceUnassigned,
            TaskDestination::SourceConfirmation {
                money_source_candidate_id: candidate_id,
            },
        ),
        RawTaskKind::PasswordParked {
            document_id,
            money_source_id,
        } => (
            format!("task:password:{document_id}"),
            TaskGroup::Parked,
            TaskConsequence::PasswordParked,
            TaskDestination::Password {
                document_id,
                money_source_id,
            },
        ),
        RawTaskKind::InboxFileParked { intake_item_id } => (
            format!("task:inbox:{intake_item_id}"),
            TaskGroup::Parked,
            TaskConsequence::InboxFileParked,
            TaskDestination::InboxIssue { intake_item_id },
        ),
    };

    TaskRow {
        row_key,
        group,
        title: raw.title,
        consequence,
        timestamp: raw.timestamp,
        destination,
    }
}

#[tauri::command]
pub(crate) async fn list_tasks(
    filter: TaskFilter,
    runtime: State<'_, VaultRuntime>,
) -> Result<Tasks, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_tasks(filter)).await
}
