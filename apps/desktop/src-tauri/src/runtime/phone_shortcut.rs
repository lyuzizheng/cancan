//! Phone capture: installing the Share-sheet Shortcut and proving the Inbox
//! folder it shares into is reachable.
//!
//! The artifact itself belongs to [`crate::phone_shortcut`]; this module owns
//! the three host commands around it and the device-local record of the last
//! destination check.
//!
//! * `phone_shortcut_status` — the version this build installs and how the last
//!   check ended. Reading it needs neither the Vault nor a live Inbox, so a
//!   setup row can render before either exists.
//! * `install_phone_shortcut` — writes the embedded artifact into the app's data
//!   directory, signs it with the local Shortcuts CLI, and hands the result to
//!   Shortcuts. Signing is preferred, not required: when it fails the unsigned
//!   artifact is opened instead, because Shortcuts imports either.
//! * `test_phone_shortcut_inbox` — the test action of the setup row. It runs
//!   under the Vault's Inbox scan guard and against the authorized Inbox, so it
//!   takes exactly the path a phone capture takes: the bookmark is resolved, the
//!   Inbox is written to, the bytes are read back, and the file the check made
//!   is removed again. No existing file is ever touched.

use super::*;
use crate::phone_shortcut::{self, PhoneShortcutInboxCheck};
#[cfg(target_os = "macos")]
use tokio::time::timeout;

/// Where the install action materializes the artifact inside the app's data
/// directory, before signing and handing it over.
const PHONE_SHORTCUT_DIRECTORY_NAME: &str = "phone-shortcut";
/// The recorded Inbox check. It sits beside the Vault like the
/// intake-notification status, because "the phone's folder was proven reachable"
/// is a device-local fact that has to read back before the Vault is unlocked.
const PHONE_SHORTCUT_CHECK_STATUS_FILE_NAME: &str = "phone-shortcut-check.status";
/// Signing is a local call into the Shortcuts CLI; the deadline only bounds a
/// hang, so it is generous.
#[cfg(target_os = "macos")]
const PHONE_SHORTCUT_SIGN_TIMEOUT: Duration = Duration::from_secs(60);

/// The phone-capture surface the renderer reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct PhoneShortcutStatus {
    /// The artifact version this build installs.
    pub(crate) version: u32,
    /// The name Shortcuts shows for it.
    pub(crate) name: String,
    pub(crate) inbox_check: PhoneShortcutInboxCheck,
}

#[tauri::command]
pub(crate) async fn phone_shortcut_status(
    runtime: State<'_, VaultRuntime>,
) -> Result<PhoneShortcutStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || Ok(runtime.phone_shortcut_status())).await
}

#[tauri::command]
pub(crate) async fn install_phone_shortcut(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<PhoneShortcutStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    install_phone_shortcut_artifact(&app, runtime).await
}

#[tauri::command]
pub(crate) async fn test_phone_shortcut_inbox(
    runtime: State<'_, VaultRuntime>,
) -> Result<PhoneShortcutStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.test_phone_shortcut_inbox()).await
}

async fn install_phone_shortcut_artifact(
    app: &AppHandle,
    runtime: VaultRuntime,
) -> Result<PhoneShortcutStatus, VaultCommandError> {
    // Shortcuts is the only importer of the artifact, so the install action is
    // macOS-only. Every other build says so instead of writing a copy that
    // nothing on the machine can open.
    if !cfg!(target_os = "macos") {
        return Err(RuntimeError::new("phone_shortcut_unsupported_platform").into());
    }
    let directory = runtime.phone_shortcut_artifact_directory();
    let materialize_runtime = runtime.clone();
    let artifact = run_runtime_task(move || {
        phone_shortcut::materialize_artifact(&directory).map_err(|error| {
            runtime_failure(
                &materialize_runtime,
                "phone_shortcut_materialize",
                "phone_shortcut_unavailable",
                &error,
            )
        })
    })
    .await?;
    let signed = sign_phone_shortcut(app, &artifact).await;
    let target = signed
        .as_deref()
        .unwrap_or(artifact.as_path())
        .to_str()
        .ok_or_else(|| RuntimeError::new("phone_shortcut_unavailable"))?
        .to_string();
    #[allow(
        deprecated,
        reason = "the same narrow shell opener the Gmail authorization flow and the notification settings use until the desktop shell adopts tauri-plugin-opener"
    )]
    app.shell().open(target, None).map_err(|error| {
        runtime_failure(
            &runtime,
            "phone_shortcut_open",
            "phone_shortcut_unavailable",
            &error,
        )
    })?;
    let status_runtime = runtime.clone();
    run_runtime_task(move || Ok(status_runtime.phone_shortcut_status())).await
}

