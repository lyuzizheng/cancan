import type { PreparedLedgerEvent } from "@cancan/core";

import { inTransaction, type SqliteDatabase, type SqlValue } from "./sqlite";

type AccountStatus = "candidate" | "confirmed" | "archived" | "merged";
type ExternalRecordStatus = "staged" | "review" | "committed" | "superseded";
type StagedRecordStatus = Extract<ExternalRecordStatus, "staged" | "review">;

export interface StagedImport {
  moneySource: {
    id: string;
    providerKey: string;
    displayName: string;
    sourceType: string;
  };
  accounts: Array<{
    id: string;
    moneySourceId: string;
    providerKey: string;
    providerAccountId?: string;
    accountType: string;
    displayName: string;
    maskedIdentifier?: string;
    currency?: string;
    status: AccountStatus;
    mergedIntoAccountId?: string;
    rawIdentity?: Record<string, unknown>;
  }>;
  instruments: Array<{
    id: string;
    instrumentType: string;
    symbol: string;
    currency?: string;
    displayName: string;
  }>;
  document: {
    id: string;
    moneySourceId: string;
    fileSha256: string;
    semanticDocumentKey: string;
  };
  parseRun: {
    id: string;
    sourceDocumentId: string;
    normalizationProfileId: string;
    profile: Record<string, unknown>;
    status: string;
  };
  records: Array<{
    id: string;
    parseRunId: string;
    sourceDocumentId: string;
    accountId?: string;
    stableRecordKey: string;
    version: number;
    status: StagedRecordStatus;
    recordType: string;
    eventType?: string;
    postedOn?: string;
    amountValue?: string;
    currency?: string;
    accountBalanceDelta?: string;
    raw: Record<string, unknown>;
    validation: Record<string, unknown>;
  }>;
  reviewItems: Array<{
    id: string;
    externalRecordId: string;
    reasonCode: string;
  }>;
}

export interface RecordRelationship {
  eventId: string;
  eventType: string;
  sourceRecords: Array<{ id: string; accountId: string }>;
}

export interface AuditEntry {
  action: string;
  actor: string;
  reason: string;
  policyVersion: string;
}

export interface ReviewItem {
  id: string;
  externalRecordId: string;
  reasonCode: string;
}

export interface ExternalRecordView {
  id: string;
  status: ExternalRecordStatus;
  raw: Record<string, unknown>;
  validation: Record<string, unknown>;
}

export interface BalanceObservationView {
  balanceValue: string;
  observedOn: string;
  sourceRecordId: string;
}

function nullable(value: SqlValue | undefined): SqlValue {
  return value ?? null;
}

function stringColumn(row: Record<string, unknown>, column: string): string {
  const value = row[column];
  if (typeof value !== "string") {
    throw new Error(`expected ${column} to be text`);
  }
  return value;
}

function magnitude(value: string): string {
  return value.startsWith("-") ? value.slice(1) : value;
}

function jsonColumn(row: Record<string, unknown>, column: string): Record<string, unknown> {
  const parsed: unknown = JSON.parse(stringColumn(row, column));
  if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") {
    throw new Error(`expected ${column} to contain a JSON object`);
  }
  return parsed as Record<string, unknown>;
}

export class SyntheticCoreRepository {
  public constructor(private readonly database: SqliteDatabase) {}

