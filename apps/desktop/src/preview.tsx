/**
 * Dev-only visual inspection harness. Renders the Command Center views with
 * deterministic fixtures, selected via `?state=` (overview, overview-empty,
 * tasks, tasks-parked, source-confirm, review, review-empty, review-detail,
 * review-committed, review-job, sources, source-detail, source-detail-empty,
 * source-create, source-rename, source-remove-password,
 * sources-inbox-disabled, sources-inbox-enabled,
 * sources-inbox-reauth, document-viewer, document-preview, document-unlock,
 * delete-confirm, vault-gate-loading, vault-gate-create,
 * vault-gate-locked, primitives, primitives-dialog).
 * Not part of the shipped bundle: `vite build` only bundles index.html.
 */
import { AppShell, LedgerColumn, LedgerRegion } from "@cancan/ui";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@cancan/ui/fonts.css";
import "./app.css";

import { PrimitivesGallery } from "./preview-gallery";

import type {
  LocalInboxStatus,
  MoneyOverview,
  MoneySourceSummary,
  RecentActivitySummary,
  RelationshipCandidateSummary,
  ReviewItemDetail,
  ReviewItemSummary,
  SourceConfirmationPrompt,
  Tasks,
} from "./command-contracts";
import {
  DeleteSourceDocumentDialog,
  DocumentPreview,
  DocumentUnlock,
  DocumentViewer,
} from "./document-modals";
import { OverviewView } from "./overview";
import { ReviewView, type ReviewDetailState, type ReviewJobPanelState } from "./review";
import { FocusedSourceConfirmationDialog } from "./source-confirmation";
import {
  dbsDocuments,
  previewMoneySources,
  renderSourcesView,
  SourceDialogPreview,
  sourceDialogStates,
} from "./preview-sources";
import { TasksView } from "./tasks-view";
import { VaultGate } from "./vault-gate";
import { VaultSpine, type AppView } from "./vault-spine";
import { App } from "./app";
import { createVaultApi, type TauriInvoke } from "./vault-api";

// Dev-only rendered page fixture (generated statement-page PNG, base64).
import statementPagePng from "./preview-statement-page.b64?raw";

const moneyOverview: MoneyOverview = {
  assets: [
    {
      accountId: "account-dbs",
      accountLabel: "DBS Multiplier Account",
      asOf: "2026-07-19",
      currency: "SGD",
      value: "12456.78",
    },
    {
      accountId: "account-wise",
      accountLabel: "Wise USD Balance",
      asOf: "2026-07-18",
      currency: "USD",
      value: "980.50",
    },
  ],
  liabilities: [
    {
      accountId: "account-card",
      accountLabel: "DBS Visa Card",
      asOf: "2026-07-19",
      currency: "SGD",
      value: "1234.56",
    },
  ],
};

const recentActivity: RecentActivitySummary[] = [
  {
    canUndo: true,
    eventDate: "2026-07-18",
    eventId: "event-1",
    eventType: "credit_card_repayment",
    sourceLabels: ["DBS", "DBS Card"],
    spending: false,
  },
  {
    canUndo: false,
    eventDate: "2026-07-17",
    eventId: "event-2",
    eventType: "purchase",
    sourceLabels: ["DBS Card"],
    spending: true,
  },
];

const reviewItems: ReviewItemSummary[] = [
  {
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
  },
  {
    accountLabel: "DBS Visa Card",
    amountValue: "512.34",
    currency: "SGD",
    eventType: "credit_card_repayment",
    postedOn: "2026-07-17",
    reasonCode: "possible_card_repayment",
    recordCommitted: false,
    recordId: "record-2",
    recordVersion: 1,
    reviewItemId: "review-2",
  },
  {
    accountLabel: "DBS Multiplier Account",
    amountValue: "86.40",
    currency: "SGD",
    eventType: "purchase",
    postedOn: null,
    reasonCode: "classification_conflict",
    recordCommitted: false,
    recordId: "record-3",
    recordVersion: 1,
    reviewItemId: "review-3",
  },
  {
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
  },
];

const reviewDetail: ReviewItemDetail = {
  ...reviewItems[0]!,
  documentLabel: "July statement.pdf",
  sourceLabel: "DBS",
};

const committedReviewDetail: ReviewDetailState = {
  candidates: [],
  confirmingRemove: false,
  detail: {
    ...reviewItems[3]!,
    documentLabel: "June statement.pdf",
    sourceLabel: "HSBC",
  },
  editing: null,
  reviewItemId: "review-4",
  summary: reviewItems[3]!,
};

