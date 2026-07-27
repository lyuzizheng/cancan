use super::*;

#[tauri::command]
pub(crate) async fn undo_committed_event(
    event_id: String,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<UndoOutcome, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let event = {
        let runtime = runtime.clone();
        let event_id = event_id.clone();
        tauri::async_runtime::spawn_blocking(move || {
            runtime.committed_review_event_for_reversal(&event_id)
        })
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    let Some(event) = event else {
        return Err(VaultCommandError::new("undo_unavailable"));
    };
    let result = run_review_core_sidecar(
        &app,
        "prepare_review_reversal",
        &ReversalPreparationInput {
            event_date: &event.event_date,
            event: &event,
        },
    )
    .await
    .map_err(VaultCommandError::from)?;
    let reversal = match result {
        ReviewCoreResult::Ready {
            event: ReviewCoreReadyEvent::Reversal(event),
        } => event,
        ReviewCoreResult::Ready {
            event: ReviewCoreReadyEvent::Relationship(_),
        }
        | ReviewCoreResult::Candidates { .. } => {
            return Err(VaultCommandError::new("review_core_failed"));
        }
        ReviewCoreResult::Review { reasons } => {
            let _ = reasons;
            return Err(VaultCommandError::new("review_core_failed"));
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        runtime.persist_review_reversal(&event_id, &reversal)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(|_| VaultCommandError::new("undo_unavailable"))?
    .ok_or_else(|| VaultCommandError::new("undo_unavailable"))
}
