// Dev-only Sources fixtures and previews, split from preview.tsx under the
// 800-line source guardrail (docs/specs/0001-repo-structure.md).
import type {
  AccountConfirmationPrompt,
  LocalInboxStatus,
  MoneySourceDetail,
  MoneySourceSummary,
  SourceDocumentSummary,
  SupportedMoneySourceProviderSummary,
} from "./command-contracts";
import { SourceDialog } from "./source-dialogs";
import { SourcesView } from "./sources-view";

export const previewMoneySources: MoneySourceSummary[] = [
  {
    displayName: "DBS",
    moneySourceId: "source-dbs",
    providerKey: "dbs",
    sourceType: "bank",
  },
  {
    displayName: "Wise",
    moneySourceId: "source-wise",
    providerKey: "wise",
    sourceType: "wallet",
  },
];

export const dbsDocuments: SourceDocumentSummary[] = [
  {
    attentionReason: null,
    byteSize: 248_000,
    documentId: "document-1",
    documentStatus: "ready",
    fileState: "available",
    mimeType: "application/pdf",
    originalFilename: "June statement.pdf",
    receivedAt: "2026-07-19T09:12:00Z",
  },
  {
    attentionReason: "password_required",
    byteSize: 196_000,
    documentId: "document-2",
    documentStatus: "needs_attention",
    fileState: "available",
    mimeType: "application/pdf",
    originalFilename: "May statement.pdf",
    receivedAt: "2026-05-18T08:03:00Z",
  },
  {
    attentionReason: null,
    byteSize: 84_000,
    documentId: "document-3",
    documentStatus: "file_deleted",
    fileState: "deleted",
    mimeType: "text/csv",
    originalFilename: "wise-export.csv",
    receivedAt: "2026-05-02T17:20:00Z",
  },
];

const dbsSourceDetail: MoneySourceDetail = {
  actions: {
    canEnterStatementPassword: true,
    hasSavedStatementPassword: true,
  },
  displayName: "DBS",
  documents: dbsDocuments,
  moneySourceId: "source-dbs",
  providerKey: "dbs",
  sourceType: "bank",
};

const dbsSourceDetailEmpty: MoneySourceDetail = {
  actions: {
    canEnterStatementPassword: false,
    hasSavedStatementPassword: false,
  },
  displayName: "DBS",
  documents: [],
  moneySourceId: "source-dbs",
  providerKey: "dbs",
  sourceType: "bank",
};

const previewProviders: SupportedMoneySourceProviderSummary[] = [
  {
    configuredMoneySourceId: "source-dbs",
    displayName: "DBS",
    providerKey: "dbs",
    sourceType: "bank",
  },
  {
    configuredMoneySourceId: null,
    displayName: "HSBC",
    providerKey: "hsbc",
    sourceType: "bank",
  },
];

export const sourceDialogStates: Record<string, true> = {
  "source-create": true,
  "source-rename": true,
  "source-remove-password": true,
};

export const unassignedEvidence: SourceDocumentSummary[] = [
  {
    attentionReason: null,
    byteSize: 312_000,
    documentId: "document-9",
    documentStatus: "processing",
    fileState: "available",
    mimeType: "image/png",
    originalFilename: "phone-receipt.png",
    receivedAt: "2026-07-21T10:40:00Z",
  },
];

export const accountPrompts: AccountConfirmationPrompt[] = [
  {
    candidateAccounts: [
      {
        accountId: "account-dbs",
        accountType: "deposit_account",
        currency: "SGD",
        displayName: "DBS Multiplier Account",
        maskedIdentifier: "•••• 1234",
      },
    ],
    dismissedAccounts: [],
    displayName: "DBS",
    moneySourceId: "source-dbs",
    proposalVersion: "proposal-version-1",
  },
];

const noop = () => undefined;

export function renderSourcesView(
  inbox: LocalInboxStatus,
  detail: "none" | "loading" | "ready" | "empty" = "none",
  sourcePrompts: Parameters<typeof SourcesView>[0]["sourcePrompts"] = [],
) {
  return (
    <SourcesView
      accountPrompts={accountPrompts}
      attentionBusyKey={null}
      busy={false}
      existingSources={previewMoneySources}
      importing={false}
      inbox={inbox}
      inboxError={null}
      inboxBusy={false}
      inboxConfirmingDisable={false}
      loadingDocuments={detail === "loading"}
      normalizingDocumentId={null}
      notice={null}
      onClearMoneySourceSelection={noop}
      onConfirmSourceCandidate={noop}
      onCreateMoneySource={noop}
      onDecideAccounts={noop}
      onImport={noop}
      onInboxCancelDisable={noop}
      onInboxChoose={noop}
      onInboxConfirmDisable={noop}
      onInboxRequestDisable={noop}
      onInboxRescan={noop}
      onInboxRetry={noop}
      onKeepSourceCandidateUnassigned={noop}
      onLock={noop}
      onNormalize={noop}
      onOpenEditMoneySource={noop}
      onOpenRemoveSourcePassword={noop}
      onOpenUnlock={noop}
      onRefresh={noop}
      onRememberedChange={noop}
      onRequestDelete={noop}
      onRestoreAccount={noop}
      onSaveRecoveryFile={noop}
      onSaveSourceCopy={noop}
      onSelectMoneySource={noop}
      onView={noop}
      onViewPromptDocument={noop}
      recoveryConfigured={false}
      rememberedOnThisMac={false}
      savingCopyDocumentId={null}
      savingRecoveryFile={false}
      selectedMoneySourceId={detail === "none" ? null : "source-dbs"}
      sourceDocuments={[
        {
          detail: detail === "ready" ? dbsSourceDetail
            : detail === "empty" ? dbsSourceDetailEmpty
            : null,
          source: previewMoneySources[0]!,
        },
        { detail: null, source: previewMoneySources[1]! },
      ]}
      sourcePrompts={sourcePrompts}
      unassignedDocuments={unassignedEvidence}
      updatingRemembered={false}
    />
  );
}

/** The three Money Source dialogs against the shared provider/source fixtures. */
export function SourceDialogPreview({ state }: { state: string }) {
  const shared = {
    onClose: noop,
    onCreate: noop,
    onDisplayNameChange: noop,
    onProviderChange: noop,
    onRemovePassword: noop,
    onRename: noop,
    onRetryProviders: noop,
  };
  if (state === "source-create") {
    return (
      <SourceDialog
        {...shared}
        state={{
          busy: false,
          displayName: "",
          error: null,
          kind: "create",
          providerKey: "",
          providers: previewProviders,
        }}
      />
    );
  }
  if (state === "source-rename") {
    return (
      <SourceDialog
        {...shared}
        state={{
          busy: false,
          displayName: previewMoneySources[0]!.displayName,
          error: null,
          kind: "rename",
          source: previewMoneySources[0]!,
        }}
      />
    );
  }
  return (
    <SourceDialog
      {...shared}
      state={{
        busy: false,
        error: null,
        kind: "remove_password",
        source: previewMoneySources[0]!,
      }}
    />
  );
}
