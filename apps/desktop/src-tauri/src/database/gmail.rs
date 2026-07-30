use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GmailAccountStatus {
    Connected,
    Disconnected,
    PendingDelete,
    PendingSave,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct GmailAccountState {
    pub(crate) id: String,
    pub(crate) mailbox_address: String,
    pub(crate) secret_storage_key: String,
    pub(crate) status: GmailAccountStatus,
}

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "mailbox save and disconnect are wired by the next connector execution checkpoint"
    )
)]
impl ManualImportStore {
    pub(crate) fn gmail_account_state(
        &self,
        mailbox_address: &str,
    ) -> StoreResult<Option<GmailAccountState>> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, mailbox_address, secret_storage_key, connection_status \
                 FROM gmail_accounts WHERE mailbox_address = ?1",
                [mailbox_address],
                gmail_account_state_from_row,
            )
            .optional()?)
    }

    pub(crate) fn pending_gmail_account_states(&self) -> StoreResult<Vec<GmailAccountState>> {
        let mut statement = self.connection.prepare(
            "SELECT id, mailbox_address, secret_storage_key, connection_status \
             FROM gmail_accounts \
             WHERE connection_status IN ('pending_save', 'pending_delete') \
             ORDER BY id",
        )?;
        let rows = statement.query_map([], gmail_account_state_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn begin_gmail_account_save(
        &self,
        id: &str,
        mailbox_address: &str,
        secret_storage_key: &str,
    ) -> StoreResult<GmailAccountState> {
        Ok(self.connection.query_row(
            "INSERT INTO gmail_accounts( \
               id, mailbox_address, secret_storage_key, connection_status \
             ) VALUES (?1, ?2, ?3, 'pending_save') \
             ON CONFLICT(mailbox_address) DO UPDATE SET \
               connection_status = 'pending_save', \
               updated_at = CURRENT_TIMESTAMP \
             RETURNING id, mailbox_address, secret_storage_key, connection_status",
            params![id, mailbox_address, secret_storage_key],
            gmail_account_state_from_row,
        )?)
    }

    pub(crate) fn mark_gmail_account_connected(&self, id: &str) -> StoreResult<()> {
        update_gmail_account_status(&self.connection, id, "pending_save", "connected")
    }

    pub(crate) fn begin_gmail_account_delete(&self, id: &str) -> StoreResult<()> {
        update_gmail_account_status(&self.connection, id, "connected", "pending_delete")
    }

    pub(crate) fn mark_gmail_account_disconnected(
        &self,
        id: &str,
        expected_status: &str,
    ) -> StoreResult<()> {
        update_gmail_account_status(&self.connection, id, expected_status, "disconnected")
    }
}

fn gmail_account_state_from_row(row: &Row<'_>) -> rusqlite::Result<GmailAccountState> {
    let status = match row.get::<_, String>(3)?.as_str() {
        "connected" => GmailAccountStatus::Connected,
        "disconnected" => GmailAccountStatus::Disconnected,
        "pending_delete" => GmailAccountStatus::PendingDelete,
        "pending_save" => GmailAccountStatus::PendingSave,
        _ => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                io::Error::new(io::ErrorKind::InvalidData, "invalid Gmail account status").into(),
            ));
        }
    };
    Ok(GmailAccountState {
        id: row.get(0)?,
        mailbox_address: row.get(1)?,
        secret_storage_key: row.get(2)?,
        status,
    })
}

fn update_gmail_account_status(
    connection: &Connection,
    id: &str,
    expected_status: &str,
    next_status: &str,
) -> StoreResult<()> {
    let changed = connection.execute(
        "UPDATE gmail_accounts \
         SET connection_status = ?1, updated_at = CURRENT_TIMESTAMP \
         WHERE id = ?2 AND connection_status = ?3",
        params![next_status, id, expected_status],
    )?;
    if changed != 1 {
        return Err(
            io::Error::new(io::ErrorKind::InvalidData, "Gmail account state changed").into(),
        );
    }
    Ok(())
}
