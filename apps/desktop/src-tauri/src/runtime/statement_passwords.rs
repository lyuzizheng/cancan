//! Statement-password lifecycle: saving a verified password, removing it,
//! and reconciling the source rows that reference stored secrets. Split out
//! of `documents.rs` under the source-file size guardrail.

use super::*;

impl VaultRuntime {
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
