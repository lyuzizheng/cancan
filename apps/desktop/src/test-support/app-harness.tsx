// Shared fixtures and DOM harness for the App interaction test files.
// Each interaction test file calls installAppHarness() once at module scope.
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, vi } from "vitest";

import type {
  AccountConfirmationOutcome,
  AccountConfirmationPrompt,
  ConfirmedMoneySourceCandidate,
  IntakeNotificationSettings,
  LocalInboxScanSummary,
  LocalInboxStatus,
  MoneyOverview,
  MoneySourceCandidateState,
  MoneySourceSummary,
  OperationalDiagnosticsPreview,
  RecentActivitySummary,
  RelationshipCandidateSummary,
  ReviewItemDetail,
  ReviewItemSummary,
  ReviewJobSummary,
  ReviewMutationOutcome,
  SourceDocumentImportOutcome,
  RenderedDocumentPage,
  SavedStatementPasswordResult,
  SourceConfirmationPrompt,
  SourceDocumentPreview,
  SourceDocumentSummary,
  TaskRow,
  Tasks,
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
  providerKey: "synthetic-bank",
  sourceType: "bank",
};
export const otherMoneySource: MoneySourceSummary = {
  displayName: "Another Bank",
  moneySourceId: "money-source-2",
  providerKey: "another-bank",
  sourceType: "bank",
};