impl VaultRuntime {
    pub(crate) fn phone_shortcut_status(&self) -> PhoneShortcutStatus {
        PhoneShortcutStatus {
            version: phone_shortcut::version(),
            name: phone_shortcut::name().to_string(),
            inbox_check: self.recorded_phone_shortcut_inbox_check(),
        }
    }

    pub(crate) fn phone_shortcut_artifact_directory(&self) -> PathBuf {
        self.inner.root.join(PHONE_SHORTCUT_DIRECTORY_NAME)
    }

    /// Runs the Inbox destination check and records its outcome.
    ///
    /// The check holds the same guard an Inbox scan does, so the sentinel can
    /// never be observed by a scan mid-flight, and it fails exactly the way a
    /// scan does when the Vault is locked, the Inbox was never configured, or
    /// its bookmark has to be authorized again.
    pub(crate) fn test_phone_shortcut_inbox(&self) -> Result<PhoneShortcutStatus, RuntimeError> {
        self.require_unlocked()?;
        let _scan_guard = self
            .inner
            .local_inbox_scan_guard
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?;
        let inbox = self.authorized_local_inbox()?;
        let check = match phone_shortcut::check_inbox_destination(&inbox) {
            Ok(()) => PhoneShortcutInboxCheck::passed(),
            Err(failure) => PhoneShortcutInboxCheck::failed(failure),
        };
        self.record_phone_shortcut_inbox_check(&check)?;
        Ok(PhoneShortcutStatus {
            version: phone_shortcut::version(),
            name: phone_shortcut::name().to_string(),
            inbox_check: check,
        })
    }

    /// The last recorded check, or `never_checked` when the record is absent,
    /// unreadable, or was written for another artifact version.
    fn recorded_phone_shortcut_inbox_check(&self) -> PhoneShortcutInboxCheck {
        fs::read(self.phone_shortcut_check_status_path())
            .ok()
            .and_then(|bytes| PhoneShortcutInboxCheck::decode(&bytes))
            .unwrap_or_else(PhoneShortcutInboxCheck::never_checked)
    }

    fn phone_shortcut_check_status_path(&self) -> PathBuf {
        self.inner.root.join(PHONE_SHORTCUT_CHECK_STATUS_FILE_NAME)
    }

    fn record_phone_shortcut_inbox_check(
        &self,
        check: &PhoneShortcutInboxCheck,
    ) -> Result<(), RuntimeError> {
        write_atomic(&self.phone_shortcut_check_status_path(), &check.encode()).map_err(|error| {
            runtime_failure(
                self,
                "phone_shortcut_check_status",
                "phone_shortcut_storage_failed",
                &error,
            )
        })
    }
}

/// Signs the materialized artifact beside itself and returns the signed path,
/// or `None` when signing did not produce a file — an unsigned artifact still
/// imports, so signing is a preference, not a requirement. A copy signed from
/// an older artifact is removed first: it must never be the file handed over.
#[cfg(target_os = "macos")]
async fn sign_phone_shortcut(app: &AppHandle, artifact: &Path) -> Option<PathBuf> {
    let signed = artifact.with_file_name(crate::phone_shortcut::PHONE_SHORTCUT_SIGNED_FILE_NAME);
    let _ = fs::remove_file(&signed);
    let (Some(source), Some(destination)) = (artifact.to_str(), signed.to_str()) else {
        return None;
    };
    let signing = app
        .shell()
        .command("shortcuts")
        .args(["sign", "-m", "anyone", "-i", source, "-o", destination])
        .output();
    match timeout(PHONE_SHORTCUT_SIGN_TIMEOUT, signing).await {
        Ok(Ok(output)) if output.status.success() && signed.is_file() => Some(signed),
        Ok(Ok(output)) => {
            eprintln!(
                "signing the phone Shortcut artifact failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            None
        }
        Ok(Err(error)) => {
            eprintln!("signing the phone Shortcut artifact failed: {error}");
            None
        }
        Err(_) => {
            eprintln!("signing the phone Shortcut artifact timed out");
            None
        }
    }
}

/// Signing is a Shortcuts CLI call that only exists on macOS.
#[cfg(not(target_os = "macos"))]
async fn sign_phone_shortcut(_app: &AppHandle, _artifact: &Path) -> Option<PathBuf> {
    None
}
