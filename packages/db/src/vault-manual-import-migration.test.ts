import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { afterEach, describe, expect, it } from "vitest";

import { applyMigrations } from "./index";
import { resetTestDatabase } from "./testing";

const testDirectories: string[] = [];
const testDatabases: DatabaseSync[] = [];

function migration(name: string): string {
  return readFileSync(new URL(`../migrations/${name}`, import.meta.url), "utf8");
}

function openDatabase(maxVersion = 3): DatabaseSync {
  process.env.CANCAN_TEST = "1";
  const directory = mkdtempSync(join(tmpdir(), "cancan-test-vault-manual-import-"));
  testDirectories.push(directory);
  const databasePath = join(directory, "source-documents.sqlite");
  resetTestDatabase(databasePath);
  const database = new DatabaseSync(databasePath);
  testDatabases.push(database);
  const migrations = [
    { version: 1, name: "0001_synthetic_core.sql" },
    { version: 2, name: "0002_vault_manual_import.sql" },
    { version: 3, name: "0003_source_document_pending_identity.sql" },
  ]
    .filter(({ version }) => version <= maxVersion)
    .map(({ version, name }) => ({
      version,
      sql: migration(name),
      foreignKeysOff: version === 3,
    }));
  applyMigrations(database, migrations);
  applyMigrations(database, migrations);
  database
    .prepare(`
      INSERT INTO money_sources(id, provider_key, display_name, source_type)
      VALUES ('source-dbs', 'dbs', 'DBS', 'bank')
    `)
    .run();
  return database;
}

afterEach(() => {
  for (const database of testDatabases.splice(0)) {
    database.close();
  }
  for (const directory of testDirectories.splice(0)) {
    rmSync(directory, { force: true, recursive: true });
  }
  delete process.env.CANCAN_TEST;
});

describe("vault manual import migration", () => {
  it("rolls back an invalid rebuild and restores foreign-key enforcement", () => {
    const database = openDatabase(2);

    expect(() =>
      applyMigrations(database, [
        {
          version: 99,
          foreignKeysOff: true,
          sql: `
            INSERT INTO parse_runs(
              id, source_document_id, normalization_profile_id, profile_json, status
            ) VALUES ('parse-invalid', 'missing-document', 'profile-v1', '{}', 'failed')
          `,
        },
      ]),
    ).toThrow(/violates foreign keys/);

    expect(database.prepare("PRAGMA foreign_keys").get()).toEqual({ foreign_keys: 1 });
    expect(
      database.prepare("SELECT id FROM parse_runs WHERE id = 'parse-invalid'").get(),
    ).toBeUndefined();
    expect(
      database.prepare("SELECT version FROM schema_migrations WHERE version = 99").get(),
    ).toBeUndefined();
  });

  it("preserves existing relationships while allowing identity to remain pending", () => {
    const database = openDatabase(2);
    database
      .prepare(`
        INSERT INTO source_documents(
          id, money_source_id, file_sha256, semantic_document_key,
          original_filename, mime_type, byte_size, encrypted_locator, file_state
        ) VALUES (
          'document-existing', 'source-dbs', ?, 'dbs:checking:2026-06',
          'DBS-June-2026.pdf', 'application/pdf', 2048,
          'files/document-existing.ccenv', 'available'
        )
      `)
      .run("e".repeat(64));
    database
      .prepare(`
        INSERT INTO parse_runs(
          id, source_document_id, normalization_profile_id, profile_json, status
        ) VALUES (
          'parse-existing', 'document-existing', 'profile-v1', '{}', 'succeeded'
        )
      `)
      .run();

    applyMigrations(database, [
      {
        version: 3,
        sql: migration("0003_source_document_pending_identity.sql"),
        foreignKeysOff: true,
      },
    ]);

    const identityColumn = database
      .prepare("PRAGMA table_info(source_documents)")
      .all()
      .find((column) => column.name === "semantic_document_key");
    expect(identityColumn?.notnull).toBe(0);
    expect(
      database
        .prepare("SELECT semantic_document_key FROM source_documents WHERE id = ?")
        .get("document-existing"),
    ).toEqual({ semantic_document_key: "dbs:checking:2026-06" });
    expect(
      database.prepare("SELECT source_document_id FROM parse_runs WHERE id = ?").get(
        "parse-existing",
      ),
    ).toEqual({ source_document_id: "document-existing" });
    expect(database.prepare("PRAGMA foreign_key_check").all()).toEqual([]);
    expect(database.prepare("PRAGMA foreign_keys").get()).toEqual({ foreign_keys: 1 });

    database
      .prepare(`
        INSERT INTO source_documents(
          id, money_source_id, file_sha256,
          original_filename, mime_type, byte_size, encrypted_locator, file_state
        ) VALUES (
          'document-pending', 'source-dbs', ?,
          'Pending.pdf', 'application/pdf', 1024,
          'files/document-pending.ccenv', 'available'
        )
      `)
      .run("p".repeat(64));
    expect(
      database
        .prepare("SELECT semantic_document_key FROM source_documents WHERE id = ?")
        .get("document-pending"),
    ).toEqual({ semantic_document_key: null });
  });

  it("enforces source-file lifecycle and exact byte identity", () => {
    const database = openDatabase();
    const insert = database.prepare(`
      INSERT INTO source_documents(
        id, money_source_id, file_sha256, semantic_document_key,
        original_filename, mime_type, byte_size, encrypted_locator, file_state
      ) VALUES (?, 'source-dbs', ?, ?, ?, 'application/pdf', 4096, ?, 'available')
    `);
    insert.run(
      "document-july",
      "a".repeat(64),
      "dbs:checking:2026-07",
      "DBS-July-2026.pdf",
      "files/document-july.ccenv",
    );

    expect(() =>
      insert.run(
        "document-duplicate",
        "a".repeat(64),
        "dbs:checking:2026-07",
        "duplicate.pdf",
        "files/duplicate.ccenv",
      ),
    ).toThrow(/UNIQUE constraint failed/);
    expect(() =>
      database
        .prepare(`
          INSERT INTO source_documents(
            id, money_source_id, file_sha256, semantic_document_key,
            original_filename, mime_type, byte_size, file_state
          ) VALUES (
            'invalid', 'source-dbs', ?, 'invalid',
            'invalid.pdf', 'application/pdf', 1, 'available'
          )
        `)
        .run("b".repeat(64)),
    ).toThrow(/invalid source document file lifecycle/);
  });

  it("uses the source-owned document-list index", () => {
    const database = openDatabase();
    const plan = database
      .prepare(`
        EXPLAIN QUERY PLAN
        SELECT id FROM source_documents
        WHERE money_source_id = ?
        ORDER BY received_at DESC, id
      `)
      .all("source-dbs")
      .map((row) => String(row.detail));
    expect(plan.join(" ")).toContain("source_documents_money_source_received");
  });
});