export const reviewItem: ReviewItemSummary = {
  accountLabel: "DBS Multiplier Account",
  amountValue: "512.34",
  currency: "SGD",
  eventType: "credit_card_repayment",
  postedOn: "2026-07-15",
  reasonCode: "possible_card_repayment",
  recordCommitted: false,
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
export const committedReviewItem: ReviewItemSummary = {
  accountLabel: "HSBC Everyday",
  amountValue: "800.00",
  currency: "SGD",
  eventType: "credit_card_repayment",
  postedOn: "2026-06-30",
  reasonCode: "reparse_divergence",
  recordCommitted: true,
  recordId: "record-4",
  recordVersion: 1,
  reviewItemId: "review-4",
};
export const committedReviewDetail: ReviewItemDetail = {
  ...committedReviewItem,
  documentLabel: "June statement.pdf",
  sourceLabel: "HSBC",
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

export const diagnosticsPreview: OperationalDiagnosticsPreview = {
  appVersion: "0.1.0",
  components: [{ count: 2, label: "runtime.documents" }],
  entryCount: 2,
  errorCodes: [{ count: 1, label: "import_failed" }],
  exportLimit: 5000,
  newestEntryAt: "2026-09-17T08:00:00Z",
  oldestEntryAt: "2026-09-16T08:00:00Z",
  retentionDays: 30,
  sampleLines: [
    "2026-09-16T08:00:00Z ERROR component=runtime.documents code=import_failed",
    "2026-09-17T08:00:00Z INFO component=runtime.inbox",
  ],
};

export const sourceConfirmationPrompt: SourceConfirmationPrompt = {
  candidateId: "candidate-dbs",
  documentCount: 2,
  latestDocumentId: "document-1",
  latestDocumentTitle: "June statement.pdf",
  providerKey: "dbs",
  scopeKind: "provider_singleton",
  status: "pending",
  version: 1,
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
  dismissedAccounts: [],
  displayName: "Synthetic Bank",
  moneySourceId: "money-source-1",
  proposalVersion: "proposal-version-1",
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
    attentionReason: null,
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
    acknowledgeReviewItem: vi.fn(async (): Promise<ReviewMutationOutcome> => ({
      reason: null,
      recordVersion: null,
      reviewItemId: null,
      status: "acknowledged",
    })),
    chooseLocalInboxRoot: vi.fn(
      async (): Promise<LocalInboxStatus | null> => null,
    ),
    closeSourceDocumentView: vi.fn(async (): Promise<void> => undefined),
    confirmSourceCandidate: vi.fn(
      async (): Promise<ConfirmedMoneySourceCandidate> => ({
        candidateId: "candidate-1",
        moneySourceId: "source-1",
        version: 3,
      }),
    ),
    createVault: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
    deleteSourceDocument: vi.fn(async (): Promise<boolean> => true),
    decideCandidateAccounts: vi.fn(
      async (): Promise<AccountConfirmationOutcome> => ({ status: "updated" }),
    ),
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
    intakeNotificationSettings: vi.fn(
      async (): Promise<IntakeNotificationSettings> => ({
        enabled: false,
        permission: "not_determined",
      }),
    ),
    listAccountConfirmationPrompts: vi.fn(async () => []),
    listSourceConfirmationPrompts: vi.fn(async () => []),
    listMoneySources: vi.fn(async (): Promise<MoneySourceSummary[]> => []),
    listRecentActivity: vi.fn(async (): Promise<RecentActivitySummary[]> => []),
    listRelationshipCandidates: vi.fn(
      async (): Promise<RelationshipCandidateSummary[]> => [],
    ),
    listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => []),
    listSourceDocuments: vi.fn(async (): Promise<SourceDocumentSummary[]> => []),
    listStatementPasswordSources: vi.fn(async () => []),
    listTasks: vi.fn(async (): Promise<Tasks> => ({ needsActionCount: 0, rows: [] })),
    listUnassignedSourceDocuments: vi.fn(
      async (): Promise<SourceDocumentSummary[]> => [],
    ),
    localInboxStatus: vi.fn(async (): Promise<LocalInboxStatus> => inboxDisabled),
    lockVault: vi.fn(async (): Promise<VaultStatus> => "locked"),
    onBackgroundIntakeRoute: vi.fn(async () => () => undefined),
    onVaultLocked: vi.fn(async () => () => undefined),
    openNotificationSettings: vi.fn(async (): Promise<void> => undefined),
    operationalDiagnosticsPreview: vi.fn(
      async (): Promise<OperationalDiagnosticsPreview> => diagnosticsPreview,
    ),
    parkSourceCandidate: vi.fn(
      async (): Promise<MoneySourceCandidateState> => ({
        candidateId: "candidate-1",
        confirmedMoneySourceId: null,
        status: "kept_unassigned",
        version: 4,
      }),
    ),
    previewSourceDocument: vi.fn(
      async (): Promise<SourceDocumentPreview> => ({
        lineCount: 0,
        previewLines: 0,
        previewText: "",
        truncated: false,
      }),
    ),
    reparseSourceDocument: vi.fn(async (): Promise<void> => undefined),
    rememberVaultOnThisMac: vi.fn(async (): Promise<void> => undefined),
    restoreDismissedCandidateAccount: vi.fn(
      async (): Promise<AccountConfirmationOutcome> => ({ status: "restored" }),
    ),
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
    setIntakeNotificationsEnabled: vi.fn(
      async (enabled: boolean): Promise<IntakeNotificationSettings> => ({
        enabled,
        permission: enabled ? "authorized" : "not_determined",
      }),
    ),
    takeBackgroundIntakeRoute: vi.fn(async (): Promise<TaskRow | null> => null),
    saveOperationalDiagnostics: vi.fn(async (): Promise<boolean> => true),
    trySavedStatementPasswords: vi.fn(
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

export async function mount(api: VaultApi, view?: "sources" | "review" | "settings") {
  await act(async () => {
    root.render(<App api={api} />);
    await settle();
  });
  if (view !== undefined) {
    await act(async () => {
      navItem(view === "sources" ? "Sources" : view === "review" ? "Review" : "Settings").click();
      await settle();
    });
  }
}

export function navItem(label: string): HTMLButtonElement {
  const matches = [...container.querySelectorAll<HTMLButtonElement>('nav[aria-label="Command Center"] button')]
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

/** The single modal rendered outside `container` (Radix portals to body). */
export function bodyDialog(): HTMLElement {
  const dialogs = [...document.body.querySelectorAll<HTMLElement>('[role="dialog"]')]
    .filter((element) => !container.contains(element));
  expect(dialogs).toHaveLength(1);
  return dialogs[0]!;
}

export function noBodyDialog() {
  const dialogs = [...document.body.querySelectorAll<HTMLElement>('[role="dialog"]')]
    .filter((element) => !container.contains(element));
  expect(dialogs).toHaveLength(0);
}

export function dialogButton(label: string): HTMLButtonElement {
  const matches = [...bodyDialog().querySelectorAll("button")].filter(
    (element) => element.textContent?.trim() === label
      || element.getAttribute("aria-label") === label,
  );
  expect(matches).toHaveLength(1);
  return matches[0]!;
}

export async function clickDialogButton(label: string) {
  await act(async () => {
    dialogButton(label).click();
    await settle();
  });
}

export function dialogInput(selector: string): HTMLInputElement {
  const input = bodyDialog().querySelector<HTMLInputElement>(selector);
  expect(input).not.toBeNull();
  return input!;
}

export async function enterDialogInput(selector: string, value: string) {
  const input = dialogInput(selector);
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  expect(setter).toBeDefined();

  await act(async () => {
    setter!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await settle();
  });
}

/** Opens the token Select inside the active dialog and picks an option. */
export async function chooseDialogSelectOption(ariaLabel: string, optionLabel: string) {
  await act(async () => {
    const trigger = bodyDialog().querySelector<HTMLButtonElement>(
      `button[aria-label="${ariaLabel}"]`,
    );
    expect(trigger).not.toBeNull();
    trigger!.click();
    await settle();
  });
  const options = [...document.body.querySelectorAll<HTMLElement>('[role="option"]')]
    .filter((element) => element.textContent?.trim().startsWith(optionLabel));
  expect(options).toHaveLength(1);
  await act(async () => {
    options[0]!.click();
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


export async function enterField(label: string, value: string) {
  const input = [...container.querySelectorAll<HTMLInputElement>("input[aria-label]")]
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
