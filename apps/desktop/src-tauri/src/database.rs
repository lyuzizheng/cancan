use crate::vault::{FileVault, StoredFile};
use hkdf::Hkdf;
use rand::{RngCore, rngs::OsRng};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, params};
use serde::Serialize;
use sha2::Sha256;
use std::{collections::HashSet, error::Error, fs, io, path::Path};
use zeroize::Zeroizing;

const KEY_LEN: usize = 32;
pub(crate) const DATABASE_FILE_NAME: &str = "finance.sqlite";
const DATABASE_KEY_CONTEXT: &[u8] = b"cancan:database:v1";
struct Migration {
    version: i64,
    sql: &'static str,
    foreign_keys_off: bool,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../../../../packages/db/migrations/0001_synthetic_core.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 2,
        sql: include_str!("../../../../packages/db/migrations/0002_vault_manual_import.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 3,
        sql: include_str!(
            "../../../../packages/db/migrations/0003_source_document_pending_identity.sql"
        ),
        foreign_keys_off: true,
    },
    Migration {
        version: 4,
        sql: include_str!(
            "../../../../packages/db/migrations/0004_source_document_pending_source.sql"
        ),
        foreign_keys_off: true,
    },
    Migration {
        version: 5,
        sql: include_str!(
            "../../../../packages/db/migrations/0005_money_source_statement_password.sql"
        ),
        foreign_keys_off: false,
    },
];

type StoreResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDocumentImportStatus {
    Imported,
    AlreadyPresent,
    Restored,
    RestoreConfirmationRequired,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDocumentImportOutcome {
    pub document_id: String,
    pub status: SourceDocumentImportStatus,
}

#[derive(Debug)]
pub struct SourceDocumentImport<'a> {
    pub audit_actor: &'a str,
    pub audit_id: &'a str,
    pub audit_policy_version: &'a str,
    pub audit_reason: &'a str,
    pub document_id: &'a str,
    pub mime_type: &'a str,
    pub original_filename: &'a str,
    pub source_path: &'a Path,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SourceDocumentView {
    pub byte_size: u64,
    pub document_id: String,
    pub encrypted_locator: Option<String>,
    pub file_sha256: String,
    pub file_state: String,
    pub mime_type: String,
    pub money_source_id: Option<String>,
    pub original_filename: String,
    pub received_at: String,
    pub semantic_document_key: Option<String>,
}

