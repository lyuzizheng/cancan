import { AppShell, LedgerColumn, LedgerRegion } from "@cancan/ui";
import { useCallback, useMemo, useRef, useState } from "react";

import type {
  AccountConfirmationPrompt,
  SourceConfirmationPrompt,
} from "./command-contracts";
import { EMPTY_COMMAND_WIRING, type CommandWiring } from "./command-wiring";
import {
  DeleteSourceDocumentDialog,
  DocumentPreview,
  DocumentUnlock,
  DocumentViewer,
} from "./document-modals";
import { Feedback } from "./feedback";
import { OverviewView } from "./overview";
import { ReviewView } from "./review";
import { FocusedSourceConfirmationDialog } from "./source-confirmation";
import { SourcesView } from "./sources-view";
import { createTaskDestinationActions } from "./task-destination-actions";
import { TasksView } from "./tasks-view";
import { useAttention } from "./use-attention";
import { useCommandCenter } from "./use-command-center";
import { useEvidenceOverlays } from "./use-evidence-overlays";
import { useInbox } from "./use-inbox";
import { useReviewQueue } from "./use-review-queue";
import { useVaultDocuments } from "./use-vault-documents";
import { useVaultSession } from "./use-vault-session";
import { createVaultApi, type VaultApi } from "./vault-api";
import { VaultGate } from "./vault-gate";
import { VaultSpine, type AppView } from "./vault-spine";

const defaultVaultApi = createVaultApi();

const NO_ACCOUNT_PROMPTS: AccountConfirmationPrompt[] = [];
const NO_SOURCE_PROMPTS: SourceConfirmationPrompt[] = [];

/**
 * The command center: one view at a time over the vault session. Every domain
 * (documents, review, inbox, attention, overlays, read model) lives in its own
 * hook and reports back through `wiring`; this component only routes between
 * them and the views.
 */
