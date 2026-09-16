use super::*;
use crate::diagnostics::OperationalLogEntry;
use std::error::Error;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct VaultCommandError {
    pub(super) code: &'static str,
}

impl VaultCommandError {
    pub(super) fn new(code: &'static str) -> Self {
        Self { code }
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeError {
    pub(super) code: &'static str,
}

impl RuntimeError {
    pub(super) fn new(code: &'static str) -> Self {
        Self { code }
    }

    #[cfg(test)]
    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl From<RuntimeError> for VaultCommandError {
    fn from(error: RuntimeError) -> Self {
        Self::new(error.code)
    }
}

/// Records the technical detail of a failed store step before collapsing it
/// into the static code the renderer already receives (spec 0015).
///
/// The log write is best-effort: the caller gets the same static code whether
/// or not the Vault could record the detail, and the durable job row stays the
/// authoritative record for job failures.
pub(super) fn store_failure(
    store: &ManualImportStore,
    component: &'static str,
    code: &'static str,
    error: &(dyn Error + Send + Sync + 'static),
) -> RuntimeError {
    let _ = store.record_operational_log(&OperationalLogEntry::failure(component, code, error));
    RuntimeError::new(code)
}

/// [`store_failure`] for a step that runs without a store handle in scope, for
/// example the encrypted file write that follows an intake capture.
///
/// Best-effort in the same way: the lock is taken only to append the entry, and
/// an unreachable Vault records nothing rather than replacing the original
/// failure with a logging failure.
pub(super) fn runtime_failure(
    runtime: &VaultRuntime,
    component: &'static str,
    code: &'static str,
    error: &(dyn Error + Send + Sync + 'static),
) -> RuntimeError {
    if let Ok(store) = runtime.store()
        && let Some(store) = store.as_ref()
    {
        let _ = store.record_operational_log(&OperationalLogEntry::failure(component, code, error));
    }
    RuntimeError::new(code)
}

/// [`store_failure`] as a one-line combinator: every store step that collapses
/// its error into a static code logs the discarded detail on the way out.
pub(super) trait StoreResultExt<T> {
    fn map_store_error(
        self,
        store: &ManualImportStore,
        component: &'static str,
        code: &'static str,
    ) -> Result<T, RuntimeError>;
}

impl<T> StoreResultExt<T> for Result<T, Box<dyn Error + Send + Sync>> {
    fn map_store_error(
        self,
        store: &ManualImportStore,
        component: &'static str,
        code: &'static str,
    ) -> Result<T, RuntimeError> {
        self.map_err(|error| store_failure(store, component, code, &*error))
    }
}

pub(super) async fn run_runtime_task<T, F>(task: F) -> Result<T, VaultCommandError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, RuntimeError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| {
            eprintln!("runtime task join failed: {error}");
            VaultCommandError::new("runtime_unavailable")
        })?
        .map_err(Into::into)
}
