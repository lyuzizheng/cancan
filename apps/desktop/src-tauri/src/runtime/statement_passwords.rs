//! Statement-password lifecycle: the confirmed-source save guard, the
//! presentation-safe document context, saving a verified password, removing
//! it, and reconciling the source rows that reference stored secrets. Split
//! out of `documents.rs` under the source-file size guardrail.

use super::*;

/// Presentation-safe context for the statement-password surface of one
/// evidence document.
///
/// It deliberately carries no secret and no source list: only the Money
/// Source trusted classification confirmed (or `None` while that is pending),
/// its current display name, and whether a verified password is already
/// stored for it. The renderer may offer `Save for this source` only when
/// `money_source_id` is present, and passes that same id back; the host
/// re-checks the document's owner before writing the Keychain.
#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct StatementPasswordUnlockContext {
    pub(crate) has_saved_password: bool,
    pub(crate) money_source_display_name: Option<String>,
    pub(crate) money_source_id: Option<String>,
}

impl VaultRuntime {
    /// Reads the confirmed owner and saved-password state for one document.
    ///
    /// The response is the bounded host contract the password prompt uses:
    /// no password value, no attempt result, and no list of other sources. A
    /// missing document is reported as `document_unavailable`; a document
    /// without a confirmed owner returns all-null/false.
    pub(crate) fn statement_password_document_context(
        &self,
        document_id: &str,
    ) -> Result<StatementPasswordUnlockContext, RuntimeError> {
        if document_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let owner = store
            .statement_password_document_owner(document_id)
            .map_store_error(
                store,
                "statement_password_document_owner",
                "document_unavailable",
            )?;
        Ok(StatementPasswordUnlockContext {
            has_saved_password: owner.has_saved_password,
            money_source_display_name: owner.display_name,
            money_source_id: owner.money_source_id,
        })
    }

