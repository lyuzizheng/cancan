// This file is generated from Rust wire types. Do not edit.

export type VaultAccessStatus = { recoveryConfigured: boolean, rememberedOnThisMac: boolean | null, status: VaultStatus, };

export type VaultStatus = "not_created" | "locked" | "unlocked";

export type SavedStatementPasswordResult = "invalid" | "unavailable" | "unlocked";

export type LocalInboxAccessState = "disabled" | "enabled" | "needs_attention" | "needs_reauthorization" | "paused";

export type LocalInboxScanSummary = { alreadyPresent: number, deferred: number, imported: number, suppressed: number, };

export type LocalInboxStatus = { accessState: LocalInboxAccessState, backupsPrepared: boolean, enabled: boolean, inboxLabel: string, lastScan: LocalInboxScanSummary | null, };

export type SourceDocumentImportOutcome = { documentId: string, status: SourceDocumentImportStatus, };

export type SourceDocumentImportStatus = "imported" | "already_present" | "restored" | "restore_confirmation_required";

export type MoneySourceSummary = { displayName: string, moneySourceId: string, sourceType: string, };

export type SourceDocumentPreview = { lineCount: number, previewLines: number, previewText: string, truncated: boolean, };

export type SourceDocumentStatus = "file_deleted" | "missing" | "needs_attention" | "processing" | "ready";

export type SourceDocumentSummary = { attentionReason: string | null, byteSize: number, documentStatus: SourceDocumentStatus, documentId: string, fileState: string, mimeType: string, originalFilename: string, receivedAt: string, };

export type StatementPasswordSourceSummary = { displayName: string, hasSavedPassword: boolean, moneySourceId: string, };

export type RenderedDocumentPage = { pageCount: number, pageNumber: number, pngBase64: string, };

export type OperationalDiagnosticsCategory = { label: string, count: number, };

export type OperationalDiagnosticsPreview = { appVersion: string, retentionDays: number,
/**
 * Retained entries in the Vault.
 */
entryCount: number,
/**
 * The most entries one export can contain.
 */
exportLimit: number, oldestEntryAt: string | null, newestEntryAt: string | null, components: Array<OperationalDiagnosticsCategory>, errorCodes: Array<OperationalDiagnosticsCategory>, sampleLines: Array<string>, };

export type IntakeNotificationPermission = "not_determined" | "authorized" | "denied";

export type IntakeNotificationSettings = { enabled: boolean, permission: IntakeNotificationPermission, };

export type PhoneShortcutInboxCheck = { state: PhoneShortcutInboxCheckState,
/**
 * The failure code, present exactly when `state` is `failed`.
 */
reason: string | null, };

export type PhoneShortcutInboxCheckState = "never_checked" | "passed" | "failed";

export type PhoneShortcutStatus = {
/**
 * The artifact version this build installs.
 */
version: number,
/**
 * The name Shortcuts shows for it.
 */
name: string, inboxCheck: PhoneShortcutInboxCheck, };

export type TaskConsequence = "processing" | "password_needed" | "new_source_detected" | "needs_review" | "restore_source_file" | "inbox_file_could_not_be_added" | "import_interrupted" | "needs_attention" | "file_not_added" | "already_in_cancan" | "same_statement_content" | "source_file_restored" | "source_file_left_deleted" | "ready" | "source_unassigned" | "password_parked" | "inbox_file_parked" | "save_recovery_file" | "setup_reminder_postponed";

export type TaskDestination = { "kind": "document", documentId: string, } | { "kind": "password", documentId: string, moneySourceId: string, } | { "kind": "source_confirmation", moneySourceCandidateId: string, } | { "kind": "review_group", documentId: string, } | { "kind": "receipt", intakeItemId: string, } | { "kind": "inbox_issue", intakeItemId: string, } | { "kind": "recovery_setup" };

export type TaskFilter = "command_center" | "full";

export type TaskGroup = "needs_action" | "in_progress" | "recently_completed" | "parked";

export type TaskRow = { rowKey: string, group: TaskGroup, title: string, consequence: TaskConsequence, timestamp: string, destination: TaskDestination, };

export type Tasks = { needsActionCount: number, rows: Array<TaskRow>, };

export type AccountConfirmationCandidate = { accountId: string, accountType: string, currency: string | null, displayName: string, maskedIdentifier: string | null, };

