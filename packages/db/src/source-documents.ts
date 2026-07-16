import { inTransaction, type SqliteDatabase } from "./sqlite";

export type SourceDocumentFileState = "available" | "deleted" | "missing";
export type SourceDocumentImportStatus =
  | "imported"
  | "already_present"
  | "restored"
  | "probable_existing_statement";

export interface SourceDocumentImport {
  id: string;
  moneySourceId: string;
  fileSha256: string;
  semanticDocumentKey: string;
  originalFilename: string;
  mimeType: string;
  byteSize: number;
  encryptedLocator: string;
  audit: {
    id: string;
    actor: string;
    reason: string;
    policyVersion: string;
  };
}

export interface SourceDocumentImportOutcome {
  documentId: string;
  status: SourceDocumentImportStatus;
}

export interface SourceDocumentView {
  id: string;
  fileSha256: string;
  semanticDocumentKey: string;
  originalFilename: string;
  mimeType: string;
  byteSize: number;
  encryptedLocator?: string;
  fileState: SourceDocumentFileState;
  receivedAt: string;
}

interface ExistingDocument {
  id: string;
  fileState: SourceDocumentFileState;
}

function requiredString(value: unknown, column: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`expected ${column} to be non-empty text`);
  }
  return value;
}

function optionalString(value: unknown, column: string): string | undefined {
  if (value === null || value === undefined) {
    return undefined;
  }
  return requiredString(value, column);
}

function fileState(value: unknown): SourceDocumentFileState {
  if (value !== "available" && value !== "deleted" && value !== "missing") {
    throw new Error("expected file_state to contain a supported lifecycle state");
  }
  return value;
}

function integerColumn(value: unknown, column: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    throw new Error(`expected ${column} to be a safe integer`);
  }
  return value;
}

function validateImport(input: SourceDocumentImport): void {
  if (!/^[a-f0-9]{64}$/.test(input.fileSha256)) {
    throw new Error("fileSha256 must be a lowercase SHA-256 hex digest");
  }
  if (!Number.isSafeInteger(input.byteSize) || input.byteSize < 0) {
    throw new Error("byteSize must be a non-negative safe integer");
  }
  for (const [name, value] of [
    ["id", input.id],
    ["moneySourceId", input.moneySourceId],
    ["semanticDocumentKey", input.semanticDocumentKey],
    ["originalFilename", input.originalFilename],
    ["mimeType", input.mimeType],
    ["encryptedLocator", input.encryptedLocator],
    ["audit.id", input.audit.id],
    ["audit.actor", input.audit.actor],
    ["audit.reason", input.audit.reason],
    ["audit.policyVersion", input.audit.policyVersion],
  ] as const) {
    if (value.length === 0) {
      throw new Error(`${name} must not be empty`);
    }
  }
}

export class SourceDocumentRepository {
  public constructor(private readonly database: SqliteDatabase) {}

  public registerImport(input: SourceDocumentImport): SourceDocumentImportOutcome {
    validateImport(input);
    return inTransaction(this.database, () => {
      const exactRow = this.database
        .prepare("SELECT id, file_state FROM source_documents WHERE file_sha256 = ?")
        .get(input.fileSha256);
      if (exactRow) {
        const existing: ExistingDocument = {
          id: requiredString(exactRow.id, "id"),
          fileState: fileState(exactRow.file_state),
        };
        const status: SourceDocumentImportStatus =
          existing.fileState === "available" ? "already_present" : "restored";
        if (status === "restored") {
          this.database
            .prepare(`
              UPDATE source_documents
              SET encrypted_locator = ?, file_state = 'available',
                  deleted_at = NULL, deletion_audit_id = NULL
              WHERE id = ?
            `)
            .run(input.encryptedLocator, existing.id);
        }
        this.appendAudit(input, existing.id, status);
        return { documentId: existing.id, status };
      }

      const semanticMatch = this.database
        .prepare(`
          SELECT 1 FROM source_documents
          WHERE money_source_id = ? AND semantic_document_key = ?
          LIMIT 1
        `)
        .get(input.moneySourceId, input.semanticDocumentKey);
      const status: SourceDocumentImportStatus = semanticMatch
        ? "probable_existing_statement"
        : "imported";
      this.database
        .prepare(`
          INSERT INTO source_documents(
            id, money_source_id, file_sha256, semantic_document_key,
            original_filename, mime_type, byte_size, encrypted_locator, file_state
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'available')
        `)
        .run(
          input.id,
          input.moneySourceId,
          input.fileSha256,
          input.semanticDocumentKey,
          input.originalFilename,
          input.mimeType,
          input.byteSize,
          input.encryptedLocator,
        );
      this.appendAudit(input, input.id, status);
      return { documentId: input.id, status };
    });
  }

  public listForMoneySource(moneySourceId: string): SourceDocumentView[] {
    return this.database
      .prepare(`
        SELECT id, file_sha256, semantic_document_key, original_filename,
               mime_type, byte_size, encrypted_locator, file_state, received_at
        FROM source_documents
        WHERE money_source_id = ?
        ORDER BY received_at DESC, id
      `)
      .all(moneySourceId)
      .map((row) => ({
        id: requiredString(row.id, "id"),
        fileSha256: requiredString(row.file_sha256, "file_sha256"),
        semanticDocumentKey: requiredString(row.semantic_document_key, "semantic_document_key"),
        originalFilename: requiredString(row.original_filename, "original_filename"),
        mimeType: requiredString(row.mime_type, "mime_type"),
        byteSize: integerColumn(row.byte_size, "byte_size"),
        encryptedLocator: optionalString(row.encrypted_locator, "encrypted_locator"),
        fileState: fileState(row.file_state),
        receivedAt: requiredString(row.received_at, "received_at"),
      }));
  }

  private appendAudit(
    input: SourceDocumentImport,
    documentId: string,
    status: SourceDocumentImportStatus,
  ): void {
    this.database
      .prepare(`
        INSERT INTO audit_log(
          id, entity_type, entity_id, action, actor, reason, source_ref, policy_version
        ) VALUES (?, 'source_document', ?, ?, ?, ?, ?, ?)
      `)
      .run(
        input.audit.id,
        documentId,
        `manual_import_${status}`,
        input.audit.actor,
        input.audit.reason,
        input.fileSha256,
        input.audit.policyVersion,
      );
  }
}
