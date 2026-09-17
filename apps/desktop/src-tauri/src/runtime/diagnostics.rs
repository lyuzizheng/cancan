//! The user-initiated diagnostic export.
//!
//! Spec 0015 keeps the operational log local and never sends it anywhere. The
//! only way it leaves the Vault is this pair of commands: the preview states
//! what the file would contain — categories, counts, retention, and the same
//! redacted sample lines — and the export writes exactly that, so the user
//! approves what they are about to share instead of discovering it afterwards.
//! Redaction itself belongs to [`crate::diagnostics`]; nothing here re-encodes
//! an entry.

use super::*;
use crate::diagnostics::OperationalDiagnosticsPreview;

impl VaultRuntime {
    /// What an export would contain. Read-only: it never writes the file.
    pub(super) fn operational_diagnostics_preview(
        &self,
    ) -> Result<OperationalDiagnosticsPreview, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store.operational_diagnostics_preview().map_store_error(
            store,
            "operational_diagnostics_preview",
            "diagnostics_unavailable",
        )
    }

    /// Writes the redacted export the preview described to a user-chosen path.
    pub(super) fn save_operational_diagnostics(
        &self,
        destination: &Path,
    ) -> Result<(), RuntimeError> {
        let export = {
            let store = self.store()?;
            let store = store
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            store.operational_diagnostics_export().map_store_error(
                store,
                "operational_diagnostics_export",
                "diagnostics_unavailable",
            )?
        };
        fs::write(destination, export.as_bytes()).map_err(|error| {
            runtime_failure(
                self,
                "operational_diagnostics_export",
                "diagnostics_export_failed",
                &error,
            )
        })
    }
}

/// The categories and counts an export would contain, with the redacted sample
/// lines the file itself would carry.
#[tauri::command]
pub(crate) async fn operational_diagnostics_preview(
    runtime: State<'_, VaultRuntime>,
) -> Result<OperationalDiagnosticsPreview, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.operational_diagnostics_preview()).await
}

/// Writes the redacted diagnostic export. `false` means the user dismissed the
/// save dialog, so no file was written.
#[tauri::command]
pub(crate) async fn save_operational_diagnostics(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<bool, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || -> Result<bool, RuntimeError> {
        runtime.require_unlocked()?;
        let selected = app
            .dialog()
            .file()
            .set_title("Save CanCan diagnostics")
            .set_file_name("CanCan Diagnostics.txt")
            .add_filter("Text file", &["txt"])
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(false);
        };
        let destination = selected
            .into_path()
            .map_err(|_| RuntimeError::new("file_selection_failed"))?;
        runtime.save_operational_diagnostics(&destination)?;
        Ok(true)
    })
    .await
}
