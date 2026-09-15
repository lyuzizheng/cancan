import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  AccountConfirmationOutcome,
  AccountConfirmationPrompt,
  AcceptReviewRelationshipArgs,
  CandidateAccountDecisionInput,
  ConfirmSourceCandidateArgs,
  ConfirmedMoneySourceCandidate,
  DecideCandidateAccountsArgs,
  EditReviewRecordArgs,
  EnqueueCommitReviewBatchArgs,
  GetReviewJobArgs,
  DeleteSourceDocumentArgs,
  ListSourceDocumentsArgs,
  ListTasksArgs,
  TaskFilter,
  LocalInboxScanSummary,
  LocalInboxStatus,
  MoneyOverview,
  MoneySourceCandidateState,
  MoneySourceSummary,
  ParkSourceCandidateArgs,
  ReparseSourceDocumentArgs,
  PreviewSourceDocumentArgs,
  RecentActivitySummary,
  RemoveStatementPasswordArgs,
  ReviewItemDetail,
  ReviewItemIdArgs,
  ReviewItemSummary,
  ReviewJobSummary,
  ReviewMutationOutcome,
  RenderedDocumentPage,
  RenderSourceDocumentPageArgs,
  RelationshipCandidateSummary,
  SavedStatementPasswordResult,
  SaveSourceDocumentCopyArgs,
  SourceConfirmationPrompt,
  SourceDocumentImportOutcome,
  SourceDocumentPreview,
  SourceDocumentSummary,
  Tasks,
  DocumentStatementPasswordArgs,
  RestoreDismissedCandidateAccountArgs,
  StatementPasswordSourceSummary,
  TrySavedStatementPasswordArgs,
  UndoCommittedEventArgs,
  UndoOutcome,
  VaultAccessStatus,
  VaultPasswordArgs,
  VaultStatus,
  ReviewVersionArgs,
} from "./command-contracts";

export type TauriInvoke = <
  Result,
  Args extends Record<string, unknown> | undefined = undefined,
>(
  command: string,
  args?: Args,
) => Promise<Result>;

export type TauriListen = (
  event: string,
  handler: () => void,
) => Promise<() => void>;

export interface VaultApi {
  acceptReviewRelationship(
    reviewItemId: string,
    expectedRecordVersion: number,
    candidateRecordId: string,
    expectedCandidateVersion: number,
  ): Promise<ReviewMutationOutcome>;
  decideCandidateAccounts(
    moneySourceId: string,
    proposalVersion: string,
    decisions: CandidateAccountDecisionInput[],
  ): Promise<AccountConfirmationOutcome>;
  confirmSourceCandidate(
    candidateId: string,
    expectedVersion: number,
    displayName: string,
    sourceType: string,
  ): Promise<ConfirmedMoneySourceCandidate>;
  parkSourceCandidate(
    candidateId: string,
    expectedVersion: number,
  ): Promise<MoneySourceCandidateState>;
  createVault(password: string): Promise<VaultStatus>;
  chooseLocalInboxRoot(): Promise<LocalInboxStatus | null>;
  deleteSourceDocument(documentId: string): Promise<boolean>;
  disableLocalInbox(): Promise<LocalInboxStatus>;
  editReviewRecord(
    reviewItemId: string,
    expectedRecordVersion: number,
    patch: Omit<EditReviewRecordArgs, "expectedRecordVersion" | "reviewItemId">,
  ): Promise<ReviewMutationOutcome>;
  enqueueCommitReviewBatch(reviewItemIds: string[]): Promise<ReviewJobSummary>;
  forgetVaultOnThisMac(): Promise<void>;
  getMoneyOverview(): Promise<MoneyOverview>;
  getReviewDetail(reviewItemId: string): Promise<ReviewItemDetail | null>;
  getReviewJob(jobId: string): Promise<ReviewJobSummary | null>;
  importSourceDocument(): Promise<SourceDocumentImportOutcome | null>;
  listMoneySources(): Promise<MoneySourceSummary[]>;
  listAccountConfirmationPrompts(): Promise<AccountConfirmationPrompt[]>;
  listSourceConfirmationPrompts(): Promise<SourceConfirmationPrompt[]>;
  listRecentActivity(): Promise<RecentActivitySummary[]>;
  listRelationshipCandidates(
    reviewItemId: string,
    expectedRecordVersion: number,
  ): Promise<RelationshipCandidateSummary[]>;
  listReviewItems(): Promise<ReviewItemSummary[]>;
  listSourceDocuments(moneySourceId: string): Promise<SourceDocumentSummary[]>;
  listStatementPasswordSources(): Promise<StatementPasswordSourceSummary[]>;
  listTasks(filter: TaskFilter): Promise<Tasks>;
  listUnassignedSourceDocuments(): Promise<SourceDocumentSummary[]>;
  lockVault(): Promise<VaultStatus>;
  localInboxStatus(): Promise<LocalInboxStatus>;
  onVaultLocked(handler: () => void): Promise<() => void>;
  previewSourceDocument(documentId: string): Promise<SourceDocumentPreview>;
  reparseSourceDocument(documentId: string): Promise<void>;
  rememberVaultOnThisMac(): Promise<void>;
  restoreDismissedCandidateAccount(accountId: string): Promise<AccountConfirmationOutcome>;
  removeReviewRecord(
    reviewItemId: string,
    expectedRecordVersion: number,
  ): Promise<ReviewMutationOutcome>;
  removeStatementPassword(moneySourceId: string): Promise<void>;
  renderSourceDocumentPage(
    documentId: string,
    pageNumber: number,
  ): Promise<RenderedDocumentPage>;
  rescanLocalInbox(): Promise<LocalInboxScanSummary>;
  saveRecoveryFile(): Promise<boolean>;
  saveSourceDocumentCopy(documentId: string): Promise<boolean>;
  trySavedStatementPassword(
    documentId: string,
    moneySourceId: string,
  ): Promise<SavedStatementPasswordResult>;
  unlockSourceDocument(
    documentId: string,
    moneySourceId: string,
    password: string,
    updateSavedPassword: boolean,
  ): Promise<void>;
  unlockVault(password: string): Promise<VaultStatus>;
  unlockVaultWithKeychain(): Promise<VaultStatus>;
  undoCommittedEvent(eventId: string): Promise<UndoOutcome>;
  vaultAccessStatus(): Promise<VaultAccessStatus>;
  vaultStatus(): Promise<VaultStatus>;
}

