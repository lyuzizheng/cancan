import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { afterEach, describe, expect, it } from "vitest";

import { applyMigrations } from "./index";
import { resetTestDatabase } from "./testing";

const testDirectories: string[] = [];
const testDatabases: DatabaseSync[] = [];

function openDatabase(): DatabaseSync {
  process.env.CANCAN_TEST = "1";
  const directory = mkdtempSync(join(tmpdir(), "cancan-test-vault-manual-import-"));
  testDirectories.push(directory);
  const databasePath = join(directory, "source-documents.sqlite");
  resetTestDatabase(databasePath);
  const database = new DatabaseSync(databasePath);
  testDatabases.push(database);
  const migration = (name: string) =>
    readFileSync(new URL(`../migrations/${name}`, import.meta.url), "utf8");
  const migrations = [
    { version: 1, sql: migration("0001_synthetic_core.sql") },
    { version: 2, sql: migration("0002_vault_manual_import.sql") },
  ];
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
