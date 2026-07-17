import { inTransaction, type SqliteDatabase } from "./sqlite";

export interface Migration {
  version: number;
  sql: string;
  foreignKeysOff?: boolean;
}

export function applyMigrations(database: SqliteDatabase, migrations: Migration[]): void {
  database.exec("PRAGMA foreign_keys = ON");
  database.exec(`
    CREATE TABLE IF NOT EXISTS schema_migrations (
      version INTEGER PRIMARY KEY,
      applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    )
  `);

  for (const migration of [...migrations].sort((left, right) => left.version - right.version)) {
    if (
      database
        .prepare("SELECT version FROM schema_migrations WHERE version = ?")
        .get(migration.version)
    ) {
      continue;
    }
    if (migration.foreignKeysOff) {
      database.exec("PRAGMA foreign_keys = OFF");
    }
    try {
      inTransaction(database, () => {
        database.exec(migration.sql);
        const violation = database.prepare("PRAGMA foreign_key_check").get();
        if (violation) {
          throw new Error(`migration ${migration.version} violates foreign keys`);
        }
        database
          .prepare("INSERT INTO schema_migrations(version) VALUES (?)")
          .run(migration.version);
      });
    } finally {
      if (migration.foreignKeysOff) {
        database.exec("PRAGMA foreign_keys = ON");
      }
    }
  }
}
