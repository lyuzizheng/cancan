import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";

import {
  prepareBalanceObservation,
  prepareSameCurrencyTransfer,
  type RecordEligibility,
  type TransferPreparation,
} from "@cancan/core";
import {
  validateStructuredProposal,
  type StructuredProposalValidation,
} from "@cancan/parsers";
import {
  createSyntheticTransferFixture,
  syntheticBankRecordContract,
} from "@cancan/parsers/testing";
import { afterEach, describe, expect, it } from "vitest";

import {
  applyMigrations,
  type StagedImport,
  SyntheticCoreRepository,
} from "./index";
import { resetTestDatabase } from "./testing";

type ValidatedProposal = Extract<StructuredProposalValidation, { status: "valid" }>;

const qualifiedRecord: RecordEligibility = {
  sourceConfigured: true,
  providerVerified: true,
  profileQualified: true,
  schemaValid: true,
  rawGrounded: true,
  deterministicValidationPassed: true,
  accountResolved: true,
  duplicateFree: true,
  allocationComplete: true,
  veryHighConfidence: true,
  requiredLegsPresent: true,
};

const testDirectories: string[] = [];
const testDatabases: DatabaseSync[] = [];
let previousTestMarker: string | undefined;

function openMigratedTestDatabase(): DatabaseSync {
  previousTestMarker = process.env.CANCAN_TEST;
  process.env.CANCAN_TEST = "1";
  const directory = mkdtempSync(join(tmpdir(), "cancan-test-"));
  testDirectories.push(directory);
  const databasePath = join(directory, "synthetic-core.sqlite");
  resetTestDatabase(databasePath);

  const database = new DatabaseSync(databasePath);
  testDatabases.push(database);
  const migrationSql = readFileSync(
    new URL("../migrations/0001_synthetic_core.sql", import.meta.url),
    "utf8",
  );
  applyMigrations(database, [{ version: 1, sql: migrationSql }]);
  applyMigrations(database, [{ version: 1, sql: migrationSql }]);
  return database;
}

async function parseSyntheticTransfer(): Promise<ValidatedProposal> {
  const parsed = await validateStructuredProposal({
    ...createSyntheticTransferFixture(),
    recordContract: syntheticBankRecordContract,
  });
  if (parsed.status !== "valid") {
    throw new Error("synthetic proposal did not validate");
  }
  return parsed;
}

function prepareSyntheticTransfer(
  parsed: ValidatedProposal,
  autoCommitEnabled: boolean,
): TransferPreparation {
  const openingByAccount = new Map(
    parsed.openingSnapshots.map((record) => [record.proposalAccountId, record]),
  );
  const closingByAccount = new Map(
    parsed.closingSnapshots.map((record) => [record.proposalAccountId, record]),
  );
  const snapshot = (
    records: Map<string | undefined, ValidatedProposal["openingSnapshots"][number]>,
    accountId: string,
  ) => {
    const record = records.get(accountId);
    if (!record?.balanceAfter) {
      throw new Error(`synthetic fixture is missing a balance snapshot for ${accountId}`);
    }
    return {
      recordId: record.proposalRecordId,
      value: record.balanceAfter.value,
      eligibility: qualifiedRecord,
    };
  };

  return prepareSameCurrencyTransfer({
    autoCommitEnabled,
    commitIdempotencyKey: "synthetic-transfer-v1",
    records: parsed.records.map((record) => ({
      id: record.proposalRecordId,
      accountId: record.proposalAccountId ?? "",
      instrumentId: "instrument-sgd",
      postedOn: record.postedOn ?? "",
      currency: record.accountBalanceDelta?.currency ?? "",
      accountBalanceDelta: record.accountBalanceDelta?.value ?? "",
      eligibility: qualifiedRecord,
    })),
    reconciliationWindows: [
      {
        accountId: "account-checking",
        currency: "SGD",
        opening: snapshot(openingByAccount, "account-checking"),
        closing: snapshot(closingByAccount, "account-checking"),
      },
      {
        accountId: "account-savings",
        currency: "SGD",
        opening: snapshot(openingByAccount, "account-savings"),
        closing: snapshot(closingByAccount, "account-savings"),
      },
    ],
  });
}