pub struct SourceDocumentFileInput {
    pub file_sha256: String,
    pub mime_type: String,
    pub plaintext: Zeroizing<Vec<u8>>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct MoneySourceView {
    pub(crate) display_name: String,
    pub(crate) money_source_id: String,
    pub(crate) source_type: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatementPasswordStatus {
    PendingDelete,
    PendingSave,
    Saved,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StatementPasswordState {
    pub(crate) money_source_id: String,
    pub(crate) secret_storage_key: String,
    pub(crate) status: StatementPasswordStatus,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StatementPasswordSource {
    pub(crate) display_name: String,
    pub(crate) has_saved_password: bool,
    pub(crate) money_source_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDocumentRoutingStatus {
    Routed,
    NeedsAttention,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDocumentRoutingOutcome {
    pub account_ids: Vec<String>,
    pub document_id: String,
    pub money_source_id: Option<String>,
    pub reason: Option<&'static str>,
    pub status: SourceDocumentRoutingStatus,
}

impl SourceDocumentRoutingOutcome {
    pub(crate) fn needs_attention(document_id: &str, reason: &'static str) -> Self {
        Self {
            account_ids: Vec::new(),
            document_id: document_id.to_owned(),
            money_source_id: None,
            reason: Some(reason),
            status: SourceDocumentRoutingStatus::NeedsAttention,
        }
    }
}

#[derive(Debug)]
pub struct TrustedAccountCandidate<'a> {
    pub account_id: &'a str,
    pub account_type: &'a str,
    pub currency: Option<&'a str>,
    pub display_name: &'a str,
    pub masked_identifier: Option<&'a str>,
    pub provider_account_id: Option<&'a str>,
}

#[derive(Debug)]
pub struct TrustedDocumentClassification<'a> {
    pub accounts: &'a [TrustedAccountCandidate<'a>],
    pub audit_id: &'a str,
    pub document_id: &'a str,
    pub provider_key: &'a str,
    pub semantic_document_key: &'a str,
}

#[derive(Debug)]
struct ExistingDocument {
    document_id: String,
    encrypted_locator: Option<String>,
    file_sha256: String,
    file_state: String,
}

pub struct ManualImportStore {
    connection: Connection,
    files: FileVault,
    master_key: Zeroizing<[u8; KEY_LEN]>,
}

impl ManualImportStore {
    pub fn open(root: &Path, master_key: Zeroizing<[u8; KEY_LEN]>) -> StoreResult<Self> {
        fs::create_dir_all(root)?;
        Self::open_with_flags(
            root,
            master_key,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
    }

    pub fn open_existing(root: &Path, master_key: Zeroizing<[u8; KEY_LEN]>) -> StoreResult<Self> {
        Self::open_with_flags(root, master_key, OpenFlags::SQLITE_OPEN_READ_WRITE)
    }

    fn open_with_flags(
        root: &Path,
        master_key: Zeroizing<[u8; KEY_LEN]>,
        flags: OpenFlags,
    ) -> StoreResult<Self> {
        let mut connection =
            open_encrypted_database(&root.join(DATABASE_FILE_NAME), &master_key, flags)?;
        apply_migrations(&mut connection)?;
        let mut store = Self {
            connection,
            files: FileVault::new(root),
            master_key,
        };
        store.reconcile_files()?;
        Ok(store)
    }

    pub(crate) fn master_key(&self) -> &[u8; KEY_LEN] {
        &self.master_key
    }

    pub fn register_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        validate_import(input)?;
        let source = FileVault::prepare(input.source_path)?;
        let existing = find_exact_document(&self.connection, source.file_sha256())?;
        match (existing.as_ref(), restore_deleted_document_id) {
            (Some(existing), None) if existing.file_state == "deleted" => {
                return Ok(SourceDocumentImportOutcome {
                    document_id: existing.document_id.clone(),
                    status: SourceDocumentImportStatus::RestoreConfirmationRequired,
                });
            }
            (Some(existing), Some(expected_document_id))
                if existing.file_state == "deleted"
                    && existing.document_id == expected_document_id => {}
            (_, Some(_)) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "selected source no longer matches the deleted document",
                )
                .into());
            }
            _ => {}
        }
        let replace_existing = existing
            .as_ref()
            .is_some_and(|document| document.file_state != "available");
        let stored = match self
            .files
            .store_prepared(&self.master_key, &source, replace_existing)
        {
            Ok(stored) => stored,
            Err(error)
                if existing
                    .as_ref()
                    .is_some_and(|document| document.file_state == "available")
                    && matches!(
                        error.kind(),
                        io::ErrorKind::InvalidData | io::ErrorKind::NotFound
                    ) =>
            {
                let document = existing.as_ref().expect("guarded existing document");
                mark_missing(&mut self.connection, document, source.file_sha256())?;
                self.files.store_prepared(&self.master_key, &source, true)?
            }
            Err(error) => return Err(error.into()),
        };

        match persist_import(&mut self.connection, input, &stored) {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                if stored.created {
                    let _ = self.files.remove(&stored.encrypted_locator);
                }
                Err(error.into())
            }
        }
    }

    pub fn delete_source_document(&mut self, document_id: &str, audit_id: &str) -> StoreResult<()> {
        if document_id.is_empty() || audit_id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "source deletion identifiers must not be empty",
            )
            .into());
        }
        let document = find_document_by_id(&self.connection, document_id)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "source document not found"))?;
        if document.file_state != "available" {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source document file is unavailable",
            )
            .into());
        }
        let locator = document.encrypted_locator.as_deref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "available source document has no encrypted locator",
            )
        })?;
        if !self
            .files
            .verifies(&self.master_key, locator, &document.file_sha256)?
        {
            mark_missing(&mut self.connection, &document, &document.file_sha256)?;
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source document file is unavailable",
            )
            .into());
        }
        persist_source_deletion(&mut self.connection, &document, audit_id)?;
        self.files.remove(locator)?;
        Ok(())
    }

    pub fn list_documents(&self, money_source_id: &str) -> StoreResult<Vec<SourceDocumentView>> {
        let mut statement = self.connection.prepare(
            "SELECT id, money_source_id, file_sha256, semantic_document_key, original_filename, \
                    mime_type, byte_size, encrypted_locator, file_state, received_at \
             FROM source_documents \
             WHERE money_source_id = ?1 \
             ORDER BY received_at DESC, id",
        )?;
        let rows = statement.query_map([money_source_id], |row| {
            let byte_size: i64 = row.get(6)?;
            Ok(SourceDocumentView {
                byte_size: u64::try_from(byte_size).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Integer,
                        Box::new(error),
                    )
                })?,
                document_id: row.get(0)?,
                money_source_id: row.get(1)?,
                file_sha256: row.get(2)?,
                semantic_document_key: row.get(3)?,
                original_filename: row.get(4)?,
                mime_type: row.get(5)?,
                encrypted_locator: row.get(7)?,
                file_state: row.get(8)?,
                received_at: row.get(9)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_unassigned_documents(&self) -> StoreResult<Vec<SourceDocumentView>> {
        let mut statement = self.connection.prepare(
            "SELECT id, money_source_id, file_sha256, semantic_document_key, original_filename, \
                    mime_type, byte_size, encrypted_locator, file_state, received_at \
             FROM source_documents \
             WHERE money_source_id IS NULL \
             ORDER BY received_at DESC, id",
        )?;
        let rows = statement.query_map([], source_document_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn list_money_sources(&self) -> StoreResult<Vec<MoneySourceView>> {
        let mut statement = self.connection.prepare(
            "SELECT id, display_name, source_type FROM money_sources \
             ORDER BY display_name, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(MoneySourceView {
                money_source_id: row.get(0)?,
                display_name: row.get(1)?,
                source_type: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn source_document_input(&self, document_id: &str) -> StoreResult<SourceDocumentFileInput> {
        let document = self
            .connection
            .query_row(
                "SELECT file_sha256, mime_type, encrypted_locator, file_state \
                 FROM source_documents WHERE id = ?1",
                [document_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((file_sha256, mime_type, encrypted_locator, file_state)) = document else {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        };
        if file_state != "available" {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source document file is unavailable",
            )
            .into());
        }
        let encrypted_locator = encrypted_locator.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "available source document has no encrypted locator",
            )
        })?;
        let plaintext = self
            .files
            .open_in_memory(&self.master_key, &encrypted_locator)?;
        Ok(SourceDocumentFileInput {
            file_sha256,
            mime_type,
            plaintext,
        })
    }

    pub fn source_document_mime_type(&self, document_id: &str) -> StoreResult<String> {
        let document = self
            .connection
            .query_row(
                "SELECT mime_type, file_state FROM source_documents WHERE id = ?1",
                [document_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((mime_type, file_state)) = document else {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        };
        if file_state != "available" {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source document file is unavailable",
            )
            .into());
        }
        Ok(mime_type)
    }

    pub(crate) fn statement_password_state(
        &self,
        money_source_id: &str,
    ) -> StoreResult<Option<StatementPasswordState>> {
        let state = self
            .connection
            .query_row(
                "SELECT money_source_id, secret_storage_key, status \
                 FROM statement_secret_refs WHERE money_source_id = ?1",
                [money_source_id],
                statement_password_state_from_row,
            )
            .optional()?;
        Ok(state)
    }

    pub(crate) fn statement_password_sources(&self) -> StoreResult<Vec<StatementPasswordSource>> {
        let mut statement = self.connection.prepare(
            "SELECT money_sources.id, money_sources.display_name, \
                    EXISTS( \
                      SELECT 1 FROM statement_secret_refs \
                      WHERE statement_secret_refs.money_source_id = money_sources.id \
                        AND statement_secret_refs.status = 'saved' \
                    ) \
             FROM money_sources \
             ORDER BY money_sources.display_name, money_sources.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(StatementPasswordSource {
                money_source_id: row.get(0)?,
                display_name: row.get(1)?,
                has_saved_password: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn pending_statement_password_states(
        &self,
    ) -> StoreResult<Vec<StatementPasswordState>> {
        let mut statement = self.connection.prepare(
            "SELECT money_source_id, secret_storage_key, status \
             FROM statement_secret_refs WHERE status <> 'saved' ORDER BY money_source_id",
        )?;
        let rows = statement.query_map([], statement_password_state_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn begin_statement_password_save(
        &self,
        money_source_id: &str,
        secret_storage_key: &str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO statement_secret_refs( \
               id, money_source_id, secret_storage_key, status, hint_label \
             ) VALUES (?1, ?2, ?3, 'pending_save', NULL)",
            params![
                format!("statement-secret-ref:{money_source_id}"),
                money_source_id,
                secret_storage_key
            ],
        )?;
        Ok(())
    }

    pub(crate) fn mark_statement_password_saved(&self, money_source_id: &str) -> StoreResult<()> {
        update_statement_password_status(&self.connection, money_source_id, "pending_save", "saved")
    }

    pub(crate) fn begin_statement_password_delete(&self, money_source_id: &str) -> StoreResult<()> {
        update_statement_password_status(
            &self.connection,
            money_source_id,
            "saved",
            "pending_delete",
        )
    }

    pub(crate) fn remove_statement_password_ref(
        &self,
        money_source_id: &str,
        expected_status: &str,
    ) -> StoreResult<()> {
        let changed = self.connection.execute(
            "DELETE FROM statement_secret_refs WHERE money_source_id = ?1 AND status = ?2",
            params![money_source_id, expected_status],
        )?;
        if changed != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "statement password state changed",
            )
            .into());
        }
        Ok(())
    }

    pub fn apply_trusted_classification(
        &mut self,
        input: &TrustedDocumentClassification<'_>,
    ) -> StoreResult<SourceDocumentRoutingOutcome> {
        validate_classification(input)?;
        let transaction = self.connection.transaction()?;
        let existing_identity = transaction
            .query_row(
                "SELECT money_source_id, semantic_document_key \
                 FROM source_documents WHERE id = ?1",
                [input.document_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?;
        let Some((assigned_source_id, semantic_document_key)) = existing_identity else {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        };

        let mut source_statement = transaction
            .prepare("SELECT id FROM money_sources WHERE provider_key = ?1 ORDER BY id LIMIT 2")?;
        let source_ids = source_statement
            .query_map([input.provider_key], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(source_statement);
        if source_ids.len() != 1 {
            return Ok(needs_attention(
                input.document_id,
                if source_ids.is_empty() {
                    "money_source_not_found"
                } else {
                    "money_source_ambiguous"
                },
            ));
        }
        let money_source_id = &source_ids[0];
        if assigned_source_id
            .as_deref()
            .is_some_and(|id| id != money_source_id)
            || semantic_document_key
                .as_deref()
                .is_some_and(|key| key != input.semantic_document_key)
        {
            return Ok(needs_attention(
                input.document_id,
                "classification_conflict",
            ));
        }

        let mut account_ids = Vec::with_capacity(input.accounts.len());
        for account in input.accounts {
            let Some(provider_account_id) = account.provider_account_id else {
                return Ok(needs_attention(input.document_id, "account_mapping_needed"));
            };
            let existing = transaction
                .query_row(
                    "SELECT id, status FROM accounts \
                     WHERE money_source_id = ?1 AND provider_key = ?2 \
                       AND provider_account_id = ?3 AND status <> 'merged'",
                    params![money_source_id, input.provider_key, provider_account_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            match existing {
                Some((_, status)) if status == "archived" => {
                    return Ok(needs_attention(
                        input.document_id,
                        "account_restore_required",
                    ));
                }
                Some((account_id, _)) => account_ids.push(account_id),
                None => {
                    transaction.execute(
                        "INSERT INTO accounts( \
                           id, money_source_id, provider_key, provider_account_id, account_type, \
                           display_name, masked_identifier, currency, status \
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'candidate')",
                        params![
                            account.account_id,
                            money_source_id,
                            input.provider_key,
                            provider_account_id,
                            account.account_type,
                            account.display_name,
                            account.masked_identifier,
                            account.currency,
                        ],
                    )?;
                    account_ids.push(account.account_id.to_owned());
                }
            }
        }

        transaction.execute(
            "UPDATE source_documents \
             SET money_source_id = ?1, semantic_document_key = ?2 \
             WHERE id = ?3",
            params![
                money_source_id,
                input.semantic_document_key,
                input.document_id
            ],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES (?1, 'source_document', ?2, 'trusted_classification_applied', \
                       'system', 'provider_and_account_verified', 'classification-v1')",
            params![input.audit_id, input.document_id],
        )?;
        transaction.commit()?;
        Ok(SourceDocumentRoutingOutcome {
            account_ids,
            document_id: input.document_id.to_owned(),
            money_source_id: Some(money_source_id.to_owned()),
            reason: None,
            status: SourceDocumentRoutingStatus::Routed,
        })
    }

    #[cfg(test)]
    pub(crate) fn seed_money_source(
        &self,
        id: &str,
        provider_key: &str,
        display_name: &str,
        source_type: &str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES (?1, ?2, ?3, ?4)",
            params![id, provider_key, display_name, source_type],
        )?;
        Ok(())
    }

    fn reconcile_files(&mut self) -> StoreResult<()> {
        let documents = {
            let mut statement = self.connection.prepare(
                "SELECT id, file_sha256, encrypted_locator, file_state \
                 FROM source_documents \
                 WHERE encrypted_locator IS NOT NULL",
            )?;
            let rows = statement.query_map([], |row| {
                Ok(ExistingDocument {
                    document_id: row.get(0)?,
                    encrypted_locator: row.get(2)?,
                    file_sha256: row.get(1)?,
                    file_state: row.get(3)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let mut referenced = HashSet::new();
        for document in documents {
            let Some(locator) = document.encrypted_locator.as_deref() else {
                continue;
            };
            referenced.insert(locator.to_owned());
            if document.file_state == "available"
                && !self
                    .files
                    .verifies(&self.master_key, locator, &document.file_sha256)?
            {
                mark_missing(&mut self.connection, &document, &document.file_sha256)?;
            }
        }
        self.files.remove_unreferenced(&referenced)?;
        Ok(())
    }
}

fn statement_password_state_from_row(row: &Row<'_>) -> rusqlite::Result<StatementPasswordState> {
    let status = match row.get::<_, String>(2)?.as_str() {
        "pending_delete" => StatementPasswordStatus::PendingDelete,
        "pending_save" => StatementPasswordStatus::PendingSave,
        "saved" => StatementPasswordStatus::Saved,
        _ => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid statement password status",
                )
                .into(),
            ));
        }
    };
    Ok(StatementPasswordState {
        money_source_id: row.get(0)?,
        secret_storage_key: row.get(1)?,
        status,
    })
}

fn update_statement_password_status(
    connection: &Connection,
    money_source_id: &str,
    expected_status: &str,
    next_status: &str,
) -> StoreResult<()> {
    let changed = connection.execute(
        "UPDATE statement_secret_refs SET status = ?1, updated_at = CURRENT_TIMESTAMP \
         WHERE money_source_id = ?2 AND status = ?3",
        params![next_status, money_source_id, expected_status],
    )?;
    if changed != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "statement password state changed",
        )
        .into());
    }
    Ok(())
}

fn source_document_from_row(row: &Row<'_>) -> rusqlite::Result<SourceDocumentView> {
    let byte_size: i64 = row.get(6)?;
    Ok(SourceDocumentView {
        byte_size: u64::try_from(byte_size).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Integer,
                Box::new(error),
            )
        })?,
        document_id: row.get(0)?,
        money_source_id: row.get(1)?,
        file_sha256: row.get(2)?,
        semantic_document_key: row.get(3)?,
        original_filename: row.get(4)?,
        mime_type: row.get(5)?,
        encrypted_locator: row.get(7)?,
        file_state: row.get(8)?,
        received_at: row.get(9)?,
    })
}

fn needs_attention(document_id: &str, reason: &'static str) -> SourceDocumentRoutingOutcome {
    SourceDocumentRoutingOutcome::needs_attention(document_id, reason)
}

fn open_encrypted_database(
    path: &Path,
    master_key: &[u8; KEY_LEN],
    flags: OpenFlags,
) -> StoreResult<Connection> {
    let connection = Connection::open_with_flags(path, flags)?;
    let database_key = derive_database_key(master_key)?;
    let raw_key = hex_encode_secret(database_key.as_ref());
    let pragma = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", raw_key.as_str()));
    connection.execute_batch(pragma.as_str())?;
    let cipher_version: String =
        connection.query_row("PRAGMA cipher_version", [], |row| row.get(0))?;
    if cipher_version.trim().is_empty() {
        return Err(io::Error::other("SQLCipher did not report a cipher version").into());
    }
    connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(connection)
}

fn derive_database_key(master_key: &[u8; KEY_LEN]) -> StoreResult<Zeroizing<[u8; KEY_LEN]>> {
    let hkdf = Hkdf::<Sha256>::new(None, master_key);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    hkdf.expand(DATABASE_KEY_CONTEXT, key.as_mut())
        .map_err(|_| io::Error::other("database key derivation failed"))?;
    Ok(key)
}

fn apply_migrations(connection: &mut Connection) -> StoreResult<()> {
    apply_migration_set(connection, MIGRATIONS)
}

fn apply_migration_set(connection: &mut Connection, migrations: &[Migration]) -> StoreResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations ( \
           version INTEGER PRIMARY KEY, \
           applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP \
         );",
    )?;
    for migration in migrations {
        let applied = connection
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = ?1",
                [migration.version],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if applied {
            continue;
        }
        if migration.foreign_keys_off {
            connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
        }
        let result: StoreResult<()> = (|| {
            let transaction = connection.transaction()?;
            transaction.execute_batch(migration.sql)?;
            let violation = transaction
                .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()?;
            if violation.is_some() {
                return Err(io::Error::other(format!(
                    "migration {} violates foreign keys",
                    migration.version
                ))
                .into());
            }
            transaction.execute(
                "INSERT INTO schema_migrations(version) VALUES (?1)",
                [migration.version],
            )?;
            transaction.commit()?;
            Ok(())
        })();
        if migration.foreign_keys_off {
            connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        }
        result?;
    }
    Ok(())
}