export type AccountConfirmationOutcome = { status: AccountConfirmationStatus, };

export type AccountConfirmationPrompt = { candidateAccounts: Array<AccountConfirmationCandidate>, dismissedAccounts: Array<AccountConfirmationCandidate>, displayName: string, moneySourceId: string, proposalVersion: string, };

export type AccountConfirmationStatus = "already_confirmed" | "confirmed" | "conflict" | "restored" | "updated";

export type CandidateAccountDecision = "accept" | "dismiss";

export type CandidateAccountDecisionInput = { accountId: string, action: CandidateAccountDecision, };

export type DecideCandidateAccountsRequest = { decisions: Array<CandidateAccountDecisionInput>, moneySourceId: string, proposalVersion: string, };

export type ConfirmedMoneySourceCandidate = { candidateId: string, moneySourceId: string, version: number, };

export type MoneySourceCandidateState = { candidateId: string, confirmedMoneySourceId: string | null, status: MoneySourceCandidateStatus, version: number, };

export type MoneySourceCandidateStatus = "confirmed" | "kept_unassigned" | "pending";

export type SourceConfirmationPrompt = { candidateId: string, documentCount: number, latestDocumentId: string | null, latestDocumentTitle: string | null, providerKey: string, scopeKind: SourceConfirmationScopeKind, status: SourceConfirmationPromptStatus, version: number, };

export type SourceConfirmationPromptStatus = "pending" | "kept_unassigned";

export type SourceConfirmationScopeKind = "provider_singleton" | "provider_root_id";

export type ConfirmSourceCandidateRequest = { candidateId: string, displayName: string, expectedVersion: number, sourceType: string, };

export type ParkSourceCandidateRequest = { candidateId: string, expectedVersion: number, };

export type DuplicateCommittedVersionAuditRow = { stableRecordKey: string, externalRecordId: string, sourceDocumentId: string, version: number, versionRank: number, recordEventType: string | null, postedOn: string | null, amountValue: string | null, currency: string | null, ledgerEventId: string | null, ledgerEventType: string | null, ledgerEventDate: string | null, ledgerEventStatus: string | null, ledgerEventIsReversal: boolean, ledgerEventHasReversal: boolean, allocationValue: string | null, matchUnit: string | null, matchReviewStatus: string | null, reversalSafe: boolean, };

export type MoneyOverview = { assets: Array<MoneyOverviewAmount>, liabilities: Array<MoneyOverviewAmount>, };

export type MoneyOverviewAmount = { accountId: string, accountLabel: string, asOf: string, currency: string, value: string, };

export type RecentActivitySummary = { canUndo: boolean, eventDate: string, eventId: string, eventType: string, sourceLabels: Array<string>, spending: boolean, };

export type RelationshipCandidateSummary = { accountLabel: string, amountValue: string, currency: string, eventType: string, postedOn: string, recordId: string, recordVersion: number, };

export type ReviewBatchGroupOutcome = { reason: string | null, recordIds: Array<string>, status: ReviewBatchGroupStatus, };

export type ReviewBatchGroupStatus = "already_committed" | "committed" | "stale" | "still_needs_review";

export type ReviewItemDetail = { accountLabel: string, amountValue: string | null, currency: string | null, documentLabel: string, eventType: string | null, postedOn: string | null, reasonCode: string, recordCommitted: boolean, recordId: string, recordVersion: number, reviewItemId: string, sourceLabel: string, };

export type ReviewItemSummary = { accountLabel: string, amountValue: string | null, currency: string | null, eventType: string | null, postedOn: string | null, reasonCode: string, recordCommitted: boolean, recordId: string, recordVersion: number, reviewItemId: string, };

export type ReviewJobStatus = "blocked" | "cancelled" | "failed" | "queued" | "running" | "succeeded";

export type ReviewJobSummary = { createdAt: string, finishedAt: string | null, jobId: string, outcomes: Array<ReviewBatchGroupOutcome>, status: ReviewJobStatus, };

export type ReviewMutationOutcome = { reason: string | null, recordVersion: number | null, reviewItemId: string | null, status: ReviewMutationStatus, };

export type ReviewMutationStatus = "acknowledged" | "conflict" | "relationship_accepted" | "removed" | "updated";

export type UndoOutcome = { eventId: string, status: UndoStatus, };

export type UndoStatus = "already_undone" | "undone";
