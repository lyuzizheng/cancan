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

function openFullyMigratedDatabase(): DatabaseSync {
  process.env.CANCAN_TEST = "1";
  const directory = mkdtempSync(join(tmpdir(), "cancan-test-schema-review-"));
  testDirectories.push(directory);
  const databasePath = join(directory, "schema-review.sqlite");
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
    { version: 9, name: "0009_post_pr41_hardening.sql" },
    { version: 10, name: "0010_schema_review_hardening.sql" },
  ].map(({ version, name }) => ({
    version,
    sql: migration(name),
    foreignKeysOff: version === 3 || version === 4 || version === 6 || version === 9,
  }));
  applyMigrations(database, migrations);
  // Idempotency: second run is a no-op
  applyMigrations(database, migrations);

  database
    .prepare(
      "INSERT INTO money_sources(id, provider_key, display_name, source_type) VALUES ('src-1', 'test-bank', 'Test Bank', 'bank')",
    )
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

describe("0010 schema review hardening", () => {
  describe("account self-merge guard", () => {
    it("rejects INSERT of an account that merges into itself", () => {
      const database = openFullyMigratedDatabase();
      expect(() =>
        database
          .prepare(
            `INSERT INTO accounts(
              id, money_source_id, provider_key, account_type, display_name,
              status, merged_into_account_id
            ) VALUES ('acct-self', 'src-1', 'test-bank', 'checking', 'Self', 'merged', 'acct-self')`,
          )
          .run(),
      ).toThrow(/account cannot merge into itself/);
    });

    it("rejects UPDATE that sets merged_into_account_id to own id", () => {
      const database = openFullyMigratedDatabase();
      database
        .prepare(
          `INSERT INTO accounts(
            id, money_source_id, provider_key, account_type, display_name, status
          ) VALUES ('acct-a', 'src-1', 'test-bank', 'checking', 'Account A', 'confirmed')`,
        )
        .run();
      expect(() =>
        database
          .prepare("UPDATE accounts SET merged_into_account_id = 'acct-a', status = 'merged' WHERE id = 'acct-a'")
          .run(),
      ).toThrow(/account cannot merge into itself/);
    });

    it("allows merging into a different account", () => {
      const database = openFullyMigratedDatabase();
      database
        .prepare(
          `INSERT INTO accounts(
            id, money_source_id, provider_key, account_type, display_name, status
          ) VALUES ('acct-b', 'src-1', 'test-bank', 'checking', 'Account B', 'confirmed')`,
        )
        .run();
      database
        .prepare(
          `INSERT INTO accounts(
            id, money_source_id, provider_key, account_type, display_name, status
          ) VALUES ('acct-c', 'src-1', 'test-bank', 'savings', 'Account C', 'confirmed')`,
        )
        .run();
      expect(() =>
        database
          .prepare("UPDATE accounts SET merged_into_account_id = 'acct-c', status = 'merged' WHERE id = 'acct-b'")
          .run(),
      ).not.toThrow();
    });
  });

  describe("indexes exist", () => {
    it("creates audit_log_entity_lookup index", () => {
      const database = openFullyMigratedDatabase();
      const row = database
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'audit_log_entity_lookup'")
        .get();
      expect(row).toBeDefined();
    });

    it("creates review_items_open partial index", () => {
      const database = openFullyMigratedDatabase();
      const row = database
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'review_items_open'")
        .get();
      expect(row).toBeDefined();
    });

    it("creates ledger_events_observation_lookup index", () => {
      const database = openFullyMigratedDatabase();
      const row = database
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'ledger_events_observation_lookup'")
        .get();
      expect(row).toBeDefined();
    });
  });

  describe("match_edges.created_at", () => {
    it("has created_at column with default timestamp", () => {
      const database = openFullyMigratedDatabase();
      const info = database.prepare("PRAGMA table_info(match_edges)").all();
      const createdAt = info.find((col) => col.name === "created_at");
      expect(createdAt).toBeDefined();
      expect(createdAt!.type).toBe("TEXT");
      expect(createdAt!.dflt_value).toBe("CURRENT_TIMESTAMP");
    });
  });

  describe("audit_log index is used", () => {
    it("EXPLAIN QUERY PLAN uses audit_log_entity_lookup", () => {
      const database = openFullyMigratedDatabase();
      const plan = database
        .prepare(
          "EXPLAIN QUERY PLAN SELECT action, actor, reason, policy_version FROM audit_log WHERE entity_type = 'ledger_event' AND entity_id = 'x' ORDER BY created_at, id",
        )
        .all();
      const planText = JSON.stringify(plan);
      expect(planText).toContain("audit_log_entity_lookup");
    });
  });
});