fn find_exact_document(
    connection: &Connection,
    file_sha256: &str,
) -> rusqlite::Result<Option<ExistingDocument>> {
    connection
        .query_row(
            "SELECT id, encrypted_locator, file_sha256, file_state \
             FROM source_documents WHERE file_sha256 = ?1",
            [file_sha256],
            |row| {
                Ok(ExistingDocument {
                    document_id: row.get(0)?,
                    encrypted_locator: row.get(1)?,
                    file_sha256: row.get(2)?,
                    file_state: row.get(3)?,
                })
            },
        )
        .optional()
}

fn find_document_by_id(
    connection: &Connection,
    document_id: &str,
) -> rusqlite::Result<Option<ExistingDocument>> {
    connection
        .query_row(
            "SELECT id, encrypted_locator, file_sha256, file_state \
             FROM source_documents WHERE id = ?1",
            [document_id],
            |row| {
                Ok(ExistingDocument {
                    document_id: row.get(0)?,
                    encrypted_locator: row.get(1)?,
                    file_sha256: row.get(2)?,
                    file_state: row.get(3)?,
                })
            },
        )
        .optional()
}

fn persist_source_deletion(
    connection: &mut Connection,
    document: &ExistingDocument,
    audit_id: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, 'source_file_deletion_decided', \
                   'user', 'user_requested', ?3, 'source-file-deletion-v1')",
        params![audit_id, document.document_id, document.file_sha256],
    )?;
    let changed = transaction.execute(
        "UPDATE source_documents \
         SET encrypted_locator = NULL, file_state = 'deleted', \
             deleted_at = CURRENT_TIMESTAMP, deletion_audit_id = ?1 \
         WHERE id = ?2 AND file_state = 'available'",
        params![audit_id, document.document_id],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    transaction.execute(
        "INSERT INTO review_items(id, external_record_id, reason_code, status) \
         SELECT ?1 || ':' || external_records.id, external_records.id, \
                'source_file_deleted', 'open' \
         FROM external_records \
         WHERE source_document_id = ?2 AND status IN ('staged', 'review') \
           AND NOT EXISTS ( \
             SELECT 1 FROM review_items \
             WHERE external_record_id = external_records.id \
               AND reason_code = 'source_file_deleted' AND status = 'open' \
           )",
        params![audit_id, document.document_id],
    )?;
    transaction.execute(
        "UPDATE external_records SET status = 'review' \
         WHERE source_document_id = ?1 AND status = 'staged'",
        [&document.document_id],
    )?;
    transaction.commit()
}

