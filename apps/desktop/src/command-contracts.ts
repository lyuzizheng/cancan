/**
 * The renderer's Tauri command contract.
 *
 * Every Rust type that crosses the command bridge is generated from its Rust
 * definition into `./generated/presentation-types.ts` — the harness is
 * `src-tauri/src/presentation_types.rs` and the `check:presentation-types` gate
 * fails when that file goes stale — and re-exported below. Nothing here mirrors
 * a Rust type by hand, so a Rust field, variant, or rename cannot drift away
 * from the renderer unnoticed.
 *
 * What stays hand-written are the `*Args` payloads. Tauri deserializes a
 * command's flat parameters directly out of the invoke object, so
 * `{ documentId }` has no Rust type to generate from; only the three commands
 * that already take a single request struct embed a generated type.
 */

import type {
  ConfirmSourceCandidateRequest,
  DecideCandidateAccountsRequest,
  ParkSourceCandidateRequest,
  TaskFilter,
} from "./generated/presentation-types";

export type {
  AccountConfirmationCandidate,
  AccountConfirmationOutcome,
  AccountConfirmationPrompt,
  AccountConfirmationStatus,
  CandidateAccountDecision,
  CandidateAccountDecisionInput,
  ConfirmedMoneySourceCandidate,
  ConfirmSourceCandidateRequest,
  DecideCandidateAccountsRequest,
  DuplicateCommittedVersionAuditRow,
  LocalInboxAccessState,
  LocalInboxScanSummary,
  LocalInboxStatus,
  MoneyOverview,
  MoneyOverviewAmount,
  MoneySourceCandidateState,
  MoneySourceCandidateStatus,
  MoneySourceSummary,
  ParkSourceCandidateRequest,
  RecentActivitySummary,
  RelationshipCandidateSummary,
  RenderedDocumentPage,
  ReviewBatchGroupOutcome,
  ReviewBatchGroupStatus,
  ReviewItemDetail,
  ReviewItemSummary,
  ReviewJobStatus,
  ReviewJobSummary,
  ReviewMutationOutcome,
  ReviewMutationStatus,
  SavedStatementPasswordResult,
  SourceConfirmationPrompt,
  SourceConfirmationPromptStatus,
  SourceConfirmationScopeKind,
  SourceDocumentImportOutcome,
  SourceDocumentImportStatus,
  SourceDocumentPreview,
  SourceDocumentStatus,
  SourceDocumentSummary,
  StatementPasswordSourceSummary,
  TaskConsequence,
  TaskDestination,
  TaskFilter,
  TaskGroup,
  TaskRow,
  Tasks,
  UndoOutcome,
  UndoStatus,
  VaultAccessStatus,
  VaultStatus,
} from "./generated/presentation-types";

export type VaultPasswordArgs = { password: string };

export type ReparseSourceDocumentArgs = { documentId: string };

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

export type CloseSourceDocumentViewArgs = { documentId: string };

export type DecideCandidateAccountsArgs = {
  request: DecideCandidateAccountsRequest;
};

export type RestoreDismissedCandidateAccountArgs = { accountId: string };

export type ConfirmSourceCandidateArgs = {
  request: ConfirmSourceCandidateRequest;
};

export type ParkSourceCandidateArgs = {
  request: ParkSourceCandidateRequest;
};

export type ListSourceDocumentsArgs = { moneySourceId: string };

export type ListTasksArgs = { filter: TaskFilter };

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
