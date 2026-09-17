use super::*;
use crate::diagnostics::{JobFailureDetail, OperationalLogEntry};
use std::cell::Cell;
use std::error::Error;

thread_local! {
    /// How many store guards this thread holds.
    ///
    /// [`runtime_failure`] takes the store lock to append its entry, so a call
    /// from a scope that already holds it would deadlock the whole app. This
    /// count turns that mistake into a loud failure in tests instead of a hang.
    static STORE_GUARDS_HELD: Cell<u32> = const { Cell::new(0) };
}

/// Marks this thread as holding the store guard until it is dropped.
pub(super) struct StoreGuardHeld;

impl StoreGuardHeld {
    pub(super) fn new() -> Self {
        STORE_GUARDS_HELD.with(|held| held.set(held.get() + 1));
        Self
    }
}

impl Drop for StoreGuardHeld {
    fn drop(&mut self) {
        STORE_GUARDS_HELD.with(|held| held.set(held.get().saturating_sub(1)));
    }
}

fn store_guard_is_held() -> bool {
    STORE_GUARDS_HELD.with(Cell::get) > 0
}

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
    /// The technical layer of the failure, kept so the caller that records the
    /// job row can write the real cause instead of the code alone.
    detail: Option<Box<JobFailureDetail>>,
    /// The component the failure came from, for the entry a scope that held the
    /// store guard writes once it has released it.
    reporter: Option<&'static str>,
}

impl RuntimeError {
    pub(super) fn new(code: &'static str) -> Self {
        Self {
            code,
            detail: None,
            reporter: None,
        }
    }

    /// Attaches the technical layer of the failure to the static code.
    pub(super) fn with_detail(self, detail: JobFailureDetail) -> Self {
        Self {
            detail: Some(Box::new(detail)),
            ..self
        }
    }

    /// The technical layer for this failure: the one the reporting component
    /// attached, or a code-only entry for a failure that was collapsed before
    /// anything could observe it.
    pub(super) fn detail_or_code(&self, provider: Option<&'static str>) -> JobFailureDetail {
        match self.detail.as_deref() {
            Some(detail) => detail.clone(),
            None => JobFailureDetail::from_code(provider, self.code),
        }
    }

    /// The attached technical layer, when the failure has one.
    pub(super) fn detail(&self) -> Option<&JobFailureDetail> {
        self.detail.as_deref()
    }

    /// Names the component a deferred log entry belongs to.
    fn reported_by(self, reporter: &'static str) -> Self {
        Self {
            reporter: Some(reporter),
            ..self
        }
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
    RuntimeError::new(code).with_detail(JobFailureDetail::from_error(None, error))
}

/// [`store_failure`] for a step that runs without a store handle in scope, for
/// example the encrypted file write that follows an intake capture.
///
/// Best-effort in the same way: the lock is taken only to append the entry, and
/// an unreachable Vault records nothing rather than replacing the original
/// failure with a logging failure.
///
/// Only call this where no store guard is held: the lock taken here is the same
/// one that guard holds. A scope that has a guard must call [`store_failure`]
/// with it instead, and a scope that holds the guard without a store instance
/// hands its detail to [`log_released_failure`] after releasing it.
pub(super) fn runtime_failure(
    runtime: &VaultRuntime,
    component: &'static str,
    code: &'static str,
    error: &(dyn Error + Send + Sync + 'static),
) -> RuntimeError {
    debug_assert!(
        !store_guard_is_held(),
        "runtime_failure({component}) would deadlock on this thread's store guard"
    );
    if let Ok(store) = runtime.store()
        && let Some(store) = store.as_ref()
    {
        let _ = store.record_operational_log(&OperationalLogEntry::failure(component, code, error));
    }
    RuntimeError::new(code).with_detail(JobFailureDetail::from_error(None, error))
}

/// [`runtime_failure`] for a scope that holds the store guard but has no store
/// instance to log through, such as creating a Vault that does not exist yet.
///
/// It attaches the detail without logging; the scope that owns the guard writes
/// the entry with [`log_released_failure`] once the guard is gone, so one
/// failure leaves exactly one entry.
pub(super) fn guard_held_failure(
    reporter: &'static str,
    code: &'static str,
    error: &(dyn Error + Send + Sync + 'static),
) -> RuntimeError {
    RuntimeError::new(code)
        .with_detail(JobFailureDetail::from_error(None, error))
        .reported_by(reporter)
}

/// Writes the entry a guard-holding scope could not write.
///
/// The caller must have released the store guard; nothing is logged for a
/// failure that reports no component or carries no technical layer, or when the
/// Vault is no longer reachable.
pub(super) fn log_released_failure(runtime: &VaultRuntime, error: &RuntimeError) {
    let (Some(reporter), Some(detail)) = (error.reporter, error.detail()) else {
        return;
    };
    debug_assert!(
        !store_guard_is_held(),
        "log_released_failure({reporter}) would deadlock on this thread's store guard"
    );
    if let Ok(store) = runtime.store()
        && let Some(store) = store.as_ref()
    {
        let _ = store.record_operational_log(&OperationalLogEntry::from_detail(
            reporter, error.code, detail,
        ));
    }
}

/// [`runtime_failure`] for a step whose owning job row records the failure —
/// code and detail together with the job, attempt, and duration. Nothing is
/// logged here, so one failure leaves one entry instead of two.
pub(super) fn deferred_failure(code: &'static str, detail: JobFailureDetail) -> RuntimeError {
    RuntimeError::new(code).with_detail(detail)
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
