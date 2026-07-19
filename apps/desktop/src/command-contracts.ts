export type VaultStatus = "not_created" | "locked" | "unlocked";

export type VaultPasswordArgs = { password: string };

export type NormalizeSourceDocumentArgs = { documentId: string };

export type DeleteSourceDocumentArgs = { documentId: string };

export type RenderSourceDocumentPageArgs = {
  documentId: string;
  pageNumber: number;
};

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

export interface RenderedDocumentPage {
  pageCount: number;
  pageNumber: number;
  pngBase64: string;
}
