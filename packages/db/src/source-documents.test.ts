import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { afterEach, describe, expect, it } from "vitest";

import {
  applyMigrations,
  SourceDocumentRepository,
  type SourceDocumentImport,
} from "./index";
import { resetTestDatabase } from "./testing";

const testDirectories: string[] = [];
const testDatabases: DatabaseSync[] = [];
let previousTestMarker: string | undefined;

function openDatabase(): DatabaseSync {
  previousTestMarker = process.env.CANCAN_TEST;
  process.env.CANCAN_TEST = "1";
  const directory = mkdtempSync(join(tmpdir(), "cancan-test-source-documents-"));
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

function sourceImport(overrides: Partial<SourceDocumentImport> = {}): SourceDocumentImport {
  return {
    id: "document-july",
    moneySourceId: "source-dbs",
    fileSha256: "a".repeat(64),
    semanticDocumentKey: "dbs:checking:2026-07",
    originalFilename: "DBS-July-2026.pdf",
    mimeType: "application/pdf",
    byteSize: 4096,
    encryptedLocator: "files/aa/document-july.ccenv",
    audit: {
      id: "audit-import-july",
      actor: "user",
      reason: "manual_import",
      policyVersion: "manual-import-v1",
    },
    ...overrides,
  };
}

afterEach(() => {
  for (const database of testDatabases.splice(0)) {
    database.close();
  }
  for (const directory of testDirectories.splice(0)) {
    rmSync(directory, { force: true, recursive: true });
  }
  if (previousTestMarker === undefined) {
    delete process.env.CANCAN_TEST;
  } else {
    process.env.CANCAN_TEST = previousTestMarker;
  }
});

describe("source document imports", () => {
  it("registers a new encrypted file and reports an exact duplicate without duplicating the row", () => {
    const database = openDatabase();
    const repository = new SourceDocumentRepository(database);

    expect(repository.registerImport(sourceImport())).toEqual({
      documentId: "document-july",
      status: "imported",
    });
    expect(
      repository.registerImport(
        sourceImport({
          id: "ignored-duplicate-id",
          encryptedLocator: "files/aa/ignored.ccenv",
          audit: {
            ...sourceImport().audit,
            id: "audit-duplicate-july",
          },
        }),
      ),
    ).toEqual({ documentId: "document-july", status: "already_present" });

    expect(repository.listForMoneySource("source-dbs")).toHaveLength(1);
    expect(
      database.prepare("SELECT action FROM audit_log ORDER BY rowid").all(),
    ).toEqual([
      { action: "manual_import_imported" },
      { action: "manual_import_already_present" },
    ]);
  });

  it("restores an exact-hash tombstone and preserves its prior deletion audit", () => {
    const database = openDatabase();
    const repository = new SourceDocumentRepository(database);
    repository.registerImport(sourceImport());
    database
      .prepare(`
        INSERT INTO audit_log(
          id, entity_type, entity_id, action, actor, reason, policy_version
        ) VALUES (
          'audit-delete-july', 'source_document', 'document-july',
          'source_file_deletion_decided', 'user', 'user_requested', 'manual-import-v1'
        )
      `)
      .run();
    database
      .prepare(`
        UPDATE source_documents
        SET file_state = 'deleted', encrypted_locator = NULL,
            deleted_at = '2026-07-16T00:00:00Z', deletion_audit_id = 'audit-delete-july'
        WHERE id = 'document-july'
      `)
      .run();

    expect(
      repository.registerImport(
        sourceImport({
          id: "ignored-restored-id",
          encryptedLocator: "files/aa/restored.ccenv",
          audit: {
            ...sourceImport().audit,
            id: "audit-restore-july",
          },
        }),
      ),
    ).toEqual({ documentId: "document-july", status: "restored" });

    expect(repository.listForMoneySource("source-dbs")[0]).toMatchObject({
      id: "document-july",
      encryptedLocator: "files/aa/restored.ccenv",
      fileState: "available",
    });
    expect(
      database.prepare("SELECT action FROM audit_log WHERE id = 'audit-delete-july'").get(),
    ).toEqual({ action: "source_file_deletion_decided" });
  });

  it("retains byte-different evidence under the same semantic statement identity", () => {
    const database = openDatabase();
    const repository = new SourceDocumentRepository(database);
    repository.registerImport(sourceImport());

    expect(
      repository.registerImport(
        sourceImport({
          id: "document-july-rescanned",
          fileSha256: "b".repeat(64),
          originalFilename: "DBS-July-2026-rescanned.pdf",
          encryptedLocator: "files/bb/document-july-rescanned.ccenv",
          audit: {
            ...sourceImport().audit,
            id: "audit-import-july-rescanned",
          },
        }),
      ),
    ).toEqual({
      documentId: "document-july-rescanned",
      status: "probable_existing_statement",
    });
    expect(repository.listForMoneySource("source-dbs")).toHaveLength(2);
  });

  it("rejects an available file row without an encrypted locator", () => {
    const database = openDatabase();
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
        .run("c".repeat(64)),
    ).toThrow(/invalid source document file lifecycle/);
  });
});