function stagedTransfer(
  parsed: ValidatedProposal,
  status: "staged" | "review",
  reviewItems: StagedImport["reviewItems"] = [],
): StagedImport {
  return {
    moneySource: {
      id: "source-synthetic",
      providerKey: "synthetic-bank",
      displayName: "Synthetic Bank",
      sourceType: "bank",
    },
    accounts: parsed.accounts.map((account) => ({
      id: account.proposalAccountId,
      moneySourceId: "source-synthetic",
      providerKey: "synthetic-bank",
      providerAccountId: account.providerAccountId,
      accountType: account.accountType,
      displayName: account.proposalAccountId === "account-checking" ? "Checking" : "Savings",
      maskedIdentifier: account.maskedIdentifier,
      currency: account.currency,
      status: "confirmed" as const,
    })),
    instruments: [
      {
        id: "instrument-sgd",
        instrumentType: "fiat_currency",
        symbol: "SGD",
        currency: "SGD",
        displayName: "Singapore Dollar",
      },
    ],
    document: {
      id: "document-transfer",
      moneySourceId: "source-synthetic",
      fileSha256: "b".repeat(64),
      semanticDocumentKey: "synthetic:transfer:2026-07",
    },
    parseRun: {
      id: "parse-run-1",
      sourceDocumentId: "document-transfer",
      normalizationProfileId: "synthetic-profile-v1",
      profile: { runtime: "stored-mock" },
      status: "validated",
    },
    records: [
      ...parsed.openingSnapshots,
      ...parsed.records,
      ...parsed.closingSnapshots,
    ].map((record) => ({
      id: record.proposalRecordId,
      parseRunId: "parse-run-1",
      sourceDocumentId: "document-transfer",
      accountId: record.proposalAccountId,
      stableRecordKey: record.stableRecordKey,
      version: 1,
      status,
      recordType: record.recordType,
      eventType: record.eventType,
      postedOn: record.postedOn,
      amountValue: record.amount?.value,
      currency: record.amount?.currency,
      accountBalanceDelta: record.accountBalanceDelta?.value,
      raw: record.raw,
      validation: record.validation,
    })),
    reviewItems,
  };
}