fn persist_import(
    connection: &mut Connection,
    input: &SourceDocumentImport<'_>,
    stored: &StoredFile,
) -> rusqlite::Result<SourceDocumentImportOutcome> {
    let transaction = connection.transaction()?;
    let existing = find_exact_document(&transaction, &stored.file_sha256)?;
    let (document_id, status) = if let Some(existing) = existing {
        let status = if existing.file_state == "available" && !stored.created {
            SourceDocumentImportStatus::AlreadyPresent
        } else {
            transaction.execute(
                "UPDATE source_documents \
                 SET encrypted_locator = ?1, file_state = 'available', \
                     deleted_at = NULL, deletion_audit_id = NULL \
                 WHERE id = ?2",
                params![stored.encrypted_locator, existing.document_id],
            )?;
            SourceDocumentImportStatus::Restored
        };
        (existing.document_id, status)
    } else {
        let byte_size = i64::try_from(stored.byte_size)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        transaction.execute(
            "INSERT INTO source_documents( \
               id, file_sha256, original_filename, mime_type, byte_size, \
               encrypted_locator, file_state \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'available')",
            params![
                input.document_id,
                stored.file_sha256,
                input.original_filename,
                input.mime_type,
                byte_size,
                stored.encrypted_locator,
            ],
        )?;
        (
            input.document_id.to_owned(),
            SourceDocumentImportStatus::Imported,
        )
    };
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            input.audit_id,
            document_id,
            import_audit_action(status),
            input.audit_actor,
            input.audit_reason,
            stored.file_sha256,
            input.audit_policy_version,
        ],
    )?;
    transaction.commit()?;
    Ok(SourceDocumentImportOutcome {
        document_id,
        status,
    })
}

