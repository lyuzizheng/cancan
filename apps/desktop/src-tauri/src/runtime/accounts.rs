use super::*;

#[tauri::command]
pub(crate) async fn list_money_sources(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<MoneySourceSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_money_sources()).await
}

#[tauri::command]
pub(crate) async fn list_account_confirmation_prompts(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<AccountConfirmationPrompt>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_account_confirmation_prompts()).await
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct DecideCandidateAccountsRequest {
    decisions: Vec<CandidateAccountDecisionInput>,
    money_source_id: String,
    proposal_version: String,
}

#[tauri::command]
pub(crate) async fn decide_candidate_accounts(
    request: DecideCandidateAccountsRequest,
    runtime: State<'_, VaultRuntime>,
) -> Result<AccountConfirmationOutcome, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || {
        runtime.decide_candidate_accounts(
            &request.money_source_id,
            &request.proposal_version,
            &request.decisions,
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn restore_dismissed_candidate_account(
    account_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<AccountConfirmationOutcome, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.restore_dismissed_candidate_account(&account_id)).await
}