const reviewCandidates: RelationshipCandidateSummary[] = [
  {
    accountLabel: "DBS Visa Card",
    amountValue: "512.34",
    currency: "SGD",
    eventType: "credit_card_repayment",
    postedOn: "2026-07-17",
    recordId: "record-2",
    recordVersion: 1,
  },
];

const expandedDetail: ReviewDetailState = {
  candidates: reviewCandidates,
  confirmingRemove: false,
  detail: reviewDetail,
  editing: null,
  reviewItemId: "review-1",
  summary: reviewItems[0]!,
};

const finishedJob: ReviewJobPanelState = {
  jobId: "job-1",
  outcomes: [
    { reason: null, recordIds: ["record-1", "record-2"], status: "committed" },
    { reason: "core_preflight_failed", recordIds: ["record-3"], status: "still_needs_review" },
  ],
  status: "done",
};

/**
 * Task fixtures mirror the host's projection: Command Center rows exclude
 * parked and cap at five; the full route includes every group.
 */
const commandCenterTasks: Tasks = {
  needsActionCount: 2,
  rows: [
    {
      consequence: "password_needed",
      destination: { documentId: "document-1", kind: "password", moneySourceId: "source-dbs" },
      group: "needs_action",
      rowKey: "task:password:document-1",
      timestamp: "2026-07-19 09:12",
      title: "June statement.pdf",
    },
    {
      consequence: "new_source_detected",
      destination: { kind: "source_confirmation", moneySourceCandidateId: "candidate-dbs" },
      group: "needs_action",
      rowKey: "task:new_source:candidate-dbs",
      timestamp: "2026-07-18 15:40",
      title: "New source detected: DBS",
    },
    {
      consequence: "processing",
      destination: { documentId: "document-2", kind: "document" },
      group: "in_progress",
      rowKey: "task:processing:document-2",
      timestamp: "2026-07-18 08:03",
      title: "May statement.pdf",
    },
    {
      consequence: "ready",
      destination: { intakeItemId: "intake-1", kind: "receipt" },
      group: "recently_completed",
      rowKey: "task:ready:document-3",
      timestamp: "2026-07-17 19:26",
      title: "April statement.pdf",
    },
  ],
};

const fullTasks: Tasks = {
  ...commandCenterTasks,
  rows: [
    ...commandCenterTasks.rows,
    {
      consequence: "password_parked",
      destination: { documentId: "document-4", kind: "document" },
      group: "parked",
      rowKey: "task:parked:document-4",
      timestamp: "2026-07-10 11:52",
      title: "2025 tax export.csv",
    },
  ],
};

const existingSources: MoneySourceSummary[] = [
  {
    displayName: "Wise",
    moneySourceId: "source-wise",
    providerKey: "wise",
    sourceType: "wallet",
  },
];

const sourcePrompts: SourceConfirmationPrompt[] = [
  {
    candidateId: "candidate-dbs",
    documentCount: 2,
    latestDocumentId: "document-1",
    latestDocumentTitle: "June statement.pdf",
    providerKey: "dbs",
    scopeKind: "provider_singleton",
    status: "pending",
    version: 1,
  },
];

const inboxDisabled: LocalInboxStatus = {
  accessState: "disabled",
  backupsPrepared: false,
  enabled: false,
  inboxLabel: "Inbox",
  lastScan: null,
};

const inboxEnabled: LocalInboxStatus = {
  accessState: "enabled",
  backupsPrepared: false,
  enabled: true,
  inboxLabel: "Inbox",
  lastScan: {
    alreadyPresent: 12,
    deferred: 1,
    imported: 3,
    suppressed: 2,
  },
};

const inboxReauth: LocalInboxStatus = {
  accessState: "needs_reauthorization",
  backupsPrepared: false,
  enabled: true,
  inboxLabel: "Inbox",
  lastScan: null,
};


const noop = () => undefined;

const documentModalStates = new Set([
  "delete-confirm",
  "document-preview",
  "document-unlock",
  "document-viewer",
]);


/**
 * Interactive post-password Touch ID offer walkthrough. The real `App` runs
 * against a fake invoke so the gate, unlock, offer, accept, and dismiss all
 * behave like the desktop shell without a Tauri host.
 * `?state=touch-id-offer` starts locked with remembered unlock off;
 * `-remembered` and `-unavailable` cover the no-offer variants.
 */
