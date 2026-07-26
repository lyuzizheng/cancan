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

function openDatabase(maxVersion = 8): DatabaseSync {
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
    { version: 4, name: "0004_source_document_pending_source.sql" },
    { version: 5, name: "0005_money_source_statement_password.sql" },
    { version: 6, name: "0006_review_ledger.sql" },
    { version: 7, name: "0007_local_inbox.sql" },
    { version: 8, name: "0008_external_record_posting_status.sql" },
  ]
    .filter(({ version }) => version <= maxVersion)
    .map(({ version, name }) => ({
      version,
      sql: migration(name),
      foreignKeysOff: version === 3 || version === 4,
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

  it("preserves existing relationships while allowing source assignment to remain pending", () => {
    const database = openDatabase(3);
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
        version: 4,
        sql: migration("0004_source_document_pending_source.sql"),
        foreignKeysOff: true,
      },
    ]);

    const sourceColumn = database
      .prepare("PRAGMA table_info(source_documents)")
      .all()
      .find((column) => column.name === "money_source_id");
    expect(sourceColumn?.notnull).toBe(0);
    expect(
      database
        .prepare("SELECT money_source_id FROM source_documents WHERE id = ?")
        .get("document-existing"),
    ).toEqual({ money_source_id: "source-dbs" });
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
          id, file_sha256, original_filename, mime_type, byte_size,
          encrypted_locator, file_state
        ) VALUES (
          'document-unassigned', ?, 'Pending.csv', 'text/csv', 1024,
          'files/document-unassigned.ccenv', 'available'
        )
      `)
      .run("u".repeat(64));
    expect(
      database
        .prepare(
          "SELECT money_source_id, semantic_document_key FROM source_documents WHERE id = ?",
        )
        .get("document-unassigned"),
    ).toEqual({ money_source_id: null, semantic_document_key: null });
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

  it("stores only one statement-password reference and status per Money Source", () => {
    const database = openDatabase();

    expect(
      database
        .prepare("SELECT * FROM statement_secret_refs WHERE money_source_id = ?")
        .get("source-dbs"),
    ).toBeUndefined();

    database
      .prepare(`
        INSERT INTO statement_secret_refs(
          id, money_source_id, secret_storage_key, status, hint_label
        ) VALUES (?, ?, ?, 'saved', NULL)
      `)
      .run("statement-ref-dbs", "source-dbs", "money-source:source-dbs");

    expect(() =>
      database
        .prepare(`
          INSERT INTO statement_secret_refs(
            id, money_source_id, secret_storage_key, status
          ) VALUES (?, ?, ?, 'saved')
        `)
        .run("duplicate-source", "source-dbs", "another-storage-key"),
    ).toThrow(/UNIQUE constraint failed/);
    expect(() =>
      database
        .prepare("UPDATE statement_secret_refs SET status = 'not_saved'")
        .run(),
    ).toThrow(/CHECK constraint failed/);

    const columns = database
      .prepare("PRAGMA table_info(statement_secret_refs)")
      .all()
      .map((column) => String(column.name));
    expect(columns).toEqual([
      "id",
      "money_source_id",
      "secret_storage_key",
      "status",
      "hint_label",
      "created_at",
      "updated_at",
    ]);
  });

  it("migrates the current production schema to mutable review projections and durable jobs", () => {
    const database = openDatabase(5);
    database
      .prepare(`
        INSERT INTO source_documents(
          id, money_source_id, file_sha256, original_filename, mime_type, byte_size,
          encrypted_locator, file_state
        ) VALUES ('document-current', 'source-dbs', ?, 'Current.csv', 'text/csv', 1,
                  'files/current.ccenv', 'available')
      `)
      .run("c".repeat(64));
    database
      .prepare(`
        INSERT INTO parse_runs(id, source_document_id, normalization_profile_id, profile_json, status)
        VALUES ('parse-current', 'document-current', 'profile-v1', '{}', 'validated')
      `)
      .run();
    database
      .prepare(`
        INSERT INTO external_records(
          id, parse_run_id, source_document_id, stable_record_key, version, status,
          record_type, raw_json, validation_json
        ) VALUES ('record-current', 'parse-current', 'document-current', 'current:1', 1,
                  'review', 'transaction', '{}', '{}')
      `)
      .run();

    applyMigrations(database, [
      {
        version: 6,
        sql: migration("0006_review_ledger.sql"),
        foreignKeysOff: true,
      },
    ]);

    database
      .prepare("UPDATE external_records SET status = 'removed' WHERE id = 'record-current'")
      .run();
    database
      .prepare(`
        INSERT INTO jobs(id, job_type, status, input_json)
        VALUES ('job-current', 'commit_review_batch', 'queued', '{"reviewItemIds":[]}')
      `)
      .run();

    applyMigrations(database, [
      {
        version: 7,
        sql: migration("0007_local_inbox.sql"),
      },
    ]);

    expect(
      database.prepare("SELECT status FROM external_records WHERE id = 'record-current'").get(),
    ).toEqual({ status: "removed" });
    expect(database.prepare("SELECT job_type, status FROM jobs WHERE id = 'job-current'").get()).toEqual({
      job_type: "commit_review_batch",
      status: "queued",
    });
    applyMigrations(database, [
      {
        version: 8,
        sql: migration("0008_external_record_posting_status.sql"),
      },
    ]);

    expect(
      database
        .prepare("SELECT posting_status FROM external_records WHERE id = 'record-current'")
        .get(),
    ).toEqual({ posting_status: null });
    database
      .prepare("UPDATE external_records SET posting_status = 'posted' WHERE id = 'record-current'")
      .run();
    expect(
      database
        .prepare("SELECT posting_status FROM external_records WHERE id = 'record-current'")
        .get(),
    ).toEqual({ posting_status: "posted" });
    expect(() =>
      database
        .prepare("UPDATE external_records SET posting_status = 'pending' WHERE id = 'record-current'")
        .run(),
    ).toThrow(/CHECK constraint failed/);
    expect(database.prepare("PRAGMA foreign_key_check").all()).toEqual([]);
  });
});
