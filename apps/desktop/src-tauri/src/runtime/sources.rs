use super::*;

/// The Money Source providers this build lets a user configure.
///
/// A configured source has to carry the provider key its evidence will be
/// classified with (0014), so `create_money_source` accepts only a
/// product-defined provider instead of arbitrary text (0007). The entries
/// mirror the provider packages the host already accepts for normalization
/// (`runtime::sidecar`); a provider joins the catalog with its package, and
/// every other key fails closed.
const SUPPORTED_MONEY_SOURCE_PROVIDERS: &[SupportedMoneySourceProvider] = &[
    SupportedMoneySourceProvider {
        display_name: "DBS",
        provider_key: "dbs",
        source_type: "bank",
    },
    SupportedMoneySourceProvider {
        display_name: "HSBC",
        provider_key: "hsbc",
        source_type: "bank",
    },
];

struct SupportedMoneySourceProvider {
    display_name: &'static str,
    provider_key: &'static str,
    source_type: &'static str,
}

const MAX_MONEY_SOURCE_DISPLAY_NAME_BYTES: usize = 256;

fn supported_money_source_provider(
    provider_key: &str,
) -> Option<&'static SupportedMoneySourceProvider> {
    SUPPORTED_MONEY_SOURCE_PROVIDERS
        .iter()
        .find(|provider| provider.provider_key == provider_key)
}

impl VaultRuntime {
    /// Creates the one configured Money Source for a supported provider.
    ///
    /// The row persists into `money_sources` as a provider singleton, and the
    /// audit entry records which provider was configured. A provider that
    /// already has a source is reported instead of written: two provider
    /// singletons would make the classifier's routing ambiguous for that
    /// provider.
    pub(crate) fn create_money_source(
        &self,
        provider_key: &str,
        display_name: Option<&str>,
    ) -> Result<MoneySourceSummary, RuntimeError> {
        let Some(provider) = supported_money_source_provider(provider_key) else {
            return Err(RuntimeError::new("unsupported_source_provider"));
        };
        let display_name = match display_name {
            Some(display_name) => {
                let display_name = display_name.trim();
                if !valid_money_source_display_name(display_name) {
                    return Err(RuntimeError::new("invalid_source_request"));
                }
                display_name
            }
            None => provider.display_name,
        };
        let audit_id = random_identifier("audit");
        let money_source_id = random_identifier("source");
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let created = store
            .create_money_source(&CreateMoneySourceInput {
                audit_id: &audit_id,
                display_name,
                money_source_id: &money_source_id,
                provider_key: provider.provider_key,
                source_type: provider.source_type,
            })
            .map_err(|error| {
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|error| error.kind() == io::ErrorKind::AlreadyExists)
                {
                    RuntimeError::new("source_provider_already_configured")
                } else {
                    store_failure(
                        store,
                        "create_money_source",
                        "create_source_failed",
                        &*error,
                    )
                }
            })?;
        Ok(MoneySourceSummary {
            display_name: created.display_name,
            money_source_id: created.money_source_id,
            provider_key: created.provider_key,
            source_type: created.source_type,
        })
    }

    /// Renames a Money Source. Only `display_name` changes.
    pub(crate) fn edit_money_source(
        &self,
        money_source_id: &str,
        display_name: &str,
    ) -> Result<MoneySourceSummary, RuntimeError> {
        let display_name = display_name.trim();
        if money_source_id.is_empty() || !valid_money_source_display_name(display_name) {
            return Err(RuntimeError::new("invalid_source_request"));
        }
        let audit_id = random_identifier("audit");
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let renamed = store
            .rename_money_source(money_source_id, display_name, &audit_id)
            .map_err(|error| {
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|error| error.kind() == io::ErrorKind::NotFound)
                {
                    RuntimeError::new("source_not_found")
                } else {
                    store_failure(store, "rename_money_source", "edit_source_failed", &*error)
                }
            })?;
        Ok(MoneySourceSummary {
            display_name: renamed.display_name,
            money_source_id: renamed.money_source_id,
            provider_key: renamed.provider_key,
            source_type: renamed.source_type,
        })
    }

    /// The source-detail projection: the source's own metadata, its evidence
    /// documents with their user-facing status, and the actions the source
    /// currently offers.
    ///
    /// It returns no hash, locator, path, or stored secret — a document is
    /// described the same way the document list describes it, and the saved
    /// statement password is reported as a boolean.
    pub(crate) fn money_source_detail(
        &self,
        money_source_id: &str,
    ) -> Result<MoneySourceDetail, RuntimeError> {
        if money_source_id.is_empty() {
            return Err(RuntimeError::new("invalid_source_request"));
        }
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let source = store
            .money_source(money_source_id)
            .map_store_error(store, "money_source", "source_detail_failed")?
            .ok_or_else(|| RuntimeError::new("source_not_found"))?;
        let documents = store.list_documents(money_source_id).map_store_error(
            store,
            "list_documents",
            "list_documents_failed",
        )?;
        let documents = Self::source_document_summaries(store, documents)?;
        let has_saved_statement_password = store
            .has_saved_statement_password(money_source_id)
            .map_store_error(
                store,
                "has_saved_statement_password",
                "source_detail_failed",
            )?;
        let can_enter_statement_password = documents
            .iter()
            .any(|document| document.attention_reason.as_deref() == Some("password_required"));
        Ok(MoneySourceDetail {
            actions: MoneySourceDetailActions {
                can_enter_statement_password,
                has_saved_statement_password,
            },
            display_name: source.display_name,
            documents,
            money_source_id: source.money_source_id,
            provider_key: source.provider_key,
            source_type: source.source_type,
        })
    }

    /// The providers this build lets a user configure, each with the source
    /// already configured for it.
    ///
    /// The picker gets its provider identity and presentation from exactly one
    /// place - this catalog - instead of a renderer-side table that drifts on
    /// the next provider. `configured_money_source_id` applies the same
    /// provider-singleton rule `create_money_source` gates on, so every
    /// provider the picker offers is one the create command still accepts, and
    /// a configured provider can be opened directly instead of only grayed out.
    pub(crate) fn list_supported_money_source_providers(
        &self,
    ) -> Result<Vec<SupportedMoneySourceProviderSummary>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let configured = store.provider_singletons().map_store_error(
            store,
            "provider_singletons",
            "list_sources_failed",
        )?;
        Ok(SUPPORTED_MONEY_SOURCE_PROVIDERS
            .iter()
            .map(|provider| SupportedMoneySourceProviderSummary {
                configured_money_source_id: configured.get(provider.provider_key).cloned(),
                display_name: provider.display_name.to_owned(),
                provider_key: provider.provider_key.to_owned(),
                source_type: provider.source_type.to_owned(),
            })
            .collect())
    }
}