    #[cfg(test)]
    pub(crate) fn save_statement_password(
        &self,
        money_source_id: &str,
        password: &[u8],
    ) -> Result<(), RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        if money_source_id.is_empty() {
            return Err(RuntimeError::new("invalid_source_request"));
        }
        if password.is_empty() {
            return Err(RuntimeError::new("statement_password_required"));
        }
        self.save_statement_password_in_store(store, money_source_id, password)
    }

    pub(super) fn save_statement_password_in_store(
        &self,
        store: &ManualImportStore,
        money_source_id: &str,
        password: &[u8],
    ) -> Result<(), RuntimeError> {
        self.reconcile_statement_passwords(store)?;
        let state = store
            .statement_password_state(money_source_id)
            .map_store_error(store, "statement_password_state", "invalid_source_request")?;
        match state {
            Some(state) if state.status == StatementPasswordStatus::Saved => {
                self.save_verified_statement_password(store, &state.secret_storage_key, password)
            }
            None => {
                let secret_storage_key = statement_password_storage_key(money_source_id);
                store
                    .begin_statement_password_save(money_source_id, &secret_storage_key)
                    .map_store_error(
                        store,
                        "begin_statement_password_save",
                        "invalid_source_request",
                    )?;
                self.save_verified_statement_password(store, &secret_storage_key, password)?;
                store
                    .mark_statement_password_saved(money_source_id)
                    .map_store_error(
                        store,
                        "mark_statement_password_saved",
                        "statement_password_save_failed",
                    )?;
                Ok(())
            }
            Some(_) => Err(RuntimeError::new("statement_password_state_invalid")),
        }
    }

    pub(super) fn save_verified_statement_password(
        &self,
        store: &ManualImportStore,
        secret_ref: &str,
        password: &[u8],
    ) -> Result<(), RuntimeError> {
        self.inner
            .statement_passwords
            .save(secret_ref, password)
            .map_err(|error| {
                store_failure(
                    store,
                    "statement_password_save",
                    "statement_password_save_failed",
                    &error,
                )
            })?;
        let saved = self
            .inner
            .statement_passwords
            .load(secret_ref)
            .map_err(|error| {
                store_failure(
                    store,
                    "statement_password_load",
                    "statement_password_save_failed",
                    &error,
                )
            })?;
        if saved.as_ref().map(|secret| secret.as_slice()) != Some(password) {
            return Err(RuntimeError::new("statement_password_save_failed"));
        }
        Ok(())
    }

    pub(crate) fn remove_statement_password(
        &self,
        money_source_id: &str,
    ) -> Result<(), RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        if money_source_id.is_empty() {
            return Err(RuntimeError::new("invalid_source_request"));
        }
        self.reconcile_statement_passwords(store)?;
        let state = store
            .statement_password_state(money_source_id)
            .map_store_error(store, "statement_password_state", "invalid_source_request")?;
        match state {
            None => Ok(()),
            Some(state) if state.status == StatementPasswordStatus::Saved => {
                store
                    .begin_statement_password_delete(money_source_id)
                    .map_store_error(
                        store,
                        "begin_statement_password_delete",
                        "statement_password_remove_failed",
                    )?;
                self.inner
                    .statement_passwords
                    .delete(&state.secret_storage_key)
                    .map_err(|error| {
                        store_failure(
                            store,
                            "statement_password_delete",
                            "statement_password_remove_failed",
                            &error,
                        )
                    })?;
                store
                    .remove_statement_password_ref(money_source_id, "pending_delete")
                    .map_store_error(
                        store,
                        "remove_statement_password_ref",
                        "statement_password_remove_failed",
                    )
            }
            Some(_) => Err(RuntimeError::new("statement_password_state_invalid")),
        }
    }

    pub(super) fn reconcile_statement_passwords(
        &self,
        store: &ManualImportStore,
    ) -> Result<(), RuntimeError> {
        let states = store.pending_statement_password_states().map_store_error(
            store,
            "pending_statement_password_states",
            "statement_password_state_invalid",
        )?;
        for state in states {
            match state.status {
                StatementPasswordStatus::PendingSave => {
                    self.inner
                        .statement_passwords
                        .delete(&state.secret_storage_key)
                        .map_err(|error| {
                            store_failure(
                                store,
                                "statement_password_delete",
                                "statement_password_save_failed",
                                &error,
                            )
                        })?;
                    store
                        .remove_statement_password_ref(&state.money_source_id, "pending_save")
                        .map_store_error(
                            store,
                            "remove_statement_password_ref",
                            "statement_password_save_failed",
                        )?;
                }
                StatementPasswordStatus::PendingDelete => {
                    self.inner
                        .statement_passwords
                        .delete(&state.secret_storage_key)
                        .map_err(|error| {
                            store_failure(
                                store,
                                "statement_password_delete",
                                "statement_password_remove_failed",
                                &error,
                            )
                        })?;
                    store
                        .remove_statement_password_ref(&state.money_source_id, "pending_delete")
                        .map_store_error(
                            store,
                            "remove_statement_password_ref",
                            "statement_password_remove_failed",
                        )?;
                }
                StatementPasswordStatus::Saved => {
                    return Err(RuntimeError::new("statement_password_state_invalid"));
                }
            }
        }
        Ok(())
    }
}

/// Returns the requested Money Source only when the trusted pipeline confirmed
/// it as the owner of `document_id`.
///
/// This is the save/replace gate, not the unlock gate: a session-only password
/// never asks for a Money Source. Because the check runs before any Keychain
/// read or write, every rejection leaves the previously saved secret exactly
/// as it was. `None` or an empty request means the caller has no confirmed
/// save target; a differing confirmed owner means the association the caller
/// rendered is stale or wrong.
pub(super) fn confirmed_statement_password_source<'a>(
    store: &ManualImportStore,
    document_id: &str,
    requested_money_source_id: Option<&'a str>,
) -> Result<&'a str, RuntimeError> {
    let requested = requested_money_source_id
        .filter(|money_source_id| !money_source_id.is_empty())
        .ok_or_else(|| RuntimeError::new("statement_password_source_unconfirmed"))?;
    let owner = store
        .statement_password_document_owner(document_id)
        .map_store_error(
            store,
            "statement_password_document_owner",
            "document_unavailable",
        )?;
    match owner.money_source_id.as_deref() {
        None => Err(RuntimeError::new("statement_password_source_unconfirmed")),
        Some(owner_id) if owner_id == requested => Ok(requested),
        Some(_) => Err(RuntimeError::new("statement_password_source_mismatch")),
    }
}
