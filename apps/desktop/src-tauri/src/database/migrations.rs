use super::*;

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
