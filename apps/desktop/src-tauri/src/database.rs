use crate::vault::{FileVault, StoredFile};
use hkdf::Hkdf;
use rand::{RngCore, rngs::OsRng};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::Sha256;
use std::{collections::HashSet, error::Error, fs, io, path::Path};
use zeroize::Zeroizing;

const KEY_LEN: usize = 32;
const DATABASE_KEY_CONTEXT: &[u8] = b"cancan:database:v1";
const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        include_str!("../../../../packages/db/migrations/0001_synthetic_core.sql"),
    ),
    (
        2,
        include_str!("../../../../packages/db/migrations/0002_vault_manual_import.sql"),
    ),
];

type StoreResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceDocumentImportStatus {
    Imported,
    AlreadyPresent,
    Restored,
    ProbableExistingStatement,
}

#[derive(Debug, Eq, PartialEq)]
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
    pub money_source_id: &'a str,
    pub original_filename: &'a str,
    pub semantic_document_key: &'a str,
    pub source_path: &'a Path,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SourceDocumentView {
    pub document_id: String,
    pub encrypted_locator: Option<String>,
    pub file_sha256: String,
    pub file_state: String,
    pub original_filename: String,
    pub semantic_document_key: String,
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
    pub fn open(root: &Path, master_key: [u8; KEY_LEN]) -> StoreResult<Self> {
        fs::create_dir_all(root)?;
        let mut connection = open_encrypted_database(&root.join("finance.sqlite"), &master_key)?;
        apply_migrations(&mut connection)?;
        let mut store = Self {
            connection,
            files: FileVault::new(root),
            master_key: Zeroizing::new(master_key),
        };
        store.reconcile_files()?;
        Ok(store)
    }

    pub fn register_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        validate_import(input)?;
        let source = FileVault::prepare(input.source_path)?;
        let existing = find_exact_document(&self.connection, source.file_sha256())?;
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

    pub fn list_documents(&self, money_source_id: &str) -> StoreResult<Vec<SourceDocumentView>> {
        let mut statement = self.connection.prepare(
            "SELECT id, file_sha256, semantic_document_key, original_filename, \
                    encrypted_locator, file_state \
             FROM source_documents \
             WHERE money_source_id = ?1 \
             ORDER BY received_at DESC, id",
        )?;
        let rows = statement.query_map([money_source_id], |row| {
            Ok(SourceDocumentView {
                document_id: row.get(0)?,
                file_sha256: row.get(1)?,
                semantic_document_key: row.get(2)?,
                original_filename: row.get(3)?,
                encrypted_locator: row.get(4)?,
                file_state: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
            if document.file_state == "available" && !self.files.exists(locator)? {
                mark_missing(&mut self.connection, &document, &document.file_sha256)?;
            }
        }
        self.files.remove_unreferenced(&referenced)?;
        Ok(())
    }
}

fn open_encrypted_database(path: &Path, master_key: &[u8; KEY_LEN]) -> StoreResult<Connection> {
    let connection = Connection::open(path)?;
    let database_key = derive_database_key(master_key)?;
    let raw_key = hex_encode(database_key.as_ref());
    connection.execute_batch(&format!("PRAGMA key = \"x'{raw_key}'\";"))?;
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

fn apply_migrations(connection: &mut Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations ( \
           version INTEGER PRIMARY KEY, \
           applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP \
         );",
    )?;
    for (version, sql) in MIGRATIONS {
        let applied = connection
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = ?1",
                [version],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if applied {
            continue;
        }
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version) VALUES (?1)",
            [version],
        )?;
        transaction.commit()?;
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
        let semantic_match = transaction
            .query_row(
                "SELECT 1 FROM source_documents \
                 WHERE money_source_id = ?1 AND semantic_document_key = ?2 LIMIT 1",
                params![input.money_source_id, input.semantic_document_key],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        let byte_size = i64::try_from(stored.byte_size)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        transaction.execute(
            "INSERT INTO source_documents( \
               id, money_source_id, file_sha256, semantic_document_key, \
               original_filename, mime_type, byte_size, encrypted_locator, file_state \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'available')",
            params![
                input.document_id,
                input.money_source_id,
                stored.file_sha256,
                input.semantic_document_key,
                input.original_filename,
                input.mime_type,
                byte_size,
                stored.encrypted_locator,
            ],
        )?;
        let status = if semantic_match {
            SourceDocumentImportStatus::ProbableExistingStatement
        } else {
            SourceDocumentImportStatus::Imported
        };
        (input.document_id.to_owned(), status)
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
        SourceDocumentImportStatus::ProbableExistingStatement => {
            "manual_import_probable_existing_statement"
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
        ("money_source_id", input.money_source_id),
        ("original_filename", input.original_filename),
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

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; KEY_LEN] = [0x91; KEY_LEN];

    fn open_store(root: &Path) -> ManualImportStore {
        let store = ManualImportStore::open(root, KEY).expect("open encrypted Vault");
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

    fn import<'a>(
        source_path: &'a Path,
        document_id: &'a str,
        audit_id: &'a str,
        semantic_document_key: &'a str,
    ) -> SourceDocumentImport<'a> {
        SourceDocumentImport {
            audit_actor: "user",
            audit_id,
            audit_policy_version: "manual-import-v1",
            audit_reason: "manual_import",
            document_id,
            mime_type: "application/pdf",
            money_source_id: "source-dbs",
            original_filename: "DBS-July-2026.pdf",
            semantic_document_key,
            source_path,
        }
    }

    #[test]
    fn imports_deduplicates_groups_and_restores_source_documents() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let first_path = root.path().join("first.pdf");
        fs::write(&first_path, b"%PDF synthetic first statement").expect("write first fixture");
        let mut store = open_store(root.path());

        let first = store
            .register_import(&import(
                &first_path,
                "document-first",
                "audit-first",
                "dbs:checking:2026-07",
            ))
            .expect("first import");
        assert_eq!(first.status, SourceDocumentImportStatus::Imported);
        let duplicate = store
            .register_import(&import(
                &first_path,
                "document-ignored",
                "audit-duplicate",
                "dbs:checking:2026-07",
            ))
            .expect("duplicate import");
        assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

        let second_path = root.path().join("second.pdf");
        fs::write(&second_path, b"%PDF synthetic rescanned statement")
            .expect("write second fixture");
        let semantic_match = store
            .register_import(&import(
                &second_path,
                "document-second",
                "audit-second",
                "dbs:checking:2026-07",
            ))
            .expect("semantic match import");
        assert_eq!(
            semantic_match.status,
            SourceDocumentImportStatus::ProbableExistingStatement
        );

        let first_sha256 = FileVault::prepare(&first_path)
            .expect("prepare first source")
            .file_sha256()
            .to_owned();
        let first_document = find_exact_document(&store.connection, &first_sha256)
            .expect("find document")
            .expect("existing document");
        store
            .connection
            .execute(
                "INSERT INTO audit_log( \
                   id, entity_type, entity_id, action, actor, reason, policy_version \
                 ) VALUES ('audit-delete', 'source_document', ?1, \
                           'source_file_deletion_decided', 'user', 'user_requested', \
                           'manual-import-v1')",
                [&first_document.document_id],
            )
            .expect("append deletion audit");
        store
            .connection
            .execute(
                "UPDATE source_documents SET file_state = 'deleted', \
                   encrypted_locator = NULL, deleted_at = CURRENT_TIMESTAMP, \
                   deletion_audit_id = 'audit-delete' WHERE id = ?1",
                [&first_document.document_id],
            )
            .expect("create tombstone fixture");
        if let Some(locator) = first_document.encrypted_locator {
            store
                .files
                .remove(&locator)
                .expect("remove encrypted fixture");
        }

        let restored = store
            .register_import(&import(
                &first_path,
                "document-ignored-restored",
                "audit-restored",
                "dbs:checking:2026-07",
            ))
            .expect("restore exact source");
        assert_eq!(restored.document_id, first_document.document_id);
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
        assert_eq!(store.list_documents("source-dbs").expect("list").len(), 2);
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
                .register_import(&import(
                    &source_path,
                    "document-rollback",
                    "audit-conflict",
                    "dbs:checking:2026-08",
                ))
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
            .register_import(&import(
                &source_path,
                "document-missing",
                "audit-import",
                "dbs:checking:2026-09",
            ))
            .expect("import source");
        let locator = store.list_documents("source-dbs").expect("list")[0]
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
            reopened.list_documents("source-dbs").expect("list")[0].file_state,
            "missing"
        );
        let restored = reopened
            .register_import(&import(
                &source_path,
                "document-ignored",
                "audit-restore",
                "dbs:checking:2026-09",
            ))
            .expect("restore missing source");
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
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
            .register_import(&import(
                &first_path,
                "document-first",
                "audit-first",
                "dbs:checking:2026-10",
            ))
            .expect("import first source");
        store
            .register_import(&import(
                &second_path,
                "document-second",
                "audit-second",
                "dbs:checking:2026-11",
            ))
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

        let restored = store
            .register_import(&import(
                &second_path,
                "document-ignored",
                "audit-restored",
                "dbs:checking:2026-11",
            ))
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
        assert!(ManualImportStore::open(root.path(), [0x92; KEY_LEN]).is_err());
        let database_bytes =
            fs::read(root.path().join("finance.sqlite")).expect("read encrypted database");
        assert!(
            !database_bytes
                .windows("source-dbs".len())
                .any(|part| part == b"source-dbs")
        );
    }
}