fn valid_money_source_display_name(display_name: &str) -> bool {
    !display_name.is_empty()
        && display_name.len() <= MAX_MONEY_SOURCE_DISPLAY_NAME_BYTES
        && !display_name.chars().any(char::is_control)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct CreateMoneySourceRequest {
    /// The user's label for the source. Absent means the supported provider's
    /// own product name.
    display_name: Option<String>,
    provider_key: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct EditMoneySourceRequest {
    display_name: String,
    money_source_id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct MoneySourceDetail {
    actions: MoneySourceDetailActions,
    display_name: String,
    documents: Vec<SourceDocumentSummary>,
    money_source_id: String,
    /// The provider this source is bound to, so the detail surface can render
    /// provider-specific modules without trusting a user-editable name.
    provider_key: String,
    source_type: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct MoneySourceDetailActions {
    /// At least one document waits on a statement password only the user can
    /// supply, so the source offers its password surface for entry.
    can_enter_statement_password: bool,
    /// A statement password is stored for this source, so its password surface
    /// offers update/remove instead of save.
    has_saved_statement_password: bool,
}

/// One provider a user can configure, with the source configured for it.
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct SupportedMoneySourceProviderSummary {
    /// The source already configured for this provider, when one exists.
    /// Nullable rather than a flag, so the picker can open that source instead
    /// of only rendering the provider as taken.
    configured_money_source_id: Option<String>,
    display_name: String,
    provider_key: String,
    source_type: String,
}

#[tauri::command]
pub(crate) async fn create_money_source(
    request: CreateMoneySourceRequest,
    runtime: State<'_, VaultRuntime>,
) -> Result<MoneySourceSummary, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || {
        runtime.create_money_source(&request.provider_key, request.display_name.as_deref())
    })
    .await
}

#[tauri::command]
pub(crate) async fn edit_money_source(
    request: EditMoneySourceRequest,
    runtime: State<'_, VaultRuntime>,
) -> Result<MoneySourceSummary, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || {
        runtime.edit_money_source(&request.money_source_id, &request.display_name)
    })
    .await
}

#[tauri::command]
pub(crate) async fn get_money_source_detail(
    money_source_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<MoneySourceDetail, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.money_source_detail(&money_source_id)).await
}

#[tauri::command]
pub(crate) async fn list_supported_money_source_providers(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<SupportedMoneySourceProviderSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_supported_money_source_providers()).await
}