fn mark_missing(
    connection: &mut Connection,
    document: &ExistingDocument,
    file_sha256: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE source_documents \
         SET file_state = 'missing', deleted_at = NULL, deletion_audit_id = NULL \
         WHERE id = ?1",
        [&document.document_id],
    )?;
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, 'source_file_missing', \
                   'system', 'storage_verification_failed', ?3, 'vault-storage-v1')",
        params![new_audit_id(), document.document_id, file_sha256],
    )?;
    transaction.commit()
}

fn import_audit_action(status: SourceDocumentImportStatus) -> &'static str {
    match status {
        SourceDocumentImportStatus::Imported => "manual_import_imported",
        SourceDocumentImportStatus::AlreadyPresent => "manual_import_already_present",
        SourceDocumentImportStatus::Restored => "manual_import_restored",
        SourceDocumentImportStatus::RestoreConfirmationRequired => {
            "manual_import_restore_confirmation_required"
        }
    }
}

fn new_audit_id() -> String {
    let mut random = [0_u8; 16];
    OsRng.fill_bytes(&mut random);
    format!("audit-{}", hex_encode(&random))
}

fn validate_import(input: &SourceDocumentImport<'_>) -> StoreResult<()> {
    for (name, value) in [
        ("audit_actor", input.audit_actor),
        ("audit_id", input.audit_id),
        ("audit_policy_version", input.audit_policy_version),
        ("audit_reason", input.audit_reason),
        ("document_id", input.document_id),
        ("mime_type", input.mime_type),
        ("original_filename", input.original_filename),
    ] {
        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must not be empty"),
            )
            .into());
        }
    }
    Ok(())
}

fn validate_classification(input: &TrustedDocumentClassification<'_>) -> StoreResult<()> {
    for (name, value) in [
        ("audit_id", input.audit_id),
        ("document_id", input.document_id),
        ("provider_key", input.provider_key),
        ("semantic_document_key", input.semantic_document_key),
    ] {
        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must not be empty"),
            )
            .into());
        }
    }
    if input.accounts.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "classification must contain at least one account",
        )
        .into());
    }
    for account in input.accounts {
        for (name, value) in [
            ("account_id", account.account_id),
            ("account_type", account.account_type),
            ("display_name", account.display_name),
        ] {
            if value.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{name} must not be empty"),
                )
                .into());
            }
        }
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}

