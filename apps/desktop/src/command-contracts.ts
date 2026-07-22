export type VaultStatus = "not_created" | "locked" | "unlocked";

export type SavedStatementPasswordResult =
  | "invalid"
  | "unavailable"
  | "unlocked";

export interface VaultAccessStatus {
  recoveryConfigured: boolean;
  rememberedOnThisMac: boolean | null;
  status: VaultStatus;
}

export type VaultPasswordArgs = { password: string };

export type NormalizeSourceDocumentArgs = { documentId: string };

export type DeleteSourceDocumentArgs = { documentId: string };

export type DocumentStatementPasswordArgs = {
  documentId: string;
  moneySourceId: string;
  password: string;
  updateSavedPassword: boolean;
};

export type TrySavedStatementPasswordArgs = {
  documentId: string;
  moneySourceId: string;
};

export type RemoveStatementPasswordArgs = { moneySourceId: string };

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
  documentStatus:
    | "inspection_failed"
    | "password_required"
    | "protected_unlocked"
    | "ready"
    | "unavailable";
  fileState: "available" | "deleted" | "missing";
  mimeType: "application/pdf" | "text/csv";
  originalFilename: string;
  receivedAt: string;
}

export interface StatementPasswordSourceSummary {
  displayName: string;
  hasSavedPassword: boolean;
  moneySourceId: string;
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
