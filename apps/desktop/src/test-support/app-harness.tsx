// Shared fixtures and DOM harness for the App interaction test files.
// Each interaction test file calls installAppHarness() once at module scope.
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, vi } from "vitest";

import type {
  AccountConfirmationOutcome,
  AccountConfirmationPrompt,
  LocalInboxScanSummary,
  LocalInboxStatus,
  MoneyOverview,
  MoneySourceSummary,
  StatementCoveragePrompt,
  RecentActivitySummary,
  RelationshipCandidateSummary,
  ReviewItemDetail,
  ReviewItemSummary,
  ReviewJobSummary,
  ReviewMutationOutcome,
  SourceDocumentImportOutcome,
  RenderedDocumentPage,
  SavedStatementPasswordResult,
  SourceDocumentPreview,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
  UndoOutcome,
  VaultAccessStatus,
  VaultStatus,
} from "../command-contracts";
import { App } from "../app";
import type { VaultApi } from "../vault-api";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

export let container: HTMLDivElement;
export let root: Root;

export function recreateRoot() {
  root = createRoot(container);
}

export function installAppHarness() {
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(async () => {
    await act(async () => {
      root.unmount();
    });
    vi.clearAllTimers();
    vi.useRealTimers();
    container.remove();
  });
}

export const availableDocument = sourceDocument();
export const moneySource: MoneySourceSummary = {
  displayName: "Synthetic Bank",
  moneySourceId: "money-source-1",
  sourceType: "bank",
};
export const otherMoneySource: MoneySourceSummary = {
  displayName: "Another Bank",
  moneySourceId: "money-source-2",
  sourceType: "bank",
};

export const reviewItem: ReviewItemSummary = {
  accountLabel: "DBS Multiplier Account",
  amountValue: "512.34",
  currency: "SGD",
  eventType: "credit_card_repayment",
  postedOn: "2026-07-15",
  reasonCode: "possible_card_repayment",
  recordId: "record-1",
  recordVersion: 1,
  reviewItemId: "review-1",
};
export const linkedReviewItem: ReviewItemSummary = {
  ...reviewItem,
  accountLabel: "DBS Visa Card",
  postedOn: "2026-07-17",
  recordId: "record-2",
  reviewItemId: "review-2",
};
export const reviewDetail: ReviewItemDetail = {
  ...reviewItem,
  documentLabel: "July statement.pdf",
  sourceLabel: "DBS",
};
export const inboxDisabled: LocalInboxStatus = {
  accessState: "disabled",
  backupsPrepared: false,
  enabled: false,
  inboxLabel: "Inbox",
  lastScan: null,
};

export const inboxEnabled: LocalInboxStatus = {
  accessState: "enabled",
  backupsPrepared: false,
  enabled: true,
  inboxLabel: "Inbox",
  lastScan: null,
};

export const coveragePrompt: StatementCoveragePrompt = {
  accountId: "account-dbs",
  documentType: "bank_statement",
  moneySourceId: "money-source-1",
  statementPeriodFrom: "2026-06-01",
  statementPeriodTo: "2026-06-30",
  status: "confirmed_missing",
};

export const accountConfirmationPrompt: AccountConfirmationPrompt = {
  candidateAccounts: [
    {
      accountId: "account-dbs",
      accountType: "deposit_account",
      currency: "SGD",
      displayName: "DBS Multiplier Account",
      maskedIdentifier: "•••• 1234",
    },
    {
      accountId: "account-card",
      accountType: "credit_card",
      currency: "SGD",
      displayName: "DBS Visa Card",
      maskedIdentifier: null,
    },
  ],
  displayName: "Synthetic Bank",
  moneySourceId: "money-source-1",
};

export const reviewCandidate: RelationshipCandidateSummary = {
  accountLabel: "DBS Visa Card",
  amountValue: "512.34",
  currency: "SGD",
  eventType: "credit_card_repayment",
  postedOn: "2026-07-17",
  recordId: "record-2",
  recordVersion: 1,
};

