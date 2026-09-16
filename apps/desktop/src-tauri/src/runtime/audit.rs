use super::*;

impl VaultRuntime {
    pub(crate) fn audit_duplicate_committed_versions(
        &self,
    ) -> Result<Vec<DuplicateCommittedVersionAuditRow>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .audit_duplicate_committed_versions()
            .map_err(|_| RuntimeError::new("audit_unavailable"))
    }
}

#[tauri::command]
pub(crate) async fn audit_duplicate_committed_versions(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<DuplicateCommittedVersionAuditRow>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.audit_duplicate_committed_versions()).await
}
