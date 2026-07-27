// This file is generated from Rust presentation types. Do not edit.

export type SourceDocumentStatus = "file_deleted" | "missing" | "needs_attention" | "processing" | "ready";

export type SourceDocumentSummary = { attentionReason: string | null, byteSize: number, documentStatus: SourceDocumentStatus, documentId: string, fileState: string, mimeType: string, originalFilename: string, receivedAt: string, };

export type AccountConfirmationCandidate = { accountId: string, accountType: string, currency: string | null, displayName: string, maskedIdentifier: string | null, };

export type AccountConfirmationPrompt = { candidateAccounts: Array<AccountConfirmationCandidate>, dismissedAccounts: Array<AccountConfirmationCandidate>, displayName: string, moneySourceId: string, proposalVersion: string, };

export type CandidateAccountDecision = "accept" | "dismiss";

export type CandidateAccountDecisionInput = { accountId: string, action: CandidateAccountDecision, };

export type AccountConfirmationStatus = "already_confirmed" | "confirmed" | "conflict" | "restored" | "updated";

export type AccountConfirmationOutcome = { status: AccountConfirmationStatus, };
