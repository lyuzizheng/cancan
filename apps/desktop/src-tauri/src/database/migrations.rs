use super::*;

pub(super) struct Migration {
    pub(super) version: i64,
    pub(super) sql: &'static str,
    pub(super) foreign_keys_off: bool,
}

pub(super) const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../../../../../packages/db/migrations/0001_synthetic_core.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 2,
        sql: include_str!("../../../../../packages/db/migrations/0002_vault_manual_import.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 3,
        sql: include_str!(
            "../../../../../packages/db/migrations/0003_source_document_pending_identity.sql"
        ),
        foreign_keys_off: true,
    },
    Migration {
        version: 4,
        sql: include_str!(
            "../../../../../packages/db/migrations/0004_source_document_pending_source.sql"
        ),
        foreign_keys_off: true,
    },
    Migration {
        version: 5,
        sql: include_str!(
            "../../../../../packages/db/migrations/0005_money_source_statement_password.sql"
        ),
        foreign_keys_off: false,
    },
    Migration {
        version: 6,
        sql: include_str!("../../../../../packages/db/migrations/0006_review_ledger.sql"),
        foreign_keys_off: true,
    },
    Migration {
        version: 7,
        sql: include_str!("../../../../../packages/db/migrations/0007_local_inbox.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 8,
        sql: include_str!(
            "../../../../../packages/db/migrations/0008_external_record_posting_status.sql"
        ),
        foreign_keys_off: false,
    },
    Migration {
        version: 9,
        sql: include_str!("../../../../../packages/db/migrations/0009_post_pr41_hardening.sql"),
        foreign_keys_off: true,
    },
    Migration {
        version: 10,
        sql: include_str!("../../../../../packages/db/migrations/0010_gmail_accounts.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 11,
        sql: include_str!(
            "../../../../../packages/db/migrations/0011_phase1_intake_foundation.sql"
        ),
        foreign_keys_off: false,
    },
    Migration {
        version: 12,
        sql: include_str!("../../../../../packages/db/migrations/0012_provider_root_identity.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 13,
        sql: include_str!("../../../../../packages/db/migrations/0013_committed_version_final.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 14,
        sql: include_str!("../../../../../packages/db/migrations/0014_operational_logs.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 15,
        sql: include_str!(
            "../../../../../packages/db/migrations/0015_source_document_content_fingerprint.sql"
        ),
        foreign_keys_off: false,
    },
];

pub(super) fn open_encrypted_database(
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

pub(super) fn derive_database_key(
    master_key: &[u8; KEY_LEN],
) -> StoreResult<Zeroizing<[u8; KEY_LEN]>> {
    let hkdf = Hkdf::<Sha256>::new(None, master_key);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    hkdf.expand(DATABASE_KEY_CONTEXT, key.as_mut())
        .map_err(|_| io::Error::other("database key derivation failed"))?;
    Ok(key)
}

pub(super) fn apply_migrations(connection: &mut Connection) -> StoreResult<()> {
    apply_migration_set(connection, MIGRATIONS)
}

pub(super) fn apply_migration_set(
    connection: &mut Connection,
    migrations: &[Migration],
) -> StoreResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations ( \
           version INTEGER PRIMARY KEY, \
           applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP \
         );",
    )?;
    if let Some(supported_version) = migrations.last().map(|migration| migration.version) {
        let database_version =
            connection.query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, Option<i64>>(0)
            })?;
        if let Some(database_version) =
            database_version.filter(|version| *version > supported_version)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "database schema version {database_version} is newer than supported version {supported_version}"
                ),
            )
            .into());
        }
    }
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