fn hex_encode_secret(bytes: &[u8]) -> Zeroizing<String> {
    let mut hex = Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut *hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; KEY_LEN] = [0x91; KEY_LEN];

    fn open_store(root: &Path) -> ManualImportStore {
        let store =
            ManualImportStore::open(root, Zeroizing::new(KEY)).expect("open encrypted Vault");
        store
            .connection
            .execute(
                "INSERT OR IGNORE INTO money_sources( \
                   id, provider_key, display_name, source_type \
                 ) VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
                [],
            )
            .expect("seed money source");
        store
    }

    #[test]
    fn lists_only_safe_money_source_display_fields_in_stable_order() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let store = open_store(root.path());
        store
            .seed_money_source("source-alpha", "alpha", "Alpha Bank", "bank")
            .expect("seed second source");

        assert_eq!(
            store.list_money_sources().expect("list Money Sources"),
            vec![
                MoneySourceView {
                    display_name: "Alpha Bank".to_owned(),
                    money_source_id: "source-alpha".to_owned(),
                    source_type: "bank".to_owned(),
                },
                MoneySourceView {
                    display_name: "DBS".to_owned(),
                    money_source_id: "source-dbs".to_owned(),
                    source_type: "bank".to_owned(),
                },
            ]
        );
    }

    fn import<'a>(
        source_path: &'a Path,
        document_id: &'a str,
        audit_id: &'a str,
    ) -> SourceDocumentImport<'a> {
        SourceDocumentImport {
            audit_actor: "user",
            audit_id,
            audit_policy_version: "manual-import-v1",
            audit_reason: "manual_import",
            document_id,
            mime_type: "application/pdf",
            original_filename: "DBS-July-2026.pdf",
            source_path,
        }
    }

    #[test]
    fn migrates_existing_document_relationships_to_pending_semantic_identity() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let database_path = root.path().join(DATABASE_FILE_NAME);
        let mut connection = open_encrypted_database(
            &database_path,
            &KEY,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .expect("open encrypted database");
        apply_migration_set(&mut connection, &MIGRATIONS[..2]).expect("apply old schema");
        connection
            .execute(
                "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
                 VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
                [],
            )
            .expect("seed source");
        connection
            .execute(
                "INSERT INTO source_documents( \
                   id, money_source_id, file_sha256, semantic_document_key, \
                   original_filename, mime_type, byte_size, encrypted_locator, file_state \
                 ) VALUES ( \
                   'document-existing', 'source-dbs', ?1, 'dbs:checking:2026-06', \
                   'DBS-June-2026.pdf', 'application/pdf', 2048, \
                   'files/document-existing.ccenv', 'available' \
                 )",
                ["e".repeat(64)],
            )
            .expect("seed document");
        connection
            .execute(
                "INSERT INTO parse_runs( \
                   id, source_document_id, normalization_profile_id, profile_json, status \
                 ) VALUES ( \
                   'parse-existing', 'document-existing', 'profile-v1', '{}', 'succeeded' \
                 )",
                [],
            )
            .expect("seed parse relationship");

        apply_migration_set(&mut connection, &MIGRATIONS[2..]).expect("apply pending identity");

        let not_null: i64 = connection
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('source_documents') \
                 WHERE name = 'semantic_document_key'",
                [],
                |row| row.get(0),
            )
            .expect("read semantic column");
        assert_eq!(not_null, 0);
        let semantic_document_key: String = connection
            .query_row(
                "SELECT semantic_document_key FROM source_documents \
                 WHERE id = 'document-existing'",
                [],
                |row| row.get(0),
            )
            .expect("preserve identity");
        assert_eq!(semantic_document_key, "dbs:checking:2026-06");
        let source_document_id: String = connection
            .query_row(
                "SELECT source_document_id FROM parse_runs WHERE id = 'parse-existing'",
                [],
                |row| row.get(0),
            )
            .expect("preserve parse relationship");
        assert_eq!(source_document_id, "document-existing");
        assert!(
            connection
                .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()
                .expect("check foreign keys")
                .is_none()
        );
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .expect("foreign keys enabled"),
            1
        );
        connection
            .execute(
                "INSERT INTO source_documents( \
                   id, money_source_id, file_sha256, \
                   original_filename, mime_type, byte_size, encrypted_locator, file_state \
                 ) VALUES ( \
                   'document-pending', 'source-dbs', ?1, \
                   'Pending.pdf', 'application/pdf', 1024, \
                   'files/document-pending.ccenv', 'available' \
                 )",
                ["p".repeat(64)],
            )
            .expect("insert pending identity");
        let pending_identity: Option<String> = connection
            .query_row(
                "SELECT semantic_document_key FROM source_documents \
                 WHERE id = 'document-pending'",
                [],
                |row| row.get(0),
            )
            .expect("read pending identity");
        assert_eq!(pending_identity, None);
        let source_not_null: i64 = connection
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('source_documents') \
                 WHERE name = 'money_source_id'",
                [],
                |row| row.get(0),
            )
            .expect("read source column");
        assert_eq!(source_not_null, 0);
        connection
            .execute(
                "INSERT INTO source_documents( \
                   id, file_sha256, original_filename, mime_type, byte_size, \
                   encrypted_locator, file_state \
                 ) VALUES ( \
                   'document-unassigned', ?1, 'Pending.csv', 'text/csv', 1024, \
                   'files/document-unassigned.ccenv', 'available' \
                 )",
                ["u".repeat(64)],
            )
            .expect("insert unassigned source");
        let pending_source: Option<String> = connection
            .query_row(
                "SELECT money_source_id FROM source_documents \
                 WHERE id = 'document-unassigned'",
                [],
                |row| row.get(0),
            )
            .expect("read pending source");
        assert_eq!(pending_source, None);

        let invalid = [Migration {
            version: 99,
            sql: "INSERT INTO parse_runs( \
                    id, source_document_id, normalization_profile_id, profile_json, status \
                  ) VALUES ( \
                    'parse-invalid', 'missing-document', 'profile-v1', '{}', 'failed' \
                  )",
            foreign_keys_off: true,
        }];
        assert!(apply_migration_set(&mut connection, &invalid).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .expect("foreign keys restored after failure"),
            1
        );
        let invalid_rows: i64 = connection
            .query_row(
                "SELECT count(*) FROM parse_runs WHERE id = 'parse-invalid'",
                [],
                |row| row.get(0),
            )
            .expect("invalid migration rolled back");
        assert_eq!(invalid_rows, 0);
    }

    #[test]
    fn routes_trusted_classification_to_one_source_and_reuses_the_account() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.csv");
        fs::write(&source_path, b"synthetic statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .register_import(
                &SourceDocumentImport {
                    audit_actor: "user",
                    audit_id: "audit-import",
                    audit_policy_version: "manual-import-v1",
                    audit_reason: "manual_import",
                    document_id: "document-unassigned",
                    mime_type: "text/csv",
                    original_filename: "statement.csv",
                    source_path: &source_path,
                },
                None,
            )
            .expect("capture unassigned source");
        let accounts = [TrustedAccountCandidate {
            account_id: "account-candidate",
            account_type: "deposit_account",
            currency: Some("SGD"),
            display_name: "Synthetic checking",
            masked_identifier: Some("••001"),
            provider_account_id: Some("checking-001"),
        }];
        let routed = store
            .apply_trusted_classification(&TrustedDocumentClassification {
                accounts: &accounts,
                audit_id: "audit-classify",
                document_id: "document-unassigned",
                provider_key: "dbs",
                semantic_document_key: "dbs:checking:2026-07",
            })
            .expect("route classification");
        assert_eq!(routed.status, SourceDocumentRoutingStatus::Routed);
        assert_eq!(routed.money_source_id.as_deref(), Some("source-dbs"));
        assert_eq!(routed.account_ids, vec!["account-candidate"]);
        assert!(
            store
                .list_unassigned_documents()
                .expect("list pending")
                .is_empty()
        );
        let documents = store.list_documents("source-dbs").expect("list routed");
        assert_eq!(documents.len(), 1);
        assert_eq!(
            documents[0].semantic_document_key.as_deref(),
            Some("dbs:checking:2026-07")
        );

        let repeated_accounts = [TrustedAccountCandidate {
            account_id: "account-ignored",
            account_type: "deposit_account",
            currency: Some("SGD"),
            display_name: "Synthetic checking",
            masked_identifier: Some("••001"),
            provider_account_id: Some("checking-001"),
        }];
        let repeated = store
            .apply_trusted_classification(&TrustedDocumentClassification {
                accounts: &repeated_accounts,
                audit_id: "audit-classify-repeat",
                document_id: "document-unassigned",
                provider_key: "dbs",
                semantic_document_key: "dbs:checking:2026-07",
            })
            .expect("repeat classification");
        assert_eq!(repeated.account_ids, vec!["account-candidate"]);
    }

    #[test]
    fn leaves_ambiguous_source_or_account_classification_unassigned() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF synthetic statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .connection
            .execute(
                "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
                 VALUES ('source-dbs-second', 'dbs', 'DBS second', 'bank')",
                [],
            )
            .expect("seed ambiguous source");
        store
            .register_import(
                &SourceDocumentImport {
                    audit_actor: "user",
                    audit_id: "audit-import",
                    audit_policy_version: "manual-import-v1",
                    audit_reason: "manual_import",
                    document_id: "document-unassigned",
                    mime_type: "application/pdf",
                    original_filename: "statement.pdf",
                    source_path: &source_path,
                },
                None,
            )
            .expect("capture unassigned source");
        let accounts = [TrustedAccountCandidate {
            account_id: "account-candidate",
            account_type: "deposit_account",
            currency: Some("SGD"),
            display_name: "Synthetic checking",
            masked_identifier: None,
            provider_account_id: Some("checking-001"),
        }];
        let ambiguous = store
            .apply_trusted_classification(&TrustedDocumentClassification {
                accounts: &accounts,
                audit_id: "audit-classify",
                document_id: "document-unassigned",
                provider_key: "dbs",
                semantic_document_key: "dbs:checking:2026-07",
            })
            .expect("classify ambiguous source");
        assert_eq!(
            ambiguous.status,
            SourceDocumentRoutingStatus::NeedsAttention
        );
        assert_eq!(ambiguous.reason, Some("money_source_ambiguous"));
        assert_eq!(
            store
                .list_unassigned_documents()
                .expect("list pending")
                .len(),
            1
        );
        let account_count: i64 = store
            .connection
            .query_row("SELECT count(*) FROM accounts", [], |row| row.get(0))
            .expect("count accounts");
        assert_eq!(account_count, 0);
    }

    #[test]
    fn imports_deduplicates_groups_and_restores_source_documents() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let first_path = root.path().join("first.pdf");
        fs::write(&first_path, b"%PDF synthetic first statement").expect("write first fixture");
        let mut store = open_store(root.path());

        let first = store
            .register_import(&import(&first_path, "document-first", "audit-first"), None)
            .expect("first import");
        assert_eq!(first.status, SourceDocumentImportStatus::Imported);
        let duplicate = store
            .register_import(
                &import(&first_path, "document-ignored", "audit-duplicate"),
                None,
            )
            .expect("duplicate import");
        assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

        let second_path = root.path().join("second.pdf");
        fs::write(&second_path, b"%PDF synthetic rescanned statement")
            .expect("write second fixture");
        let second = store
            .register_import(
                &import(&second_path, "document-second", "audit-second"),
                None,
            )
            .expect("second import");
        assert_eq!(second.status, SourceDocumentImportStatus::Imported);

        let first_sha256 = FileVault::prepare(&first_path)
            .expect("prepare first source")
            .file_sha256()
            .to_owned();
        let first_document = find_exact_document(&store.connection, &first_sha256)
            .expect("find document")
            .expect("existing document");
        let encrypted_path = root.path().join(
            first_document
                .encrypted_locator
                .as_deref()
                .expect("available locator"),
        );
        store
            .connection
            .execute(
                "INSERT INTO parse_runs( \
                   id, source_document_id, normalization_profile_id, profile_json, status \
                 ) VALUES ('parse-first', ?1, 'profile-v1', '{}', 'succeeded')",
                [&first_document.document_id],
            )
            .expect("seed retained parse relationship");
        store
            .connection
            .execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, stable_record_key, version, \
                   status, record_type, raw_json, validation_json \
                 ) VALUES \
                   ('record-staged', 'parse-first', ?1, 'record-staged', 1, \
                    'staged', 'transaction', '{}', '{}'), \
                   ('record-review', 'parse-first', ?1, 'record-review', 1, \
                    'review', 'transaction', '{}', '{}'), \
                   ('record-committed', 'parse-first', ?1, 'record-committed', 1, \
                    'committed', 'transaction', '{}', '{}')",
                [&first_document.document_id],
            )
            .expect("seed linked records");

        store
            .delete_source_document(&first_document.document_id, "audit-delete")
            .expect("delete encrypted source");
        assert!(!encrypted_path.exists());
        let deleted = find_document_by_id(&store.connection, &first_document.document_id)
            .expect("find deleted document")
            .expect("deleted document remains");
        assert_eq!(deleted.file_state, "deleted");
        assert!(deleted.encrypted_locator.is_none());
        let retained_parses: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM parse_runs WHERE source_document_id = ?1",
                [&first_document.document_id],
                |row| row.get(0),
            )
            .expect("count retained parse relationships");
        assert_eq!(retained_parses, 1);
        let record_states = store
            .connection
            .prepare(
                "SELECT id, status FROM external_records \
                 WHERE source_document_id = ?1 ORDER BY id",
            )
            .expect("prepare linked record query")
            .query_map([&first_document.document_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query linked records")
            .collect::<Result<Vec<_>, _>>()
            .expect("read linked records");
        assert_eq!(
            record_states,
            vec![
                ("record-committed".to_owned(), "committed".to_owned()),
                ("record-review".to_owned(), "review".to_owned()),
                ("record-staged".to_owned(), "review".to_owned()),
            ]
        );
        let deletion_review_items: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM review_items \
                 WHERE reason_code = 'source_file_deleted' AND status = 'open'",
                [],
                |row| row.get(0),
            )
            .expect("count source deletion review items");
        assert_eq!(deletion_review_items, 2);

        assert!(
            store
                .delete_source_document(&first_document.document_id, "audit-delete-retry")
                .is_err()
        );
        let deletion_audits: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
                [&first_document.document_id],
                |row| row.get(0),
            )
            .expect("count deletion decisions after retry");
        assert_eq!(deletion_audits, 1);

        let confirmation = store
            .register_import(
                &import(&first_path, "document-ignored-restored", "audit-restored"),
                None,
            )
            .expect("request restore confirmation");
        assert_eq!(confirmation.document_id, first_document.document_id);
        assert_eq!(
            confirmation.status,
            SourceDocumentImportStatus::RestoreConfirmationRequired
        );
        assert!(!encrypted_path.exists());

        fs::write(&first_path, b"%PDF changed after restore confirmation")
            .expect("change selected source");
        assert!(
            store
                .register_import(
                    &import(&first_path, "document-changed", "audit-changed"),
                    Some(&first_document.document_id),
                )
                .is_err()
        );
        assert_eq!(store.list_unassigned_documents().expect("list").len(), 2);
        fs::write(&first_path, b"%PDF synthetic first statement")
            .expect("restore selected source fixture");

        let restored = store
            .register_import(
                &import(&first_path, "document-ignored-restored", "audit-restored"),
                Some(&first_document.document_id),
            )
            .expect("restore exact source");
        assert_eq!(restored.document_id, first_document.document_id);
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
        assert_eq!(store.list_unassigned_documents().expect("list").len(), 2);
    }

    #[test]
    fn deletion_recovery_removes_a_blob_left_after_the_tombstone_commit() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF deletion recovery statement").expect("write fixture");
        let mut store = open_store(root.path());
        let imported = store
            .register_import(
                &import(&source_path, "document-delete", "audit-import"),
                None,
            )
            .expect("import source");
        let document = find_document_by_id(&store.connection, &imported.document_id)
            .expect("find document")
            .expect("imported document");
        let encrypted_path = root.path().join(
            document
                .encrypted_locator
                .as_deref()
                .expect("available locator"),
        );

        persist_source_deletion(&mut store.connection, &document, "audit-delete")
            .expect("commit deletion decision");
        assert!(
            encrypted_path.exists(),
            "simulate crash before blob removal"
        );
        drop(store);

        let reopened = open_store(root.path());
        assert!(!encrypted_path.exists());
        let tombstone = find_document_by_id(&reopened.connection, &imported.document_id)
            .expect("find tombstone")
            .expect("document row retained");
        assert_eq!(tombstone.file_state, "deleted");
        let deletion_audits: i64 = reopened
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
                [&imported.document_id],
                |row| row.get(0),
            )
            .expect("count deletion audit");
        assert_eq!(deletion_audits, 1);
    }

    #[test]
    fn missing_storage_is_not_recorded_as_a_user_deletion() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF missing before deletion").expect("write fixture");
        let mut store = open_store(root.path());
        let imported = store
            .register_import(
                &import(&source_path, "document-missing-delete", "audit-import"),
                None,
            )
            .expect("import source");
        let locator = store.list_unassigned_documents().expect("list")[0]
            .encrypted_locator
            .clone()
            .expect("available locator");
        store
            .files
            .remove(&locator)
            .expect("remove encrypted fixture");

        assert!(
            store
                .delete_source_document(&imported.document_id, "audit-delete")
                .is_err()
        );
        let document = find_document_by_id(&store.connection, &imported.document_id)
            .expect("find document")
            .expect("document row retained");
        assert_eq!(document.file_state, "missing");
        let deletion_audits: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
                [&imported.document_id],
                |row| row.get(0),
            )
            .expect("count deletion audit");
        assert_eq!(deletion_audits, 0);
    }

    #[test]
    fn removes_a_new_encrypted_file_when_the_database_transaction_fails() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF rollback statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .connection
            .execute(
                "INSERT INTO audit_log( \
                   id, entity_type, entity_id, action, actor, reason, policy_version \
                 ) VALUES ('audit-conflict', 'test', 'test', 'seed', 'test', 'test', 'test')",
                [],
            )
            .expect("seed audit conflict");

        assert!(
            store
                .register_import(
                    &import(&source_path, "document-rollback", "audit-conflict"),
                    None,
                )
                .is_err()
        );
        let rows: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM source_documents WHERE id = 'document-rollback'",
                [],
                |row| row.get(0),
            )
            .expect("count rolled-back documents");
        assert_eq!(rows, 0);
        let files = root.path().join("files");
        assert_eq!(fs::read_dir(files).expect("files directory").count(), 0);
    }

    #[test]
    fn marks_an_absent_registered_file_missing_and_restores_it_on_reimport() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF missing statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .register_import(
                &import(&source_path, "document-missing", "audit-import"),
                None,
            )
            .expect("import source");
        let locator = store.list_unassigned_documents().expect("list")[0]
            .encrypted_locator
            .clone()
            .expect("available locator");
        store
            .files
            .remove(&locator)
            .expect("remove file outside database");
        drop(store);

        let mut reopened = open_store(root.path());
        assert_eq!(
            reopened.list_unassigned_documents().expect("list")[0].file_state,
            "missing"
        );
        let restored = reopened
            .register_import(
                &import(&source_path, "document-ignored", "audit-restore"),
                None,
            )
            .expect("restore missing source");
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
    }

    #[test]
    fn marks_a_tampered_registered_file_missing_on_reopen() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF tampered statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .register_import(
                &import(&source_path, "document-tampered", "audit-import"),
                None,
            )
            .expect("import source");
        let locator = store.list_unassigned_documents().expect("list")[0]
            .encrypted_locator
            .clone()
            .expect("available locator");
        let encrypted_path = root.path().join(locator);
        let mut envelope = fs::read(&encrypted_path).expect("read encrypted fixture");
        *envelope.last_mut().expect("non-empty envelope") ^= 1;
        fs::write(encrypted_path, envelope).expect("tamper encrypted fixture");
        drop(store);

        let reopened = open_store(root.path());
        assert_eq!(
            reopened.list_unassigned_documents().expect("list")[0].file_state,
            "missing"
        );
    }

    #[test]
    fn restores_a_valid_envelope_found_at_the_wrong_document_locator() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let first_path = root.path().join("first.pdf");
        let second_path = root.path().join("second.pdf");
        fs::write(&first_path, b"%PDF first statement").expect("write first fixture");
        fs::write(&second_path, b"%PDF second statement").expect("write second fixture");
        let mut store = open_store(root.path());
        store
            .register_import(&import(&first_path, "document-first", "audit-first"), None)
            .expect("import first source");
        store
            .register_import(
                &import(&second_path, "document-second", "audit-second"),
                None,
            )
            .expect("import second source");

        let first_sha256 = FileVault::prepare(&first_path)
            .expect("prepare first source")
            .file_sha256()
            .to_owned();
        let second_sha256 = FileVault::prepare(&second_path)
            .expect("prepare second source")
            .file_sha256()
            .to_owned();
        let first = find_exact_document(&store.connection, &first_sha256)
            .expect("find first document")
            .expect("first document");
        let second = find_exact_document(&store.connection, &second_sha256)
            .expect("find second document")
            .expect("second document");
        let first_envelope = fs::read(
            root.path()
                .join(first.encrypted_locator.expect("first locator")),
        )
        .expect("read first envelope");
        fs::write(
            root.path()
                .join(second.encrypted_locator.expect("second locator")),
            first_envelope,
        )
        .expect("misplace valid envelope");
        drop(store);

        let mut store = open_store(root.path());
        let documents = store.list_unassigned_documents().expect("list");
        let second_document = documents
            .iter()
            .find(|document| document.document_id == "document-second")
            .expect("second document view");
        assert_eq!(second_document.file_state, "missing");

        let restored = store
            .register_import(
                &import(&second_path, "document-ignored", "audit-restored"),
                None,
            )
            .expect("restore second source");
        assert_eq!(restored.document_id, "document-second");
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
    }

    #[test]
    fn rejects_the_wrong_database_key_and_cleans_an_unregistered_envelope_on_open() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let orphan_path = root.path().join("orphan.pdf");
        fs::write(&orphan_path, b"unregistered evidence").expect("write orphan source");
        let store = open_store(root.path());
        let orphan = store
            .files
            .store(&KEY, &orphan_path)
            .expect("store orphaned envelope");
        let encrypted_path = root.path().join(&orphan.encrypted_locator);
        assert!(encrypted_path.exists());
        drop(store);

        let reopened = open_store(root.path());
        assert!(!encrypted_path.exists());
        drop(reopened);
        assert!(ManualImportStore::open(root.path(), Zeroizing::new([0x92; KEY_LEN])).is_err());
        let database_bytes =
            fs::read(root.path().join("finance.sqlite")).expect("read encrypted database");
        assert!(
            !database_bytes
                .windows("source-dbs".len())
                .any(|part| part == b"source-dbs")
        );
    }

    #[test]
    fn persists_only_statement_password_reference_state() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let store = open_store(root.path());

        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("initial statement password state"),
            None
        );

        store
            .begin_statement_password_save("source-dbs", "money-source:source-dbs")
            .expect("begin save");
        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("pending save state"),
            Some(StatementPasswordState {
                money_source_id: "source-dbs".to_owned(),
                secret_storage_key: "money-source:source-dbs".to_owned(),
                status: StatementPasswordStatus::PendingSave,
            })
        );
        assert_eq!(
            store
                .pending_statement_password_states()
                .expect("pending states")
                .len(),
            1
        );

        store
            .mark_statement_password_saved("source-dbs")
            .expect("finish save");
        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("saved state")
                .expect("saved reference")
                .status,
            StatementPasswordStatus::Saved
        );

        store
            .begin_statement_password_delete("source-dbs")
            .expect("begin delete");
        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("pending delete state")
                .expect("pending delete reference")
                .status,
            StatementPasswordStatus::PendingDelete
        );
        store
            .remove_statement_password_ref("source-dbs", "pending_delete")
            .expect("finish delete");
        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("cleared statement password state"),
            None
        );
    }
}
