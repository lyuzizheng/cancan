use super::*;

impl VaultRuntime {
    pub(super) fn acknowledge_review_item(
        &self,
        review_item_id: &str,
        expected_record_version: i64,
    ) -> Result<ReviewMutationOutcome, RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .acknowledge_review_item(review_item_id, expected_record_version)
            .map_store_error(store, "acknowledge_review_item", "review_unavailable")
    }
}

#[tauri::command]
pub(crate) async fn acknowledge_review_item(
    review_item_id: String,
    expected_record_version: i64,
    runtime: State<'_, VaultRuntime>,
) -> Result<ReviewMutationOutcome, VaultCommandError> {
    if review_item_id.is_empty() || expected_record_version <= 0 {
        return Err(VaultCommandError::new("invalid_review_request"));
    }
    let runtime = runtime.inner().clone();
    run_runtime_task(move || {
        runtime.acknowledge_review_item(&review_item_id, expected_record_version)
    })
    .await
}