function commitBalanceObservations(
  repository: SyntheticCoreRepository,
  parsed: ValidatedProposal,
): void {
  for (const record of [...parsed.openingSnapshots, ...parsed.closingSnapshots]) {
    if (!record.proposalAccountId || !record.postedOn || !record.balanceAfter) {
      throw new Error("synthetic fixture contains an incomplete balance snapshot");
    }
    const prepared = prepareBalanceObservation({
      autoCommitEnabled: true,
      commitIdempotencyKey: `balance:${record.proposalRecordId}`,
      record: {
        id: record.proposalRecordId,
        accountId: record.proposalAccountId,
        instrumentId: "instrument-sgd",
        observedOn: record.postedOn,
        currency: record.balanceAfter.currency,
        balanceValue: record.balanceAfter.value,
        eligibility: qualifiedRecord,
      },
    });
    if (prepared.status !== "ready") {
      throw new Error("synthetic balance observation was not ready");
    }
    repository.commitPreparedEvent({
      id: `event-${record.proposalRecordId}`,
      event: prepared.event,
      audit: {
        id: `audit-${record.proposalRecordId}`,
        actor: "auto_policy",
        reason: "qualified_source_observation",
        policyVersion: "synthetic-policy-v1",
      },
    });
  }
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

describe("synthetic core flow", () => {
  it("stages deterministic records, commits one transfer idempotently, and navigates both source sides", async () => {
    const database = openMigratedTestDatabase();
    const parsed = await parseSyntheticTransfer();
    const prepared = prepareSyntheticTransfer(parsed, true);
    expect(prepared.status).toBe("ready");
    if (prepared.status !== "ready") {
      throw new Error("synthetic transfer was not ready");
    }

    const repository = new SyntheticCoreRepository(database);
    repository.stageImport(stagedTransfer(parsed, "staged"));
    expect(() =>
      repository.commitPreparedEvent({
        id: "event-invalid-cardinality",
        event: {
          ...prepared.event,
          sourceRecordIds: [...prepared.event.sourceRecordIds, "record-checking-opening"],
        },
        audit: {
          id: "audit-invalid-cardinality",
          actor: "auto_policy",
          reason: "invalid_cardinality",
          policyVersion: "synthetic-policy-v1",
        },
      }),
    ).toThrow("every ledger leg requires exactly one source record");
    commitBalanceObservations(repository, parsed);
    expect(() =>
      repository.commitPreparedEvent({
        id: "event-transfer-rolled-back",
        event: {
          ...prepared.event,
          commitIdempotencyKey: "synthetic-transfer-audit-failure",
        },
        audit: {
          id: "audit-record-checking-opening",
          actor: "auto_policy",
          reason: "duplicate_audit_id",
          policyVersion: "synthetic-policy-v1",
        },
      }),
    ).toThrow();
    expect(repository.findRecordRelationships("record-checking-out")).toEqual([]);
    expect(repository.findExternalRecord("record-checking-out")).toMatchObject({
      status: "staged",
    });

    const firstCommit = repository.commitPreparedEvent({
      id: "event-transfer-1",
      event: prepared.event,
      audit: {
        id: "audit-transfer-1",
        actor: "auto_policy",
        reason: "qualified_exact_reconciliation",
        policyVersion: "synthetic-policy-v1",
      },
    });
    const repeatedCommit = repository.commitPreparedEvent({
      id: "event-transfer-retry",
      event: prepared.event,
      audit: {
        id: "audit-transfer-retry",
        actor: "auto_policy",
        reason: "retry",
        policyVersion: "synthetic-policy-v1",
      },
    });

    expect(firstCommit).toEqual({ eventId: "event-transfer-1", created: true });
    expect(repeatedCommit).toEqual({ eventId: "event-transfer-1", created: false });
    expect(() =>
      repository.commitPreparedEvent({
        id: "event-transfer-duplicate",
        event: {
          ...prepared.event,
          commitIdempotencyKey: "synthetic-transfer-different-key",
        },
        audit: {
          id: "audit-transfer-duplicate",
          actor: "auto_policy",
          reason: "different_key_same_records",
          policyVersion: "synthetic-policy-v1",
        },
      }),
    ).toThrow("source record already has a confirmed allocation");
    expect(repository.findExternalRecord("record-checking-out")).toMatchObject({
      id: "record-checking-out",
      status: "committed",
      raw: {
        date: "2026-07-01",
        description: "Transfer to savings",
        debit: "250.00",
        locator: { row: 2 },
      },
      validation: {
        schemaValid: true,
        rawGrounded: true,
        deterministicValidationPassed: true,
      },
    });
    expect(repository.findLatestBalanceObservation("account-checking", "SGD")).toEqual({
      balanceValue: "750.00",
      observedOn: "2026-07-01",
      sourceRecordId: "record-checking-closing",
    });
    expect(repository.findRecordRelationships("record-checking-out")).toEqual([
      {
        eventId: "event-transfer-1",
        eventType: "same_currency_transfer",
        sourceRecords: [
          { id: "record-checking-out", accountId: "account-checking" },
          { id: "record-savings-in", accountId: "account-savings" },
        ],
      },
    ]);
    expect(repository.findRecordRelationships("record-savings-in")).toEqual(
      repository.findRecordRelationships("record-checking-out"),
    );
    expect(repository.findEventSourceRecords("event-transfer-1")).toEqual([
      { id: "record-checking-out", accountId: "account-checking" },
      { id: "record-savings-in", accountId: "account-savings" },
    ]);
    expect(repository.findAuditEntries("ledger_event", "event-transfer-1")).toEqual([
      {
        action: "commit",
        actor: "auto_policy",
        reason: "qualified_exact_reconciliation",
        policyVersion: "synthetic-policy-v1",
      },
    ]);
    expect(repository.findRecordRelationships("record-checking-out")).toHaveLength(1);

    const recordLookupPlan = database
      .prepare(`
        EXPLAIN QUERY PLAN
        SELECT ledger_event_id FROM match_edges WHERE external_record_id = ?
      `)
      .all("record-checking-out")
      .map((row) => String(row.detail));
    const eventLookupPlan = database
      .prepare(`
        EXPLAIN QUERY PLAN
        SELECT external_record_id FROM match_edges WHERE ledger_event_id = ?
      `)
      .all("event-transfer-1")
      .map((row) => String(row.detail));
    expect(recordLookupPlan.join(" ")).toContain("SEARCH match_edges");
    expect(eventLookupPlan.join(" ")).toContain("SEARCH match_edges");
    expect([...recordLookupPlan, ...eventLookupPlan].join(" ")).not.toContain("SCAN match_edges");

    expect(() =>
      database
        .prepare("UPDATE ledger_events SET event_date = '2026-07-02' WHERE id = ?")
        .run("event-transfer-1"),
    ).toThrow("committed ledger event is immutable");
    expect(() =>
      database
        .prepare("UPDATE ledger_legs SET amount_value = '-249.00' WHERE ledger_event_id = ?")
        .run("event-transfer-1"),
    ).toThrow("committed ledger event legs are immutable");
    expect(() =>
      database
        .prepare("UPDATE external_records SET raw_json = '{}' WHERE id = ?")
        .run("record-checking-out"),
    ).toThrow("committed external record is immutable");
    expect(() =>
      database.prepare("DELETE FROM external_records WHERE id = ?").run("record-checking-out"),
    ).toThrow("committed external record is immutable");
    expect(() =>
      database
        .prepare(`
          INSERT INTO match_edges(
            external_record_id, ledger_event_id, allocation_value, unit,
            review_status, unmatched_remainder_value
          ) VALUES (?, ?, '250.00', 'SGD', 'confirmed', '0')
        `)
        .run("record-checking-out", "event-transfer-1"),
    ).toThrow("committed match edge is immutable");
    expect(() =>
      database
        .prepare("UPDATE match_edges SET allocation_value = '249.00' WHERE external_record_id = ?")
        .run("record-checking-out"),
    ).toThrow("committed match edge is immutable");
    expect(() =>
      database
        .prepare("DELETE FROM match_edges WHERE external_record_id = ?")
        .run("record-checking-out"),
    ).toThrow("committed match edge is immutable");
    expect(() =>
      database.prepare("DELETE FROM audit_log WHERE id = ?").run("audit-transfer-1"),
    ).toThrow("audit log is append-only");
  });

  it("stages qualified records for review when automatic commit is disabled", async () => {
    const database = openMigratedTestDatabase();
    const parsed = await parseSyntheticTransfer();
    expect(prepareSyntheticTransfer(parsed, false)).toEqual({
      status: "review",
      reasons: ["auto_commit_disabled"],
    });

    const repository = new SyntheticCoreRepository(database);
    const invalidStage = stagedTransfer(parsed, "staged");
    (invalidStage.records[0] as { status: string }).status = "committed";
    expect(() => repository.stageImport(invalidStage)).toThrow(
      "stageImport only accepts staged or review records",
    );
    expect(database.prepare("SELECT COUNT(*) AS count FROM money_sources").get()).toMatchObject({
      count: 0,
    });
    repository.stageImport(
      stagedTransfer(parsed, "review", [
        {
          id: "review-checking",
          externalRecordId: "record-checking-out",
          reasonCode: "auto_commit_disabled",
        },
        {
          id: "review-savings",
          externalRecordId: "record-savings-in",
          reasonCode: "auto_commit_disabled",
        },
      ]),
    );

    expect(repository.findOpenReviewItems()).toEqual([
      {
        id: "review-checking",
        externalRecordId: "record-checking-out",
        reasonCode: "auto_commit_disabled",
      },
      {
        id: "review-savings",
        externalRecordId: "record-savings-in",
        reasonCode: "auto_commit_disabled",
      },
    ]);
    expect(repository.findRecordRelationships("record-checking-out")).toEqual([]);
  });
});
