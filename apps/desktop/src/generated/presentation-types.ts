// This file is generated from Rust presentation types. Do not edit.

export type SourceDocumentStatus = "file_deleted" | "missing" | "needs_attention" | "processing" | "ready";

export type SourceDocumentSummary = { attentionReason: string | null, byteSize: number, documentStatus: SourceDocumentStatus, documentId: string, fileState: string, mimeType: string, originalFilename: string, receivedAt: string, };

export type TaskFilter = "command_center" | "full";

export type TaskGroup = "needs_action" | "in_progress" | "recently_completed" | "parked";

export type TaskConsequence = "processing" | "password_needed" | "new_source_detected" | "needs_review" | "restore_source_file" | "inbox_file_could_not_be_added" | "import_interrupted" | "needs_attention" | "file_not_added" | "already_in_cancan" | "source_file_restored" | "source_file_left_deleted" | "ready" | "source_unassigned" | "password_parked" | "inbox_file_parked" | "save_recovery_file" | "setup_reminder_postponed";

export type TaskDestination = { "kind": "document", documentId: string, } | { "kind": "password", documentId: string, moneySourceId: string, } | { "kind": "source_confirmation", moneySourceCandidateId: string, } | { "kind": "review_group", documentId: string, } | { "kind": "receipt", intakeItemId: string, } | { "kind": "inbox_issue", intakeItemId: string, } | { "kind": "recovery_setup" };

export type TaskRow = { rowKey: string, group: TaskGroup, title: string, consequence: TaskConsequence, timestamp: string, destination: TaskDestination, };

export type Tasks = { needsActionCount: number, rows: Array<TaskRow>, };

export type AccountConfirmationCandidate = { accountId: string, accountType: string, currency: string | null, displayName: string, maskedIdentifier: string | null, };

export type AccountConfirmationPrompt = { candidateAccounts: Array<AccountConfirmationCandidate>, dismissedAccounts: Array<AccountConfirmationCandidate>, displayName: string, moneySourceId: string, proposalVersion: string, };

export type CandidateAccountDecision = "accept" | "dismiss";

export type CandidateAccountDecisionInput = { accountId: string, action: CandidateAccountDecision, };

export type AccountConfirmationStatus = "already_confirmed" | "confirmed" | "conflict" | "restored" | "updated";

export type AccountConfirmationOutcome = { status: AccountConfirmationStatus, };

export type MoneySourceCandidateStatus = "confirmed" | "kept_unassigned" | "pending";

export type SourceConfirmationScopeKind = "provider_singleton" | "provider_root_id";

export type SourceConfirmationPromptStatus = "pending" | "kept_unassigned";

export type SourceConfirmationPrompt = { candidateId: string, documentCount: number, latestDocumentTitle: string | null, providerKey: string, scopeKind: SourceConfirmationScopeKind, scopeValue: string, status: SourceConfirmationPromptStatus, version: number, };

export type MoneySourceCandidateState = { candidateId: string, confirmedMoneySourceId: string | null, status: MoneySourceCandidateStatus, version: number, };

export type ConfirmedMoneySourceCandidate = { candidateId: string, moneySourceId: string, version: number, };
