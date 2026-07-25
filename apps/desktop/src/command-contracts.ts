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

export type LocalInboxAccessState =
  | "disabled"
  | "enabled"
  | "needs_attention"
  | "needs_reauthorization"
  | "paused";

export interface LocalInboxScanSummary {
  alreadyPresent: number;
  deferred: number;
  imported: number;
  suppressed: number;
}

export interface LocalInboxStatus {
  accessState: LocalInboxAccessState;
  backupsPrepared: boolean;
  enabled: boolean;
  inboxLabel: "Inbox";
  lastScan: LocalInboxScanSummary | null;
}

export interface StatementCoveragePrompt {
  accountId: string;
  documentType: string;
  moneySourceId: string;
  statementPeriodFrom: string;
  statementPeriodTo: string;
  status: "confirmed_missing" | "likely_missing";
}

export type StatementCoverageDecisionArgs = {
  accountId: string;
  action: "not_expected" | "remind_later";
  documentType: string;
  moneySourceId: string;
  remindAfter?: string;
  statementPeriodFrom: string;
  statementPeriodTo: string;
};

export type RecordStatementCoverageDecisionArgs = {
  request: StatementCoverageDecisionArgs;
};

export type VaultPasswordArgs = { password: string };

export type NormalizeSourceDocumentArgs = { documentId: string };

export type DeleteSourceDocumentArgs = { documentId: string };

export type SaveSourceDocumentCopyArgs = { documentId: string };

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

export type PreviewSourceDocumentArgs = { documentId: string };

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
  mimeType: "application/pdf" | "text/csv" | "image/png" | "image/jpeg";
  originalFilename: string;
  receivedAt: string;
}

export interface MoneySourceSummary {
  displayName: string;
  moneySourceId: string;
  sourceType: string;
}

export type ListSourceDocumentsArgs = { moneySourceId: string };

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

export interface SourceDocumentPreview {
  lineCount: number;
  previewLines: number;
  previewText: string;
  truncated: boolean;
}

export interface ReviewItemSummary {
  accountLabel: string;
  amountValue: string | null;
  currency: string | null;
  eventType: string | null;
  postedOn: string | null;
  reasonCode: string;
  recordId: string;
  recordVersion: number;
  reviewItemId: string;
}

export interface ReviewItemDetail extends ReviewItemSummary {
  documentLabel: string;
  sourceLabel: string;
}

export interface RecentActivitySummary {
  canUndo: boolean;
  eventDate: string;
  eventId: string;
  eventType: string;
  sourceLabels: string[];
  spending: boolean;
}

export interface MoneyOverviewAmount {
  accountId: string;
  accountLabel: string;
  asOf: string;
  currency: string;
  value: string;
}

export interface MoneyOverview {
  assets: MoneyOverviewAmount[];
  liabilities: MoneyOverviewAmount[];
}

export interface RelationshipCandidateSummary {
  accountLabel: string;
  amountValue: string;
  currency: string;
  eventType: string;
  postedOn: string;
  recordId: string;
  recordVersion: number;
}

export type ReviewMutationStatus =
  | "conflict"
  | "relationship_accepted"
  | "removed"
  | "updated";

export interface ReviewMutationOutcome {
  reason: string | null;
  recordVersion: number | null;
  reviewItemId: string | null;
  status: ReviewMutationStatus;
}

export type ReviewJobStatus =
  | "blocked"
  | "cancelled"
  | "failed"
  | "queued"
  | "running"
  | "succeeded";

export type ReviewBatchGroupStatus =
  | "already_committed"
  | "committed"
  | "stale"
  | "still_needs_review";

export interface ReviewBatchGroupOutcome {
  reason: string | null;
  recordIds: string[];
  status: ReviewBatchGroupStatus;
}

export interface ReviewJobSummary {
  createdAt: string;
  finishedAt: string | null;
  jobId: string;
  outcomes: ReviewBatchGroupOutcome[];
  status: ReviewJobStatus;
}

export type UndoStatus = "already_undone" | "undone";

export interface UndoOutcome {
  eventId: string;
  status: UndoStatus;
}

export type ReviewItemIdArgs = { reviewItemId: string };

export type ReviewVersionArgs = ReviewItemIdArgs & {
  expectedRecordVersion: number;
};

export type EditReviewRecordArgs = ReviewVersionArgs & {
  accountBalanceDelta?: string;
  amountValue?: string;
  postedOn?: string;
};

export type AcceptReviewRelationshipArgs = ReviewVersionArgs & {
  candidateRecordId: string;
  expectedCandidateVersion: number;
};

export type EnqueueCommitReviewBatchArgs = { reviewItemIds: string[] };

export type GetReviewJobArgs = { jobId: string };

export type UndoCommittedEventArgs = { eventId: string };