const tauriInvoke: TauriInvoke = (command, args) =>
  invoke(command, args as Record<string, unknown> | undefined);
const tauriListen: TauriListen = (event, handler) => listen(event, handler);

export function createVaultApi(
  call: TauriInvoke = tauriInvoke,
  subscribe: TauriListen = tauriListen,
): VaultApi {
  return {
    vaultAccessStatus: () => call<VaultAccessStatus>("vault_access_status"),
    vaultStatus: () => call<VaultStatus>("vault_status"),
    listReviewItems: () => call<ReviewItemSummary[]>("list_review_items"),
    getReviewDetail: (reviewItemId) => {
      const args: ReviewItemIdArgs = { reviewItemId };
      return call<ReviewItemDetail | null, ReviewItemIdArgs>(
        "get_review_detail",
        args,
      );
    },
    listRecentActivity: () =>
      call<RecentActivitySummary[]>("list_recent_activity"),
    getMoneyOverview: () => call<MoneyOverview>("get_money_overview"),
    listRelationshipCandidates: (reviewItemId, expectedRecordVersion) => {
      const args: ReviewVersionArgs = { reviewItemId, expectedRecordVersion };
      return call<RelationshipCandidateSummary[], ReviewVersionArgs>(
        "list_relationship_candidates",
        args,
      );
    },
    editReviewRecord: (reviewItemId, expectedRecordVersion, patch) => {
      const args: EditReviewRecordArgs = {
        reviewItemId,
        expectedRecordVersion,
        ...patch,
      };
      return call<ReviewMutationOutcome, EditReviewRecordArgs>(
        "edit_review_record",
        args,
      );
    },
    removeReviewRecord: (reviewItemId, expectedRecordVersion) => {
      const args: ReviewVersionArgs = { reviewItemId, expectedRecordVersion };
      return call<ReviewMutationOutcome, ReviewVersionArgs>(
        "remove_review_record",
        args,
      );
    },
    acceptReviewRelationship: (
      reviewItemId,
      expectedRecordVersion,
      candidateRecordId,
      expectedCandidateVersion,
    ) => {
      const args: AcceptReviewRelationshipArgs = {
        reviewItemId,
        expectedRecordVersion,
        candidateRecordId,
        expectedCandidateVersion,
      };
      return call<ReviewMutationOutcome, AcceptReviewRelationshipArgs>(
        "accept_review_relationship",
        args,
      );
    },
    decideCandidateAccounts: (moneySourceId, proposalVersion, decisions) => {
      const args: DecideCandidateAccountsArgs = {
        request: { moneySourceId, proposalVersion, decisions },
      };
      return call<AccountConfirmationOutcome, DecideCandidateAccountsArgs>(
        "decide_candidate_accounts",
        args,
      );
    },
    restoreDismissedCandidateAccount: (accountId) => {
      const args: RestoreDismissedCandidateAccountArgs = { accountId };
      return call<AccountConfirmationOutcome, RestoreDismissedCandidateAccountArgs>(
        "restore_dismissed_candidate_account",
        args,
      );
    },
    reparseSourceDocument: (documentId) => {
      const args: ReparseSourceDocumentArgs = { documentId };
      return call<void, ReparseSourceDocumentArgs>("reparse_source_document", args);
    },
    enqueueCommitReviewBatch: (reviewItemIds) => {
      const args: EnqueueCommitReviewBatchArgs = { reviewItemIds };
      return call<ReviewJobSummary, EnqueueCommitReviewBatchArgs>(
        "enqueue_commit_review_batch",
        args,
      );
    },
    getReviewJob: (jobId) => {
      const args: GetReviewJobArgs = { jobId };
      return call<ReviewJobSummary | null, GetReviewJobArgs>(
        "get_review_job",
        args,
      );
    },
    undoCommittedEvent: (eventId) => {
      const args: UndoCommittedEventArgs = { eventId };
      return call<UndoOutcome, UndoCommittedEventArgs>(
        "undo_committed_event",
        args,
      );
    },
    createVault: (password) => {
      const args: VaultPasswordArgs = { password };
      return call<VaultStatus, VaultPasswordArgs>("create_vault", args);
    },
    chooseLocalInboxRoot: () =>
      call<LocalInboxStatus | null>("choose_local_inbox_root"),
    localInboxStatus: () => call<LocalInboxStatus>("local_inbox_status"),
    disableLocalInbox: () => call<LocalInboxStatus>("disable_local_inbox"),
    rescanLocalInbox: () =>
      call<LocalInboxScanSummary>("rescan_local_inbox"),
    unlockVault: (password) => {
      const args: VaultPasswordArgs = { password };
      return call<VaultStatus, VaultPasswordArgs>("unlock_vault", args);
    },
    unlockVaultWithKeychain: () =>
      call<VaultStatus>("unlock_vault_with_keychain"),
    rememberVaultOnThisMac: () => call<void>("remember_vault_on_this_mac"),
    forgetVaultOnThisMac: () => call<void>("forget_vault_on_this_mac"),
    listStatementPasswordSources: () =>
      call<StatementPasswordSourceSummary[]>("list_statement_password_sources"),
    listTasks: (filter: TaskFilter) => {
      const args: ListTasksArgs = { filter };
      return call<Tasks, ListTasksArgs>("list_tasks", args);
    },
    trySavedStatementPassword: (documentId, moneySourceId) => {
      const args: TrySavedStatementPasswordArgs = {
        documentId,
        moneySourceId,
      };
      return call<SavedStatementPasswordResult, TrySavedStatementPasswordArgs>(
        "try_saved_statement_password",
        args,
      );
    },
    unlockSourceDocument: (
      documentId,
      moneySourceId,
      password,
      updateSavedPassword,
    ) => {
      const args: DocumentStatementPasswordArgs = {
        documentId,
        moneySourceId,
        password,
        updateSavedPassword,
      };
      return call<void, DocumentStatementPasswordArgs>(
        "unlock_source_document",
        args,
      );
    },
    removeStatementPassword: (moneySourceId) => {
      const args: RemoveStatementPasswordArgs = { moneySourceId };
      return call<void, RemoveStatementPasswordArgs>(
        "remove_statement_password",
        args,
      );
    },
    lockVault: () => call<VaultStatus>("lock_vault"),
    saveRecoveryFile: () => call<boolean>("save_recovery_file"),
    saveSourceDocumentCopy: (documentId) => {
      const args: SaveSourceDocumentCopyArgs = { documentId };
      return call<boolean, SaveSourceDocumentCopyArgs>(
        "save_source_document_copy",
        args,
      );
    },
    deleteSourceDocument: (documentId) => {
      const args: DeleteSourceDocumentArgs = { documentId };
      return call<boolean, DeleteSourceDocumentArgs>(
        "delete_source_document",
        args,
      );
    },
    importSourceDocument: () =>
      call<SourceDocumentImportOutcome | null>("import_source_document"),
    listMoneySources: () => call<MoneySourceSummary[]>("list_money_sources"),
    listAccountConfirmationPrompts: () =>
      call<AccountConfirmationPrompt[]>("list_account_confirmation_prompts"),
    listSourceConfirmationPrompts: () =>
      call<SourceConfirmationPrompt[]>("list_source_confirmation_prompts"),
    confirmSourceCandidate: (
      candidateId,
      expectedVersion,
      displayName,
      sourceType,
    ) => {
      const args: ConfirmSourceCandidateArgs = {
        request: { candidateId, displayName, expectedVersion, sourceType },
      };
      return call<ConfirmedMoneySourceCandidate, ConfirmSourceCandidateArgs>(
        "confirm_source_candidate",
        args,
      );
    },
    parkSourceCandidate: (candidateId, expectedVersion) => {
      const args: ParkSourceCandidateArgs = {
        request: { candidateId, expectedVersion },
      };
      return call<MoneySourceCandidateState, ParkSourceCandidateArgs>(
        "park_source_candidate",
        args,
      );
    },
    listSourceDocuments: (moneySourceId) => {
      const args: ListSourceDocumentsArgs = { moneySourceId };
      return call<SourceDocumentSummary[], ListSourceDocumentsArgs>(
        "list_source_documents",
        args,
      );
    },
    listUnassignedSourceDocuments: () =>
      call<SourceDocumentSummary[]>("list_unassigned_source_documents"),
    onVaultLocked: (handler) => subscribe("vault-locked", handler),
    previewSourceDocument: (documentId) => {
      const args: PreviewSourceDocumentArgs = { documentId };
      return call<SourceDocumentPreview, PreviewSourceDocumentArgs>(
        "preview_source_document",
        args,
      );
    },
    renderSourceDocumentPage: (documentId, pageNumber) => {
      const args: RenderSourceDocumentPageArgs = { documentId, pageNumber };
      return call<RenderedDocumentPage, RenderSourceDocumentPageArgs>(
        "render_source_document_page",
        args,
      );
    },
  };
}

