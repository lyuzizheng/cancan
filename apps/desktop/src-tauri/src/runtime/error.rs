use super::*;

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