export function sourceDocument(
  overrides: Partial<SourceDocumentSummary> = {},
): SourceDocumentSummary {
  return {
    byteSize: 42,
    documentId: "document-1",
    documentStatus: "ready",
    fileState: "available",
    mimeType: "application/pdf",
    originalFilename: "June statement.pdf",
    receivedAt: "2026-07-19T00:00:00Z",
    ...overrides,
  };
}

export function createApi(overrides: Partial<VaultApi> = {}) {
  const api = {
    acceptReviewRelationship: vi.fn(async (): Promise<ReviewMutationOutcome> => ({
      reason: null,
      recordVersion: null,
      reviewItemId: null,
      status: "relationship_accepted",
    })),
    chooseLocalInboxRoot: vi.fn(
      async (): Promise<LocalInboxStatus | null> => null,
    ),
    confirmCandidateAccounts: vi.fn(
      async (): Promise<AccountConfirmationOutcome> => ({ status: "confirmed" }),
    ),
    createVault: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
    deleteSourceDocument: vi.fn(async (): Promise<boolean> => true),
    disableLocalInbox: vi.fn(async (): Promise<LocalInboxStatus> => inboxDisabled),
    editReviewRecord: vi.fn(async (): Promise<ReviewMutationOutcome> => ({
      reason: null,
      recordVersion: 2,
      reviewItemId: null,
      status: "updated",
    })),
    enqueueCommitReviewBatch: vi.fn(async (): Promise<ReviewJobSummary> => ({
      createdAt: "2026-07-25 09:00:00",
      finishedAt: null,
      jobId: "job-1",
      outcomes: [],
      status: "queued",
    })),
    forgetVaultOnThisMac: vi.fn(async (): Promise<void> => undefined),
    getMoneyOverview: vi.fn(async (): Promise<MoneyOverview> => ({
      assets: [],
      liabilities: [],
    })),
    getReviewDetail: vi.fn(async (): Promise<ReviewItemDetail | null> => reviewDetail),
    getReviewJob: vi.fn(async (): Promise<ReviewJobSummary | null> => ({
      createdAt: "2026-07-25 09:00:00",
      finishedAt: "2026-07-25 09:00:01",
      jobId: "job-1",
      outcomes: [],
      status: "succeeded",
    })),
    importSourceDocument: vi.fn(
      async (): Promise<SourceDocumentImportOutcome | null> => null,
    ),
    listAccountConfirmationPrompts: vi.fn(async () => []),
    listMoneySources: vi.fn(async (): Promise<MoneySourceSummary[]> => []),
    listRecentActivity: vi.fn(async (): Promise<RecentActivitySummary[]> => []),
    listRelationshipCandidates: vi.fn(
      async (): Promise<RelationshipCandidateSummary[]> => [],
    ),
    listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => []),
    listSourceDocuments: vi.fn(async (): Promise<SourceDocumentSummary[]> => []),
    listStatementCoveragePrompts: vi.fn(async () => []),
    listStatementPasswordSources: vi.fn(async () => []),
    listUnassignedSourceDocuments: vi.fn(
      async (): Promise<SourceDocumentSummary[]> => [],
    ),
    localInboxStatus: vi.fn(async (): Promise<LocalInboxStatus> => inboxDisabled),
    lockVault: vi.fn(async (): Promise<VaultStatus> => "locked"),
    normalizeSourceDocument: vi.fn(
      async (documentId: string): Promise<SourceDocumentRoutingOutcome> => ({
        accountIds: [],
        documentId,
        moneySourceId: null,
        reason: "classification_uncertain",
        status: "needs_attention",
      }),
    ),
    onVaultLocked: vi.fn(async () => () => undefined),
    previewSourceDocument: vi.fn(
      async (): Promise<SourceDocumentPreview> => ({
        lineCount: 0,
        previewLines: 0,
        previewText: "",
        truncated: false,
      }),
    ),
    recordStatementCoverageDecision: vi.fn(async (): Promise<void> => undefined),
    rememberVaultOnThisMac: vi.fn(async (): Promise<void> => undefined),
    removeReviewRecord: vi.fn(async (): Promise<ReviewMutationOutcome> => ({
      reason: null,
      recordVersion: null,
      reviewItemId: null,
      status: "removed",
    })),
    renderSourceDocumentPage: vi.fn(
      async (_documentId: string, pageNumber: number): Promise<RenderedDocumentPage> => ({
        pageCount: 2,
        pageNumber,
        pngBase64: "cmVuZGVyZWQtcGFnZQ==",
      }),
    ),
    rescanLocalInbox: vi.fn(
      async (): Promise<LocalInboxScanSummary> => ({
        alreadyPresent: 12,
        deferred: 1,
        imported: 3,
        suppressed: 2,
      }),
    ),
    saveRecoveryFile: vi.fn(async (): Promise<boolean> => false),
    saveSourceDocumentCopy: vi.fn(async (): Promise<boolean> => false),
    trySavedStatementPassword: vi.fn(
      async (): Promise<SavedStatementPasswordResult> => "invalid",
    ),
    undoCommittedEvent: vi.fn(async (): Promise<UndoOutcome> => ({
      eventId: "event-1",
      status: "undone",
    })),
    unlockSourceDocument: vi.fn(async (): Promise<void> => undefined),
    unlockVault: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
    unlockVaultWithKeychain: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
    vaultAccessStatus: vi.fn(
      async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "unlocked",
      }),
    ),
    vaultStatus: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
  };

  return Object.assign(api, overrides) as VaultApi & typeof api;
}