export function commandErrorMessage(error: unknown): string {
  switch (commandErrorCode(error)) {
    case "invalid_credentials":
      return "That password did not unlock this Vault.";
    case "password_required":
      return "Enter a password to continue.";
    case "remembered_unlock_unavailable":
      return "Touch ID unlock is no longer available. Use your Vault password instead.";
    case "remembered_unlock_failed":
      return "CanCan couldn’t unlock with Touch ID.";
    case "remember_failed":
      return "CanCan couldn’t enable Touch ID unlock.";
    case "forget_failed":
      return "CanCan couldn’t disable Touch ID unlock.";
    case "statement_password_required":
      return "Enter the statement password to continue.";
    case "statement_password_save_failed":
      return "CanCan couldn’t save that statement password in this Mac’s Keychain.";
    case "statement_password_load_failed":
      return "CanCan couldn’t access the saved statement password in this Mac’s Keychain.";
    case "statement_password_invalid":
      return "That password did not unlock this statement.";
    case "statement_password_remove_failed":
      return "CanCan couldn’t remove that statement password from this Mac’s Keychain.";
    case "vault_locked":
      return "Unlock your Vault to continue.";
    case "vault_not_created":
      return "Create your Vault before adding evidence.";
    case "recovery_already_configured":
      return "A recovery file is already configured for this Vault.";
    case "recovery_location_invalid":
      return "Save the recovery file somewhere outside your CanCan Vault.";
    case "recovery_create_failed":
      return "CanCan couldn’t create a recovery file safely.";
    case "recovery_save_failed":
      return "CanCan couldn’t save the recovery file to that location.";
    case "recovery_status_failed":
      return "The recovery file was saved, but CanCan couldn’t record setup. Keep the file private and try again.";
    case "source_copy_location_invalid":
      return "Save the copy somewhere outside your CanCan Vault.";
    case "source_copy_save_failed":
      return "CanCan couldn’t save a complete copy to that location.";
    case "unsupported_document":
      return "Choose a PDF, CSV, PNG, or JPEG file.";
    case "normalizer_failed":
      return "CanCan could not finish the secure document check. Try again.";
    case "parse_already_running":
      return "The parser is already running for this document.";
    case "parse_resume_failed":
      return "CanCan couldn’t resume parsing this statement. Try again.";
    case "document_unavailable":
      return "This file is no longer available.";
    case "invalid_document_request":
      return "That document request isn’t valid.";
    case "viewer_unsupported":
      return "Preview isn’t available for this evidence.";
    case "document_render_failed":
      return "CanCan couldn’t render that document.";
    case "delete_source_failed":
      return "CanCan couldn’t finish removing this Vault file. Refresh its status before trying again.";
    case "local_inbox_not_configured":
      return "Set up your CanCan Inbox folder first.";
    case "local_inbox_setup_required":
      return "CanCan Inbox needs attention before it can check for new files.";
    case "local_inbox_setup_failed":
      return "CanCan couldn’t prepare Inbox and Backups in that folder. CanCan never changes existing files.";
    case "local_inbox_reauthorization_required":
      return "Choose your Cancan folder again so CanCan can reach it.";
    case "local_inbox_storage_failed":
    case "local_inbox_unavailable":
      return "CanCan couldn’t reach the local Inbox setup. Try again.";
    case "file_selection_failed":
      return "CanCan couldn’t use that folder choice. Try again.";
    case "account_confirmation_unavailable":
      return "CanCan couldn’t confirm those accounts. Try again.";
    case "invalid_account_confirmation_request":
      return "That account confirmation isn’t valid.";
    case "source_confirmation_unavailable":
      return "CanCan couldn’t update that money source confirmation. Try again.";
    case "invalid_source_confirmation_request":
      return "That money source confirmation isn’t valid.";
    case "list_tasks_failed":
      return "Couldn’t load your tasks. Try again.";
    case "clock_error":
      return "Couldn’t read the system clock. Try again.";
    case "import_failed":
      return "CanCan couldn’t import that file. Try again.";
    case "review_unavailable":
      return "CanCan couldn’t load the review queue. Try again.";
    case "invalid_review_request":
      return "That review change isn’t valid.";
    case "invalid_source_request":
      return "That money source request isn’t valid.";
    case "statement_password_state_invalid":
      return "The saved statement password no longer matches. Enter it again.";
    case "runtime_unavailable":
      return "CanCan is busy finishing another task. Try again in a moment.";
    case "undo_unavailable":
      return "CanCan couldn’t undo that event. Try again.";
    case "document_not_protected":
      return "This document isn’t password-protected.";
    case "invalid_vault":
      return "That Vault isn’t valid.";
    case "vault_already_exists":
      return "A Vault already exists here.";
    case "vault_create_failed":
      return "CanCan couldn’t create the Vault. Try again.";
    case "review_core_failed":
      return "CanCan couldn’t finish the review check. Try again.";
    case "normalizer_unavailable":
      return "The secure document checker isn’t available. Try again.";
    case "local_inbox_authorization_failed":
      return "CanCan couldn’t get permission to reach the Inbox folder. Choose it again.";
    case "local_inbox_parse_failed":
      return "CanCan couldn’t read a file in the Inbox folder.";
    case "local_inbox_scan_failed":
      return "CanCan couldn’t scan the Inbox folder. Try again.";
    case "local_inbox_observation_failed":
      return "CanCan couldn’t record what changed in the Inbox folder.";
    case "local_inbox_import_failed":
      return "CanCan couldn’t import a file from the Inbox folder.";
    case "intake_recovery_failed":
      return "CanCan couldn’t finish recovering an earlier import.";
    case "intake_finalization_failed":
      return "CanCan couldn’t finish processing an imported file.";
    case "recovery_status_read_failed":
    case "recovery_status_write_failed":
      return "CanCan couldn’t update the recovery file status.";
    default:
      return "Couldn’t complete that request. Try again.";
  }
}

function commandErrorCode(error: unknown): string | null {
  if (typeof error === "object" && error !== null && "code" in error) {
    const { code } = error;
    return typeof code === "string" ? code : null;
  }

  if (typeof error !== "string") {
    return null;
  }

  try {
    const parsed: unknown = JSON.parse(error);
    return commandErrorCode(parsed);
  } catch {
    return null;
  }
}
