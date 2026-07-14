import { inTransaction, type SqliteDatabase } from "./sqlite";

export interface Migration {
  version: number;
  sql: string;
}

export function applyMigrations(database: SqliteDatabase, migrations: Migration[]): void {
  database.exec("PRAGMA foreign_keys = ON");
  database.exec(`
    CREATE TABLE IF NOT EXISTS schema_migrations (
      version INTEGER PRIMARY KEY,
      applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    )
  `);

  const applied = database.prepare("SELECT version FROM schema_migrations WHERE version = ?");
  const recordApplied = database.prepare("INSERT INTO schema_migrations(version) VALUES (?)");

  for (const migration of [...migrations].sort((left, right) => left.version - right.version)) {
    if (applied.get(migration.version)) {
      continue;
    }
    inTransaction(database, () => {
      database.exec(migration.sql);
      recordApplied.run(migration.version);
    });
  }
}