export function App({ api = defaultVaultApi }: { api?: VaultApi }) {
  // The session hook is declared first and the domains after it, so the
  // loaders and resetters they publish here are read at call time — no hook
  // depends on a hook declared later in this component.
  const wiring = useRef<CommandWiring>(EMPTY_COMMAND_WIRING);
  const [activeView, setActiveView] = useState<AppView>("overview");

  const session = useVaultSession({ api, wiring });
  const documents = useVaultDocuments({ api, session });
  const review = useReviewQueue({ api, session, wiring });
  const inbox = useInbox({ api, session, wiring });
  const attention = useAttention({
    api,
    loadDocuments: documents.loadDocuments,
    session,
    wiring,
  });
  const overlays = useEvidenceOverlays({
    api,
    loadDocuments: documents.loadDocuments,
    session,
  });
  const commandCenter = useCommandCenter({ api, attention, inbox, review, session, wiring });

  wiring.current = {
    loadDocuments: documents.loadDocuments,
    loadFinanceData: commandCenter.loadFinanceData,
    resetSession: () => {
      setActiveView("overview");
      documents.reset();
      review.reset();
      inbox.reset();
      attention.reset();
      overlays.reset();
      commandCenter.reset();
    },
  };

  const { lockVault, notice, refresh, setError, setNotice, status } = session;
  const { loadFinanceData } = commandCenter;

  const navigate = useCallback((view: AppView) => {
    setActiveView(view);
    setError(null);
    setNotice(null);
  }, [setError, setNotice]);

  const requestLock = useCallback(() => {
    void lockVault();
  }, [lockVault]);

  const requestRefresh = useCallback(() => {
    void refresh();
  }, [refresh]);

  const reloadFinance = useCallback(() => {
    void loadFinanceData();
  }, [loadFinanceData]);

  const openSources = useCallback(() => navigate("sources"), [navigate]);
  const openTasks = useCallback(() => navigate("tasks"), [navigate]);

  const {
    openTaskDestination,
    viewDocument,
    viewPromptDocument,
  } = useMemo(() => createTaskDestinationActions({
    loadDocumentPreview: overlays.loadDocumentPreview,
    loadViewerPage: overlays.loadViewerPage,
    navigate,
    openDocumentUnlock: overlays.openDocumentUnlock,
    saveRecoveryFile: documents.saveRecoveryFile,
    selectMoneySource: documents.selectMoneySource,
    selectedMoneySourceIdRef: documents.selectedMoneySourceIdRef,
    setFocusedCandidateId: attention.setFocusedCandidateId,
    setNotice,
    sourceDocuments: documents.sourceDocuments,
    unassignedDocuments: documents.unassignedDocuments,
    viewerReturnFocus: overlays.viewerReturnFocus,
  }), [
    attention.setFocusedCandidateId,
    documents.saveRecoveryFile,
    documents.selectMoneySource,
    documents.selectedMoneySourceIdRef,
    documents.sourceDocuments,
    documents.unassignedDocuments,
    navigate,
    overlays.loadDocumentPreview,
    overlays.loadViewerPage,
    overlays.openDocumentUnlock,
    overlays.viewerReturnFocus,
    setNotice,
  ]);

  const modalOpen = overlays.viewer !== null || overlays.preview !== null
    || overlays.unlockingDocument !== null || attention.focusedCandidateId !== null
    || documents.confirmingDelete !== null;
  const unlocked = status === "unlocked";
  const existingSources = useMemo(
    () => documents.sourceDocuments.map((entry) => entry.source),
    [documents.sourceDocuments],
  );
  const focusedCandidateId = attention.focusedCandidateId;
  const focusedCandidate = useMemo(
    () => focusedCandidateId === null ? undefined
      : (attention.sourcePrompts ?? NO_SOURCE_PROMPTS)
        .find((prompt) => prompt.candidateId === focusedCandidateId),
    [attention.sourcePrompts, focusedCandidateId],
  );

  return (
    <AppShell>
      <VaultSpine
        activeView={activeView}
        inert={modalOpen}
        onNavigate={navigate}
        reviewCount={unlocked ? review.reviewItems?.length ?? null : null}
        tasksCount={unlocked ? commandCenter.tasks?.needsActionCount ?? null : null}
        vaultStatus={status}
      />

      <LedgerRegion
        aria-hidden={modalOpen ? true : undefined}
        inert={modalOpen}
        aria-busy={status === "loading"}
      >
        <LedgerColumn>
        {session.error ? (
          <Feedback tone="error" title="Something needs your attention" body={session.error} action={requestRefresh} />
        ) : null}

        {status === "loading" ? (
          <VaultGate busy title="Checking your Vault" body="Confirming the local Vault state before showing evidence." />
        ) : null}

        {status === "not_created" || status === "locked" ? (
          <VaultGate
            busy={session.busy}
            body={status === "not_created" ? "Create a local Vault before adding your first statement or export." : "Unlock your local Vault to add a file or check its routing. While CanCan stays open, background intake keeps working and your Mac’s login session protects the live Vault; CanCan locks only when you lock it or quit the app."}
            password={session.password}
            rememberedOnThisMac={session.rememberedOnThisMac}
            title={status === "not_created" ? "Create your Vault" : "Unlock your Vault"}
            onPasswordChange={session.setPassword}
            onSubmit={session.unlock}
            onUnlockWithKeychain={status === "locked" ? session.unlockWithKeychain : undefined}
          />
        ) : null}

        {unlocked && activeView === "overview" ? (
          <OverviewView
            loading={commandCenter.moneyOverview === null && commandCenter.recentActivity === null}
            moneyOverview={commandCenter.moneyOverview}
            notice={notice}
            onLock={requestLock}
            onOpenSources={openSources}
            onOpenTask={openTaskDestination}
            onRefresh={requestRefresh}
            onUndo={commandCenter.undoCommittedEvent}
            onViewAllTasks={openTasks}
            recentActivity={commandCenter.recentActivity}
            tasks={commandCenter.tasks}
            undoingEventId={commandCenter.undoingEventId}
          />
        ) : null}

        {unlocked && activeView === "tasks" ? (
          <TasksView
            filter={commandCenter.tasksFilter}
            onFilterChange={commandCenter.setTasksFilter}
            onLock={requestLock}
            onOpenTask={openTaskDestination}
            onRefresh={requestRefresh}
            tasks={commandCenter.tasksFull}
          />
        ) : null}

        {unlocked && activeView === "review" ? (
          <ReviewView
            detail={review.reviewDetail}
            items={review.reviewItems}
            job={review.reviewJob}
            mutatingItemId={review.mutatingReviewItemId}
            notice={notice}
            onAcceptCandidate={review.acceptCandidate}
            onAcknowledge={review.acknowledge}
            onCancelEdit={review.cancelEdit}
            onCancelRemove={review.cancelRemove}
            onClearSelection={review.clearSelection}
            onCloseDetail={review.closeDetail}
            onConfirmRemove={review.confirmRemove}
            onEditChange={review.changeEdit}
            onEnqueue={review.enqueueBatch}
            onLock={requestLock}
            onOpenDetail={review.openDetail}
            onRefresh={requestRefresh}
            onRemove={review.requestRemove}
            onSaveEdit={review.saveEdit}
            onSelectAll={review.selectAll}
            onStartEdit={review.startEdit}
            onToggleSelect={review.toggleSelection}
            selectedIds={review.selectedReviewIds}
          />
        ) : null}

        {unlocked && activeView === "sources" ? (
          <SourcesView
            accountPrompts={attention.accountPrompts ?? NO_ACCOUNT_PROMPTS}
            attentionBusyKey={attention.attentionBusyKey}
            busy={session.busy}
            existingSources={existingSources}
            importing={documents.importing}
            inbox={inbox.status}
            inboxError={inbox.error}
            inboxBusy={inbox.busy}
            inboxConfirmingDisable={inbox.confirmingDisable}
            loadingDocuments={documents.loadingDocuments}
            normalizingDocumentId={documents.normalizingDocumentId}
            notice={notice}
            onConfirmSourceCandidate={attention.confirmSourceCandidate}
            onDecideAccounts={attention.decideAccounts}
            onImport={documents.importDocument}
            onInboxCancelDisable={inbox.cancelDisable}
            onInboxChoose={inbox.choose}
            onInboxConfirmDisable={inbox.confirmDisable}
            onInboxRequestDisable={inbox.requestDisable}
            onInboxRescan={inbox.rescan}
            onInboxRetry={reloadFinance}
            onKeepSourceCandidateUnassigned={attention.parkSourceCandidate}
            onLock={requestLock}
            onNormalize={documents.normalizeDocument}
            onOpenUnlock={overlays.openDocumentUnlock}
            onRefresh={requestRefresh}
            onRememberedChange={documents.updateRemembered}
            onRequestDelete={documents.setConfirmingDelete}
            onRestoreAccount={attention.restoreAccount}
            onSelectMoneySource={documents.selectMoneySource}
            onSaveRecoveryFile={documents.saveRecoveryFile}
            onSaveSourceCopy={documents.saveSourceCopy}
            onView={viewDocument}
            onViewPromptDocument={viewPromptDocument}
            recoveryConfigured={session.recoveryConfigured}
            rememberedOnThisMac={session.rememberedOnThisMac}
            savingCopyDocumentId={documents.savingCopyDocumentId}
            savingRecoveryFile={documents.savingRecoveryFile}
            selectedMoneySourceId={documents.selectedMoneySourceId}
            sourceDocuments={documents.sourceDocuments}
            sourcePrompts={attention.sourcePrompts ?? NO_SOURCE_PROMPTS}
            unassignedDocuments={documents.unassignedDocuments}
            updatingRemembered={documents.updatingRemembered}
          />
        ) : null}
        </LedgerColumn>
      </LedgerRegion>
      {unlocked && overlays.viewer ? (
        <DocumentViewer
          onClose={overlays.clearViewer}
          onPage={(pageNumber) => {
            const openViewer = overlays.viewer;
            if (openViewer) {
              overlays.loadViewerPage(
                openViewer.documentId,
                openViewer.documentTitle,
                pageNumber,
              );
            }
          }}
          viewer={overlays.viewer}
          viewingPage={overlays.viewingPage}
        />
      ) : null}
      {unlocked && overlays.preview ? (
        <DocumentPreview
          onClose={overlays.clearPreview}
          state={overlays.preview}
        />
      ) : null}
      {unlocked && overlays.unlockingDocument ? (
        <DocumentUnlock
          onClose={overlays.closeUnlock}
          onPasswordChange={overlays.setUnlockPassword}
          onRetrySources={overlays.retryUnlockSources}
          onSourceChange={overlays.selectUnlockSource}
          onSubmit={overlays.submitDocumentPassword}
          state={overlays.unlockingDocument}
        />
      ) : null}
      {unlocked ? (
        <DeleteSourceDocumentDialog
          deleting={documents.deletingDocumentId !== null}
          document={documents.confirmingDelete}
          onCancel={documents.cancelDelete}
          onConfirm={() => {
            if (documents.confirmingDelete) {
              documents.deleteDocument(documents.confirmingDelete.documentId);
            }
          }}
        />
      ) : null}
      {unlocked ? (
        <FocusedSourceConfirmationDialog
          busyKey={attention.attentionBusyKey}
          existingSources={existingSources}
          focusedCandidate={focusedCandidate}
          onClose={() => attention.setFocusedCandidateId(null)}
          onConfirm={(prompt, displayName, sourceType) =>
            void attention.confirmSourceCandidate(prompt, displayName, sourceType)}
          onKeepUnassigned={(prompt) => void attention.parkSourceCandidate(prompt)}
          onViewDocument={viewPromptDocument}
        />
      ) : null}
    </AppShell>
  );
}
