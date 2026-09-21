use super::*;
use crate::database::intake::{
    ConfirmMoneySourceCandidateInput, ConfirmedMoneySourceCandidate, MoneySourceCandidateState,
    SourceConfirmationPrompt,
};

impl VaultRuntime {
    pub(crate) fn list_source_confirmation_prompts(
        &self,
    ) -> Result<Vec<SourceConfirmationPrompt>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store.list_source_confirmation_prompts().map_store_error(
            store,
            "list_source_confirmation_prompts",
            "source_confirmation_unavailable",
        )
    }

    pub(crate) fn confirm_source_candidate(
        &self,
        candidate_id: &str,
        expected_version: i64,
        display_name: &str,
        source_type: &str,
        target_money_source_id: Option<&str>,
    ) -> Result<ConfirmedMoneySourceCandidate, RuntimeError> {
        // The name follows the manual create/rename path: trimmed, then the
        // same non-empty/bounded/no-control-characters rule.
        let display_name = display_name.trim();
        if candidate_id.is_empty()
            || !sources::valid_money_source_display_name(display_name)
            || source_type.is_empty()
            || target_money_source_id.is_some_and(str::is_empty)
        {
            return Err(RuntimeError::new("invalid_source_confirmation_request"));
        }
        let audit_id = random_identifier("audit");
        let proposed_money_source_id = random_identifier("source");
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .confirm_money_source_candidate(&ConfirmMoneySourceCandidateInput {
                audit_id: &audit_id,
                candidate_id,
                display_name,
                expected_version,
                proposed_money_source_id: &proposed_money_source_id,
                source_type,
                target_money_source_id,
            })
            .map_err(|error| {
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|error| error.kind() == io::ErrorKind::NotFound)
                {
                    RuntimeError::new("source_not_found")
                } else {
                    store_failure(
                        store,
                        "confirm_money_source_candidate",
                        "source_confirmation_unavailable",
                        &*error,
                    )
                }
            })
    }

    pub(crate) fn park_source_candidate(
        &self,
        candidate_id: &str,
        expected_version: i64,
    ) -> Result<MoneySourceCandidateState, RuntimeError> {
        if candidate_id.is_empty() {
            return Err(RuntimeError::new("invalid_source_confirmation_request"));
        }
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .keep_money_source_candidate_unassigned(candidate_id, expected_version)
            .map_store_error(
                store,
                "keep_money_source_candidate_unassigned",
                "source_confirmation_unavailable",
            )
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct ConfirmSourceCandidateRequest {
    candidate_id: String,
    display_name: String,
    expected_version: i64,
    source_type: String,
    /// The existing Money Source the user picked; `null` creates a new source.
    target_money_source_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct ParkSourceCandidateRequest {
    candidate_id: String,
    expected_version: i64,
}

#[tauri::command]
pub(crate) async fn list_source_confirmation_prompts(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<SourceConfirmationPrompt>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_source_confirmation_prompts()).await
}

#[tauri::command]
pub(crate) async fn confirm_source_candidate(
    request: ConfirmSourceCandidateRequest,
    runtime: State<'_, VaultRuntime>,
) -> Result<ConfirmedMoneySourceCandidate, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || {
        runtime.confirm_source_candidate(
            &request.candidate_id,
            request.expected_version,
            &request.display_name,
            &request.source_type,
            request.target_money_source_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn park_source_candidate(
    request: ParkSourceCandidateRequest,
    runtime: State<'_, VaultRuntime>,
) -> Result<MoneySourceCandidateState, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || {
        runtime.park_source_candidate(&request.candidate_id, request.expected_version)
    })
    .await
}
