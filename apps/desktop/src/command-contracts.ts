export type ImportSourceDocumentArgs = Record<string, never>;

export interface NormalizeSourceDocumentArgs {
  documentId: string;
}

export interface SourceDocumentImportOutcome {
  documentId: string;
  status: "imported" | "already_present" | "restored";
}

export interface SourceDocumentSummary {
  byteSize: number;
  documentId: string;
  fileState: "available" | "deleted" | "missing";
  mimeType: "application/pdf" | "text/csv";
  originalFilename: string;
  receivedAt: string;
}

export interface SourceDocumentRoutingOutcome {
  accountIds: string[];
  documentId: string;
  moneySourceId: string | null;
  reason: string | null;
  status: "routed" | "needs_attention";
}
