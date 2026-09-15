//! Dedicated Gmail mailbox persistence route (spec 0003).
//!
//! Test-only until the later onboarding checkpoint wires a renderer command:
//! no `#[tauri::command]` calls these methods yet.
use super::gmail_connector::run_gmail_connector_sidecar;
use super::*;
use crate::database::GmailAccountStatus;
use sha2::{Digest, Sha256};

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the local connector route is wired to the renderer by a later onboarding checkpoint"
    )
)]
impl VaultRuntime {
    #[cfg_attr(
        test,
        allow(
            dead_code,
            reason = "the local connector route is wired to the renderer by a later onboarding checkpoint"
        )
    )]
    pub(crate) async fn authorize_gmail_mailbox(
        &self,
        app: &AppHandle,
        client_id: &str,
    ) -> Result<String, RuntimeError> {
        if client_id.trim().is_empty() || client_id.len() > 4096 {
            return Err(RuntimeError::new("gmail_authorization_unavailable"));
        }
        {
            let store = self.store()?;
            if store.is_none() {
                return Err(RuntimeError::new("vault_locked"));
            }
        }
        run_gmail_connector_sidecar(app, self, client_id).await
    }

    pub(crate) fn persist_gmail_mailbox(
        &self,
        mailbox_address: &str,
        refresh_token: &[u8],
    ) -> Result<(), RuntimeError> {
        let mailbox_address = normalize_gmail_mailbox(mailbox_address)?;
        if refresh_token.is_empty() {
            return Err(RuntimeError::new("gmail_authorization_invalid"));
        }
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        self.reconcile_gmail_account(store, &mailbox_address)?;

        let digest = format!("{:x}", Sha256::digest(mailbox_address.as_bytes()));
        let state = store
            .begin_gmail_account_save(
                &format!("gmail-account:{digest}"),
                &mailbox_address,
                &format!("gmail-refresh-token:{digest}"),
            )
            .map_err(|_| RuntimeError::new("gmail_connection_save_failed"))?;
        self.save_verified_gmail_refresh_token(&state.secret_storage_key, refresh_token)?;
        store
            .mark_gmail_account_connected(&state.id)
            .map_err(|_| RuntimeError::new("gmail_connection_save_failed"))
    }

    pub(crate) fn disconnect_gmail_mailbox(
        &self,
        mailbox_address: &str,
    ) -> Result<(), RuntimeError> {
        let mailbox_address = normalize_gmail_mailbox(mailbox_address)?;
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        self.reconcile_gmail_account(store, &mailbox_address)?;
        let state = store
            .gmail_account_state(&mailbox_address)
            .map_err(|_| RuntimeError::new("gmail_connection_state_invalid"))?;
        let Some(state) = state else {
            return Ok(());
        };
        match state.status {
            GmailAccountStatus::Disconnected => Ok(()),
            GmailAccountStatus::Connected => {
                store
                    .begin_gmail_account_delete(&state.id)
                    .map_err(|_| RuntimeError::new("gmail_connection_remove_failed"))?;
                self.inner
                    .gmail_refresh_tokens
                    .delete(&state.secret_storage_key)
                    .map_err(|_| RuntimeError::new("gmail_connection_remove_failed"))?;
                store
                    .mark_gmail_account_disconnected(&state.id, "pending_delete")
                    .map_err(|_| RuntimeError::new("gmail_connection_remove_failed"))
            }
            GmailAccountStatus::PendingDelete | GmailAccountStatus::PendingSave => {
                Err(RuntimeError::new("gmail_connection_state_invalid"))
            }
        }
    }

    pub(super) fn reconcile_gmail_accounts_after_unlock(&self, store: &ManualImportStore) {
        let Ok(states) = store.pending_gmail_account_states() else {
            return;
        };
        for state in states {
            let _ = self.reconcile_gmail_account_state(store, &state);
        }
    }

    fn reconcile_gmail_account(
        &self,
        store: &ManualImportStore,
        mailbox_address: &str,
    ) -> Result<(), RuntimeError> {
        let state = store
            .gmail_account_state(mailbox_address)
            .map_err(|_| RuntimeError::new("gmail_connection_state_invalid"))?;
        match state {
            Some(state) => self.reconcile_gmail_account_state(store, &state),
            None => Ok(()),
        }
    }

    fn reconcile_gmail_account_state(
        &self,
        store: &ManualImportStore,
        state: &crate::database::GmailAccountState,
    ) -> Result<(), RuntimeError> {
        let (expected_status, error_code) = match state.status {
            GmailAccountStatus::PendingSave => ("pending_save", "gmail_connection_save_failed"),
            GmailAccountStatus::PendingDelete => {
                ("pending_delete", "gmail_connection_remove_failed")
            }
            GmailAccountStatus::Connected | GmailAccountStatus::Disconnected => return Ok(()),
        };
        self.inner
            .gmail_refresh_tokens
            .delete(&state.secret_storage_key)
            .map_err(|_| RuntimeError::new(error_code))?;
        store
            .mark_gmail_account_disconnected(&state.id, expected_status)
            .map_err(|_| RuntimeError::new(error_code))
    }

    fn save_verified_gmail_refresh_token(
        &self,
        secret_ref: &str,
        refresh_token: &[u8],
    ) -> Result<(), RuntimeError> {
        self.inner
            .gmail_refresh_tokens
            .save(secret_ref, refresh_token)
            .map_err(|_| RuntimeError::new("gmail_connection_save_failed"))?;
        let saved = self
            .inner
            .gmail_refresh_tokens
            .load(secret_ref)
            .map_err(|_| RuntimeError::new("gmail_connection_save_failed"))?;
        if saved.as_ref().map(|secret| secret.as_slice()) != Some(refresh_token) {
            return Err(RuntimeError::new("gmail_connection_save_failed"));
        }
        Ok(())
    }
}

fn normalize_gmail_mailbox(mailbox_address: &str) -> Result<String, RuntimeError> {
    let mailbox_address = mailbox_address.trim().to_lowercase();
    let Some((local, domain)) = mailbox_address.split_once('@') else {
        return Err(RuntimeError::new("gmail_authorization_invalid"));
    };
    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return Err(RuntimeError::new("gmail_authorization_invalid"));
    }
    Ok(mailbox_address)
}