  public stageImport(input: StagedImport): void {
    if (input.records.some(({ status }) => status !== "staged" && status !== "review")) {
      throw new Error("stageImport only accepts staged or review records");
    }
    inTransaction(this.database, () => {
      this.database
        .prepare(
          "INSERT INTO money_sources(id, provider_key, display_name, source_type) VALUES (?, ?, ?, ?)",
        )
        .run(
          input.moneySource.id,
          input.moneySource.providerKey,
          input.moneySource.displayName,
          input.moneySource.sourceType,
        );

      const insertAccount = this.database.prepare(`
        INSERT INTO accounts(
          id, money_source_id, provider_key, provider_account_id, account_type,
          display_name, masked_identifier, currency, status, merged_into_account_id, raw_identity_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `);
      for (const account of input.accounts) {
        insertAccount.run(
          account.id,
          account.moneySourceId,
          account.providerKey,
          nullable(account.providerAccountId),
          account.accountType,
          account.displayName,
          nullable(account.maskedIdentifier),
          nullable(account.currency),
          account.status,
          nullable(account.mergedIntoAccountId),
          account.rawIdentity ? JSON.stringify(account.rawIdentity) : null,
        );
      }

      const insertInstrument = this.database.prepare(`
        INSERT INTO instruments(id, instrument_type, symbol, currency, display_name)
        VALUES (?, ?, ?, ?, ?)
      `);
      for (const instrument of input.instruments) {
        insertInstrument.run(
          instrument.id,
          instrument.instrumentType,
          instrument.symbol,
          nullable(instrument.currency),
          instrument.displayName,
        );
      }

      this.database
        .prepare(`
          INSERT INTO source_documents(id, money_source_id, file_sha256, semantic_document_key)
          VALUES (?, ?, ?, ?)
        `)
        .run(
          input.document.id,
          input.document.moneySourceId,
          input.document.fileSha256,
          input.document.semanticDocumentKey,
        );

      this.database
        .prepare(`
          INSERT INTO parse_runs(
            id, source_document_id, normalization_profile_id, profile_json, status
          ) VALUES (?, ?, ?, ?, ?)
        `)
        .run(
          input.parseRun.id,
          input.parseRun.sourceDocumentId,
          input.parseRun.normalizationProfileId,
          JSON.stringify(input.parseRun.profile),
          input.parseRun.status,
        );

      const insertRecord = this.database.prepare(`
        INSERT INTO external_records(
          id, parse_run_id, source_document_id, account_id, stable_record_key, version,
          status, record_type, event_type, posted_on, amount_value, currency,
          account_balance_delta, raw_json, validation_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `);
      for (const record of input.records) {
        insertRecord.run(
          record.id,
          record.parseRunId,
          record.sourceDocumentId,
          nullable(record.accountId),
          record.stableRecordKey,
          record.version,
          record.status,
          record.recordType,
          nullable(record.eventType),
          nullable(record.postedOn),
          nullable(record.amountValue),
          nullable(record.currency),
          nullable(record.accountBalanceDelta),
          JSON.stringify(record.raw),
          JSON.stringify(record.validation),
        );
      }

      const insertReview = this.database.prepare(`
        INSERT INTO review_items(id, external_record_id, reason_code, status)
        VALUES (?, ?, ?, 'open')
      `);
      for (const reviewItem of input.reviewItems) {
        insertReview.run(reviewItem.id, reviewItem.externalRecordId, reviewItem.reasonCode);
      }
    });
  }

  public commitPreparedEvent(input: {
    id: string;
    event: PreparedLedgerEvent;
    audit: {
      id: string;
      actor: string;
      reason: string;
      policyVersion: string;
    };
  }): { eventId: string; created: boolean } {
    return inTransaction(this.database, () => {
      const existing = this.database
        .prepare("SELECT id FROM ledger_events WHERE commit_idempotency_key = ?")
        .get(input.event.commitIdempotencyKey);
      if (existing) {
        return { eventId: stringColumn(existing, "id"), created: false };
      }
      if (input.event.legs.length !== input.event.sourceRecordIds.length) {
        throw new Error("every ledger leg requires exactly one source record");
      }
      const confirmedAllocation = this.database.prepare(`
        SELECT 1
        FROM match_edges
        WHERE external_record_id = ? AND review_status = 'confirmed'
        LIMIT 1
      `);
      for (const sourceRecordId of input.event.sourceRecordIds) {
        if (confirmedAllocation.get(sourceRecordId)) {
          throw new Error("source record already has a confirmed allocation");
        }
      }

      this.database
        .prepare(`
          INSERT INTO ledger_events(
            id, event_type, event_class, event_date, status, commit_idempotency_key
          ) VALUES (?, ?, ?, ?, 'pending', ?)
        `)
        .run(
          input.id,
          input.event.eventType,
          input.event.eventClass,
          input.event.eventDate,
          input.event.commitIdempotencyKey,
        );

      const insertLeg = this.database.prepare(`
        INSERT INTO ledger_legs(
          id, ledger_event_id, account_id, instrument_id, amount_value, balance_value, currency
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
      `);
      const insertEdge = this.database.prepare(`
        INSERT INTO match_edges(
          external_record_id, ledger_event_id, allocation_value, unit,
          review_status, unmatched_remainder_value
        ) VALUES (?, ?, ?, ?, 'confirmed', '0')
      `);
      const markCommitted = this.database.prepare(
        "UPDATE external_records SET status = 'committed' WHERE id = ?",
      );

      input.event.legs.forEach((leg, index) => {
        const sourceRecordId = input.event.sourceRecordIds[index] as string;
        const amountValue = "amountValue" in leg ? leg.amountValue : undefined;
        const balanceValue = "balanceValue" in leg ? leg.balanceValue : undefined;
        const linkedValue = amountValue ?? balanceValue;
        if (!linkedValue) {
          throw new Error("every ledger leg requires a source-backed value");
        }
        insertLeg.run(
          `${input.id}:leg:${index + 1}`,
          input.id,
          leg.accountId,
          leg.instrumentId,
          nullable(amountValue),
          nullable(balanceValue),
          leg.currency,
        );
        insertEdge.run(
          sourceRecordId,
          input.id,
          magnitude(linkedValue),
          leg.currency,
        );
        markCommitted.run(sourceRecordId);
      });

      this.database
        .prepare(`
          INSERT INTO audit_log(
            id, entity_type, entity_id, action, actor, reason, source_ref, policy_version
          ) VALUES (?, 'ledger_event', ?, 'commit', ?, ?, ?, ?)
        `)
        .run(
          input.audit.id,
          input.id,
          input.audit.actor,
          input.audit.reason,
          JSON.stringify(input.event.sourceRecordIds),
          input.audit.policyVersion,
        );
      this.database
        .prepare("UPDATE ledger_events SET status = 'committed' WHERE id = ?")
        .run(input.id);

      return { eventId: input.id, created: true };
    });
  }

