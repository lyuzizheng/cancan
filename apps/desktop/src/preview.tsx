/**
 * Dev-only visual inspection harness. Renders the Command Center views with
 * deterministic fixtures, selected via `?state=` (overview, overview-attention,
 * overview-empty, review, review-empty, review-detail, review-job,
 * sources-inbox-disabled, sources-inbox-enabled, sources-inbox-reauth).
 * Not part of the shipped bundle: `vite build` only bundles index.html.
 */
import { AppShell } from "@cancan/ui";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@cancan/ui/foundation.css";

import type {
  AccountConfirmationPrompt,
  LocalInboxStatus,
  MoneyOverview,
  MoneySourceSummary,
  RecentActivitySummary,
  RelationshipCandidateSummary,
  ReviewItemDetail,
  ReviewItemSummary,
  StatementCoveragePrompt,
} from "./command-contracts";
import { coverageKey } from "./attention";
import { OverviewView } from "./overview";
import { ReviewView, type ReviewDetailState, type ReviewJobPanelState } from "./review";
import { SourcesView } from "./sources-view";
import { VaultSpine, type AppView } from "./vault-spine";

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
    recordId: "record-3",
    recordVersion: 1,
    reviewItemId: "review-3",
  },
];

const reviewDetail: ReviewItemDetail = {
  ...reviewItems[0]!,
  documentLabel: "July statement.pdf",
  sourceLabel: "DBS",
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

const moneySources: MoneySourceSummary[] = [
  { displayName: "DBS", moneySourceId: "source-dbs", sourceType: "bank" },
];

const coveragePrompts: StatementCoveragePrompt[] = [
  {
    accountId: "account-dbs",
    documentType: "bank_statement",
    moneySourceId: "source-dbs",
    statementPeriodFrom: "2026-06-01",
    statementPeriodTo: "2026-06-30",
    status: "confirmed_missing",
  },
  {
    accountId: "account-card",
    documentType: "credit_card_statement",
    moneySourceId: "source-dbs",
    statementPeriodFrom: "2026-06-01",
    statementPeriodTo: "2026-06-30",
    status: "likely_missing",
  },
];

const accountPrompts: AccountConfirmationPrompt[] = [
  {
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
        maskedIdentifier: "•••• 5678",
      },
    ],
    displayName: "DBS",
    moneySourceId: "source-dbs",
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

function navigate(view: AppView) {
  const state = view === "sources" ? "sources-inbox-enabled" : view;
  window.location.search = `?state=${state}`;
}

function Preview() {
  const params = new URLSearchParams(window.location.search);
  const state = params.get("state") ?? "overview";

  let content = null;
  let activeView: AppView = "overview";
  if (state === "overview" || state === "overview-attention" || state === "overview-empty") {
    const empty = state === "overview-empty";
    const withAttention = state === "overview-attention";
    content = (
      <OverviewView
        accountPrompts={withAttention ? accountPrompts : []}
        attentionBusyKey={null}
        coveragePrompts={withAttention ? coveragePrompts : []}
        loading={false}
        moneyOverview={empty ? { assets: [], liabilities: [] } : moneyOverview}
        moneySources={moneySources}
        notice={null}
        onAddFile={noop}
        onCancelRemind={noop}
        onChangeRemindDate={noop}
        onConfirmAccounts={noop}
        onCoverageNotExpected={noop}
        onLock={noop}
        onOpenReview={() => navigate("review")}
        onOpenSources={() => navigate("sources")}
        onRefresh={noop}
        onSaveRemind={noop}
        onStartRemind={noop}
        onUndo={noop}
        recentActivity={empty ? [] : recentActivity}
        remind={withAttention
          ? {
              date: "2026-08-15",
              error: null,
              key: coverageKey(coveragePrompts[0]!),
              saving: false,
            }
          : null}
        reviewCount={empty ? 0 : reviewItems.length}
        undoingEventId={null}
      />
    );
  } else if (state === "sources-inbox-disabled"
    || state === "sources-inbox-enabled"
    || state === "sources-inbox-error"
    || state === "sources-inbox-reauth") {
    activeView = "sources";
    const status = state === "sources-inbox-enabled"
      ? inboxEnabled
      : state === "sources-inbox-reauth"
        ? inboxReauth
        : inboxDisabled;
    content = (
      <SourcesView
        busy={false}
        deletingDocumentId={null}
        importing={false}
        inbox={status}
        inboxBusy={false}
        inboxConfirmingDisable={false}
        inboxError={state === "sources-inbox-error"
          ? "CanCan Inbox is set up, but CanCan couldn’t watch for new files. Try again to restore automatic checks."
          : null}
        loadingDocuments={false}
        normalizingDocumentId={null}
        notice={null}
        onDelete={noop}
        onImport={noop}
        onInboxCancelDisable={noop}
        onInboxChoose={noop}
        onInboxConfirmDisable={noop}
        onInboxRequestDisable={noop}
        onInboxRescan={noop}
        onLock={noop}
        onNormalize={noop}
        onOpenUnlock={noop}
        onRefresh={noop}
        onRememberedChange={noop}
        onSaveRecoveryFile={noop}
        onSaveSourceCopy={noop}
        onSelectMoneySource={noop}
        onView={noop}
        recoveryConfigured
        rememberedOnThisMac={false}
        savingCopyDocumentId={null}
        savingRecoveryFile={false}
        selectedMoneySourceId={null}
        sourceDocuments={[{ documents: [], source: moneySources[0]! }]}
        unassignedDocuments={[]}
        updatingRemembered={false}
      />
    );
  } else {
    activeView = "review";
    content = (
      <ReviewView
        detail={state === "review-detail" ? expandedDetail : null}
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

  return (
    <AppShell>
      <VaultSpine
        activeView={activeView}
        inert={false}
        onNavigate={navigate}
        reviewCount={state === "overview-empty" || state === "review-empty" ? 0 : reviewItems.length}
        vaultStatus="unlocked"
      />
      <section className="ledger">{content}</section>
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