export function deferred<Value>() {
  let resolve: (value: Value) => void;
  let reject: (reason?: unknown) => void;
  const promise = new Promise<Value>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });

  return { promise, reject: reject!, resolve: resolve! };
}

export async function settle() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

export async function mount(api: VaultApi, view?: "sources" | "review") {
  await act(async () => {
    root.render(<App api={api} />);
    await settle();
  });
  if (view !== undefined) {
    await act(async () => {
      navItem(view === "sources" ? "Sources" : "Review").click();
      await settle();
    });
  }
}

export function navItem(label: string): HTMLButtonElement {
  const matches = [...container.querySelectorAll<HTMLButtonElement>(".vault-nav-item")]
    .filter((element) => element.textContent?.trim().startsWith(label));
  expect(matches).toHaveLength(1);
  return matches[0]!;
}

export function button(label: string): HTMLButtonElement {
  const matches = [...container.querySelectorAll("button")].filter(
    (element) => element.textContent?.trim() === label,
  );
  expect(matches).toHaveLength(1);
  return matches[0]!;
}

export function buttons(label: string): HTMLButtonElement[] {
  return [...container.querySelectorAll("button")].filter(
    (element): element is HTMLButtonElement => element.textContent?.trim() === label,
  );
}

export async function click(label: string) {
  await act(async () => {
    button(label).click();
    await settle();
  });
}

export async function enterPassword(password: string) {
  await enterInput("#vault-password", password);
}

export async function enterInput(selector: string, value: string) {
  const input = container.querySelector<HTMLInputElement>(selector);
  expect(input).not.toBeNull();
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  expect(setter).toBeDefined();

  await act(async () => {
    setter!.call(input, value);
    input!.dispatchEvent(new Event("input", { bubbles: true }));
    await settle();
  });
}

export async function enterRemindDate(value: string) {
  const input = container.querySelector<HTMLInputElement>(
    ".attention-remind input[aria-label=\"Remind after\"]",
  );
  expect(input).not.toBeNull();
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  expect(setter).toBeDefined();
  await act(async () => {
    setter!.call(input, value);
    input!.dispatchEvent(new Event("input", { bubbles: true }));
    await settle();
  });
}

export async function enterField(label: string, value: string) {
  const input = [...container.querySelectorAll<HTMLInputElement>(".review-edit-grid input")]
    .find((element) => element.getAttribute("aria-label") === label);
  expect(input).toBeDefined();
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  await act(async () => {
    setter!.call(input, value);
    input!.dispatchEvent(new Event("input", { bubbles: true }));
    await settle();
  });
}