  public findEventSourceRecords(eventId: string): Array<{ id: string; accountId: string }> {
    return this.database
      .prepare(`
        SELECT external_records.id, external_records.account_id
        FROM match_edges
        JOIN external_records ON external_records.id = match_edges.external_record_id
        WHERE match_edges.ledger_event_id = ?
        ORDER BY external_records.id
      `)
      .all(eventId)
      .map((row) => ({
        id: stringColumn(row, "id"),
        accountId: stringColumn(row, "account_id"),
      }));
  }

  public findExternalRecord(externalRecordId: string): ExternalRecordView | undefined {
    const row = this.database
      .prepare(`
        SELECT id, status, raw_json, validation_json
        FROM external_records
        WHERE id = ?
      `)
      .get(externalRecordId);
    if (!row) {
      return undefined;
    }
    return {
      id: stringColumn(row, "id"),
      status: stringColumn(row, "status") as ExternalRecordStatus,
      raw: jsonColumn(row, "raw_json"),
      validation: jsonColumn(row, "validation_json"),
    };
  }

  public findLatestBalanceObservation(
    accountId: string,
    currency: string,
  ): BalanceObservationView | undefined {
    const row = this.database
      .prepare(`
        SELECT
          ledger_legs.balance_value,
          ledger_events.event_date,
          match_edges.external_record_id
        FROM ledger_legs
        JOIN ledger_events ON ledger_events.id = ledger_legs.ledger_event_id
        JOIN match_edges ON match_edges.ledger_event_id = ledger_events.id
        WHERE ledger_legs.account_id = ?
          AND ledger_legs.currency = ?
          AND ledger_legs.balance_value IS NOT NULL
          AND ledger_events.event_class = 'observation'
          AND ledger_events.status = 'committed'
        ORDER BY ledger_events.event_date DESC, ledger_events.created_at DESC
        LIMIT 1
      `)
      .get(accountId, currency);
    if (!row) {
      return undefined;
    }
    return {
      balanceValue: stringColumn(row, "balance_value"),
      observedOn: stringColumn(row, "event_date"),
      sourceRecordId: stringColumn(row, "external_record_id"),
    };
  }

  public findRecordRelationships(externalRecordId: string): RecordRelationship[] {
    return this.database
      .prepare(`
        SELECT ledger_events.id, ledger_events.event_type
        FROM match_edges
        JOIN ledger_events ON ledger_events.id = match_edges.ledger_event_id
        WHERE match_edges.external_record_id = ?
        ORDER BY ledger_events.id
      `)
      .all(externalRecordId)
      .map((row) => {
        const eventId = stringColumn(row, "id");
        return {
          eventId,
          eventType: stringColumn(row, "event_type"),
          sourceRecords: this.findEventSourceRecords(eventId),
        };
      });
  }

  public findAuditEntries(entityType: string, entityId: string): AuditEntry[] {
    return this.database
      .prepare(`
        SELECT action, actor, reason, policy_version
        FROM audit_log
        WHERE entity_type = ? AND entity_id = ?
        ORDER BY created_at, id
      `)
      .all(entityType, entityId)
      .map((row) => ({
        action: stringColumn(row, "action"),
        actor: stringColumn(row, "actor"),
        reason: stringColumn(row, "reason"),
        policyVersion: stringColumn(row, "policy_version"),
      }));
  }

  public findOpenReviewItems(): ReviewItem[] {
    return this.database
      .prepare(`
        SELECT id, external_record_id, reason_code
        FROM review_items
        WHERE status = 'open'
        ORDER BY id
      `)
      .all()
      .map((row) => ({
        id: stringColumn(row, "id"),
        externalRecordId: stringColumn(row, "external_record_id"),
        reasonCode: stringColumn(row, "reason_code"),
      }));
  }
}