function offerPreviewApi(remembered: boolean | null) {
  let vaultStatus: "locked" | "unlocked" = "locked";
  let rememberedOnThisMac = remembered;
  const respond = (command: string): unknown => {
    switch (command) {
      case "vault_access_status":
        return {
          recoveryConfigured: true,
          rememberedOnThisMac,
          status: vaultStatus,
        };
      case "vault_status":
        return vaultStatus;
      case "unlock_vault":
      case "unlock_vault_with_keychain":
        vaultStatus = "unlocked";
        return vaultStatus;
      case "lock_vault":
        vaultStatus = "locked";
        return vaultStatus;
      case "remember_vault_on_this_mac":
        rememberedOnThisMac = true;
        return undefined;
      case "forget_vault_on_this_mac":
        rememberedOnThisMac = false;
        return undefined;
      case "local_inbox_status":
        return inboxDisabled;
      case "list_review_items":
        return reviewItems;
      case "get_money_overview":
        return moneyOverview;
      case "list_recent_activity":
        return recentActivity;
      case "list_tasks":
        return commandCenterTasks;
      case "list_money_sources":
        return previewMoneySources;
      case "intake_notification_settings":
        return { enabled: false, permission: "not_determined" };
      // The offer walkthrough is not a notification click, so its window has
      // no route waiting for it.
      case "take_background_intake_route":
        return null;
      default:
        return [];
    }
  };
  return createVaultApi(
    ((command: string) => Promise.resolve(respond(command))) as TauriInvoke,
    async () => () => undefined,
  );
}

function navigate(view: AppView) {
  const state = view === "sources" ? "sources-inbox-enabled" : view;
  window.location.search = `?state=${state}`;
}

function Preview() {
  const params = new URLSearchParams(window.location.search);
  const state = params.get("state") ?? "overview";

  if (state === "primitives" || state === "primitives-dialog") {
    return <PrimitivesGallery dialogOpen={state === "primitives-dialog"} />;
  }

  if (state.startsWith("touch-id-offer")) {
    const remembered = state === "touch-id-offer-remembered"
      ? true
      : state === "touch-id-offer-unavailable"
        ? null
        : false;
    return <App api={offerPreviewApi(remembered)} />;
  }

  let content = null;
  let activeView: AppView = "overview";
  if (state === "overview" || state === "overview-empty" || state === "source-confirm") {
    const empty = state === "overview-empty";
    content = (
      <OverviewView
        loading={false}
        moneyOverview={empty ? { assets: [], liabilities: [] } : moneyOverview}
        notice={null}
        onLock={noop}
        onOpenSources={() => navigate("sources")}
        onOpenTask={noop}
        onRefresh={noop}
        onUndo={noop}
        onViewAllTasks={() => navigate("tasks")}
        recentActivity={empty ? [] : recentActivity}
        tasks={empty ? { needsActionCount: 0, rows: [] } : commandCenterTasks}
        undoingEventId={null}
      />
    );
  } else if (state === "tasks" || state === "tasks-parked") {
    activeView = "tasks";
    content = (
      <TasksView
        filter={state === "tasks-parked" ? "parked" : "needs_action"}
        onFilterChange={noop}
        onLock={noop}
        onOpenTask={noop}
        onRefresh={noop}
        tasks={fullTasks}
      />
    );
  } else if (state === "sources-inbox-disabled"
    || state === "sources-inbox-enabled"
    || state === "sources-inbox-reauth") {
    activeView = "sources";
    const status = state === "sources-inbox-enabled"
      ? inboxEnabled
      : state === "sources-inbox-reauth"
        ? inboxReauth
        : inboxDisabled;
    content = renderSourcesView(status, "none", sourcePrompts);
  } else if (state === "sources" || state === "source-detail" || state === "source-detail-empty"
    || sourceDialogStates[state] === true || documentModalStates.has(state)) {
    activeView = "sources";
    content = renderSourcesView(
      inboxEnabled,
      state === "sources" ? "none" : state === "source-detail-empty" ? "empty" : "ready",
      sourcePrompts,
    );
  } else if (state === "vault-gate-loading" || state === "vault-gate-create" || state === "vault-gate-locked") {
    content = state === "vault-gate-loading" ? (
      <VaultGate busy title="Checking your Vault" body="Confirming the local Vault state before showing evidence." />
    ) : (
      <VaultGate
        busy={false}
        body={state === "vault-gate-create"
          ? "Create a local Vault before adding your first statement or export."
          : "Unlock your local Vault to add a file or check its routing."}
        onPasswordChange={noop}
        onSubmit={noop}
        onUnlockWithKeychain={state === "vault-gate-locked" ? noop : undefined}
        password=""
        rememberedOnThisMac={state === "vault-gate-locked" ? true : undefined}
        title={state === "vault-gate-create" ? "Create your Vault" : "Unlock your Vault"}
      />
    );
  } else {
    activeView = "review";
    content = (
      <ReviewView
        detail={state === "review-detail"
          ? expandedDetail
          : state === "review-committed"
          ? committedReviewDetail
          : null}
        items={state === "review-empty" ? [] : reviewItems}
        job={state === "review-job" ? finishedJob : null}
        mutatingItemId={null}
        notice={state === "review-detail"
          ? {
              body: "CanCan will treat them as one event. Add both together when you’re ready.",
              tone: "success",
              title: "Linked",
            }
          : null}
        onAcceptCandidate={noop}
        onAcknowledge={noop}
        onCancelEdit={noop}
        onCancelRemove={noop}
        onClearSelection={noop}
        onCloseDetail={noop}
        onConfirmRemove={noop}
        onEditChange={noop}
        onEnqueue={noop}
        onLock={noop}
        onOpenDetail={noop}
        onRefresh={noop}
        onRemove={noop}
        onSaveEdit={noop}
        onSelectAll={noop}
        onStartEdit={noop}
        onToggleSelect={noop}
        selectedIds={state === "review-job" ? new Set() : new Set(["review-1", "review-2"])}
      />
    );
  }

  const modalPreview = state === "source-confirm" || sourceDialogStates[state] === true || documentModalStates.has(state);
  return (
    <AppShell>
      <VaultSpine
        activeView={activeView}
        inert={modalPreview}
        onNavigate={navigate}
        reviewCount={state === "overview-empty" || state === "review-empty" ? 0 : reviewItems.length}
        tasksCount={state === "overview-empty" ? 0 : commandCenterTasks.needsActionCount}
        vaultStatus="unlocked"
      />
      <LedgerRegion inert={modalPreview}>
        <LedgerColumn>{content}</LedgerColumn>
      </LedgerRegion>
      {state === "source-confirm" ? (
        <FocusedSourceConfirmationDialog
          busyKey={null}
          existingSources={existingSources}
          focusedCandidate={sourcePrompts[0]}
          onClose={noop}
          onConfirm={noop}
          onKeepUnassigned={noop}
          onViewDocument={noop}
        />
      ) : null}
      {state === "document-viewer" ? (
        <DocumentViewer
          onClose={noop}
          onPage={noop}
          viewer={{
            documentId: "document-1",
            documentTitle: "June statement.pdf",
            page: { pageCount: 3, pageNumber: 1, pngBase64: statementPagePng.trim() },
          }}
          viewingPage={false}
        />
      ) : null}
      {state === "document-preview" ? (
        <DocumentPreview
          onClose={noop}
          state={{
            documentId: "document-csv",
            documentTitle: "wise-export.csv",
            preview: {
              lineCount: 241,
              previewLines: 120,
              previewText: [
                "Booking Date,Payee,Amount SGD",
                "2026-07-18,FAIRPRICE FINEST,-82.40",
                "2026-07-17,GRAB RIDES,-18.60",
                "2026-07-15,DBS VISA PAYMENT,-512.34",
                "2026-07-12,APPLE SERVICES,-14.98",
                "…",
              ].join("\n"),
              truncated: true,
            },
          }}
        />
      ) : null}
      {state === "document-unlock" ? (
        <DocumentUnlock
          onClose={noop}
          onPasswordChange={noop}
          onRetrySources={noop}
          onSourceChange={noop}
          onSubmit={noop}
          state={{
            busy: false,
            documentId: "document-2",
            documentTitle: "May statement.pdf",
            error: null,
            password: "",
            savedPasswordStatus: null,
            selectedMoneySourceId: "",
            sources: [
              { displayName: "DBS", hasSavedPassword: true, moneySourceId: "source-dbs" },
              { displayName: "Wise", hasSavedPassword: false, moneySourceId: "source-wise" },
            ],
          }}
        />
      ) : null}
      {state === "delete-confirm" ? (
        <DeleteSourceDocumentDialog
          deleting={false}
          document={dbsDocuments[0]!}
          onCancel={noop}
          onConfirm={noop}
        />
      ) : null}
      {sourceDialogStates[state] === true ? (
        <SourceDialogPreview state={state} />
      ) : null}
    </AppShell>
  );
}

const root = document.getElementById("root");

if (!root) {
  throw new Error("CanCan preview root element is missing");
}

createRoot(root).render(
  <StrictMode>
    <Preview />
  </StrictMode>,
);
