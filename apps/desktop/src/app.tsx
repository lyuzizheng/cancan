import { AppShell } from "@cancan/ui";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  AccountConfirmationPrompt,
  EditReviewRecordArgs,
  LocalInboxStatus,
  MoneyOverview,
  RecentActivitySummary,
  RelationshipCandidateSummary,
  ReviewItemSummary,
  ReviewJobSummary,
  SourceDocumentSummary,
  MoneySourceSummary,
  StatementCoveragePrompt,
} from "./command-contracts";
import { coverageKey, type RemindState } from "./attention";
import {
  DocumentPreview,
  DocumentUnlock,
  DocumentViewer,
  type DocumentPreviewState,
  type DocumentUnlockState,
  type DocumentViewerState,
} from "./document-modals";
import { Feedback, type Notice } from "./feedback";
import {
  formatLedgerDate,
  localInboxScanSummaryText,
  localIsoToday,
  reviewConflictMessage,
} from "./format";
import { importNotice, routingNotice } from "./notices";
import { OverviewView } from "./overview";
import {
  ReviewView,
  type ReviewDetailState,
  type ReviewEditState,
  type ReviewJobPanelState,
} from "./review";
import { SourcesView, type MoneySourceDocuments } from "./sources-view";
import {
  commandErrorMessage,
  createVaultApi,
  type VaultApi,
} from "./vault-api";
import { VaultGate } from "./vault-gate";
import { VaultSpine, type AppView, type VaultScreenStatus } from "./vault-spine";

export type { Notice } from "./feedback";
export type { VaultScreenStatus } from "./vault-spine";

const defaultVaultApi = createVaultApi();
const VAULT_INACTIVITY_TIMEOUT_MS = 15 * 60 * 1000;
const REVIEW_JOB_POLL_INTERVAL_MS = 600;
const REVIEW_JOB_MAX_POLLS = 50;

const UNSIGNED_DECIMAL = /^(0|[1-9]\d*)(\.\d+)?$/;
const SIGNED_DECIMAL = /^-?(0|[1-9]\d*)(\.\d+)?$/;
const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

export function App({ api = defaultVaultApi }: { api?: VaultApi }) {
  const [vaultStatus, setVaultStatus] = useState<VaultScreenStatus>("loading");
  const [busy, setBusy] = useState(true);
  const [activeView, setActiveView] = useState<AppView>("overview");
  const [deletingDocumentId, setDeletingDocumentId] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [rememberedOnThisMac, setRememberedOnThisMac] = useState<boolean | null>(
    false,
  );
  const [recoveryConfigured, setRecoveryConfigured] = useState(false);
  const [unassignedDocuments, setUnassignedDocuments] = useState<
    SourceDocumentSummary[]
  >([]);
  const [sourceDocuments, setSourceDocuments] = useState<MoneySourceDocuments[]>([]);
  const [selectedMoneySourceId, setSelectedMoneySourceId] = useState<string | null>(
    null,
  );
  const [loadingDocuments, setLoadingDocuments] = useState(false);
  const [importing, setImporting] = useState(false);
  const [normalizingDocumentId, setNormalizingDocumentId] = useState<string | null>(
    null,
  );
  const [notice, setNotice] = useState<Notice | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [viewer, setViewer] = useState<DocumentViewerState | null>(null);
  const [preview, setPreview] = useState<DocumentPreviewState | null>(null);
  const [unlockingDocument, setUnlockingDocument] = useState<DocumentUnlockState | null>(null);
  const [viewingPage, setViewingPage] = useState(false);
  const [updatingRemembered, setUpdatingRemembered] = useState(false);
  const [savingRecoveryFile, setSavingRecoveryFile] = useState(false);
  const [savingCopyDocumentId, setSavingCopyDocumentId] = useState<string | null>(null);
  const [reviewItems, setReviewItems] = useState<ReviewItemSummary[] | null>(null);
  const [selectedReviewIds, setSelectedReviewIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [reviewDetail, setReviewDetail] = useState<ReviewDetailState | null>(null);
  const [reviewJob, setReviewJob] = useState<ReviewJobPanelState | null>(null);
  const [mutatingReviewItemId, setMutatingReviewItemId] = useState<string | null>(null);
  const [moneyOverview, setMoneyOverview] = useState<MoneyOverview | null>(null);
  const [recentActivity, setRecentActivity] = useState<RecentActivitySummary[] | null>(
    null,
  );
  const [undoingEventId, setUndoingEventId] = useState<string | null>(null);
  const [moneySources, setMoneySources] = useState<MoneySourceSummary[]>([]);
  const [localInbox, setLocalInbox] = useState<LocalInboxStatus | null>(null);
  const [inboxBusy, setInboxBusy] = useState(false);
  const [inboxConfirmingDisable, setInboxConfirmingDisable] = useState(false);
  const [coveragePrompts, setCoveragePrompts] = useState<
    StatementCoveragePrompt[] | null
  >(null);
  const [accountPrompts, setAccountPrompts] = useState<
    AccountConfirmationPrompt[] | null
  >(null);
  const [attentionBusyKey, setAttentionBusyKey] = useState<string | null>(null);
  const [remind, setRemind] = useState<RemindState | null>(null);
  const viewerRequestId = useRef(0);
  const previewRequestId = useRef(0);
  const unlockRequestId = useRef(0);
  const documentLoadRequestId = useRef(0);
  const financeLoadRequestId = useRef(0);
  const reviewDetailRequestId = useRef(0);
  const reviewJobPollTimer = useRef<number | null>(null);
  const selectedMoneySourceIdRef = useRef<string | null>(null);
  const viewerReturnFocus = useRef<HTMLButtonElement | null>(null);
  const documentLoadsAllowed = useRef(false);
  const vaultSessionId = useRef(0);

  useEffect(() => () => {
    vaultSessionId.current += 1;
    unlockRequestId.current += 1;
    previewRequestId.current += 1;
    documentLoadRequestId.current += 1;
    financeLoadRequestId.current += 1;
    reviewDetailRequestId.current += 1;
    if (reviewJobPollTimer.current !== null) {
      window.clearTimeout(reviewJobPollTimer.current);
    }
  }, []);

  const clearViewer = useCallback((restoreFocus = true) => {
    viewerRequestId.current += 1;
    setViewer(null);
    setViewingPage(false);
    if (!restoreFocus) {
      viewerReturnFocus.current = null;
    }
  }, []);

  const clearPreview = useCallback(() => {
    previewRequestId.current += 1;
    setPreview(null);
  }, []);

  const stopReviewJobPolling = useCallback(() => {
    if (reviewJobPollTimer.current !== null) {
      window.clearTimeout(reviewJobPollTimer.current);
      reviewJobPollTimer.current = null;
    }
  }, []);

  const showVaultGate = useCallback((nextStatus: VaultScreenStatus) => {
    const nextSessionId = vaultSessionId.current + 1;
    vaultSessionId.current = nextSessionId;
    documentLoadsAllowed.current = false;
    documentLoadRequestId.current += 1;
    financeLoadRequestId.current += 1;
    reviewDetailRequestId.current += 1;
    stopReviewJobPolling();
    clearViewer(false);
    clearPreview();
    unlockRequestId.current += 1;
    setUnlockingDocument(null);
    setVaultStatus(nextStatus);
    setActiveView("overview");
    setError(null);
    setNotice(null);
    setPassword("");
    setUnassignedDocuments([]);
    setSourceDocuments([]);
    selectedMoneySourceIdRef.current = null;
    setSelectedMoneySourceId(null);
    setLoadingDocuments(false);
    setSavingCopyDocumentId(null);
    setReviewItems(null);
    setSelectedReviewIds(new Set());
    setReviewDetail(null);
    setReviewJob(null);
    setMutatingReviewItemId(null);
    setMoneyOverview(null);
    setRecentActivity(null);
    setUndoingEventId(null);
    setMoneySources([]);
    setLocalInbox(null);
    setInboxBusy(false);
    setInboxConfirmingDisable(false);
    setCoveragePrompts(null);
    setAccountPrompts(null);
    setAttentionBusyKey(null);
    setRemind(null);
    setBusy(false);
    return nextSessionId;
  }, [clearViewer, clearPreview, stopReviewJobPolling]);

  useEffect(() => {
    if (viewer === null && preview === null && viewerReturnFocus.current) {
      viewerReturnFocus.current.focus();
      viewerReturnFocus.current = null;
    }
  }, [viewer, preview]);

  const loadDocuments = useCallback(async (requestedSourceId?: string | null) => {
    if (!documentLoadsAllowed.current) {
      return;
    }
    const sessionId = vaultSessionId.current;
    const requestId = documentLoadRequestId.current + 1;
    const selectedSourceId = requestedSourceId === undefined
      ? selectedMoneySourceIdRef.current
      : requestedSourceId;
    documentLoadRequestId.current = requestId;
    setLoadingDocuments(true);
    try {
      const [sources, unassigned] = await Promise.all([
        api.listMoneySources(),
        api.listUnassignedSourceDocuments(),
      ]);
      const selectedSource = selectedSourceId === null
        ? undefined
        : sources.find((source) => source.moneySourceId === selectedSourceId);
      const selectedDocuments = selectedSource
        ? await api.listSourceDocuments(selectedSource.moneySourceId)
        : null;
      if (
        vaultSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        const nextSelectedSourceId = selectedSource?.moneySourceId ?? null;
        selectedMoneySourceIdRef.current = nextSelectedSourceId;
        setSelectedMoneySourceId(nextSelectedSourceId);
        setSourceDocuments(sources.map((source) => ({
          documents: source.moneySourceId === nextSelectedSourceId
            ? selectedDocuments
            : null,
          source,
        })));
        setUnassignedDocuments(unassigned);
      }
    } catch (nextError) {
      if (
        vaultSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (
        vaultSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setLoadingDocuments(false);
      }
    }
  }, [api]);

  const loadFinanceData = useCallback(async () => {
    if (!documentLoadsAllowed.current) {
      return;
    }
    const sessionId = vaultSessionId.current;
    const requestId = financeLoadRequestId.current + 1;
    financeLoadRequestId.current = requestId;
    try {
      const [
        items,
        overview,
        activity,
        sources,
        inbox,
        coverage,
        accounts,
      ] = await Promise.all([
        api.listReviewItems(),
        api.getMoneyOverview(),
        api.listRecentActivity(),
        api.listMoneySources(),
        api.localInboxStatus(),
        api.listStatementCoveragePrompts(),
        api.listAccountConfirmationPrompts(),
      ]);
      if (
        vaultSessionId.current === sessionId
        && financeLoadRequestId.current === requestId
      ) {
        setReviewItems(items);
        setMoneyOverview(overview);
        setRecentActivity(activity);
        setMoneySources(sources);
        setLocalInbox(inbox);
        setCoveragePrompts(coverage);
        setAccountPrompts(accounts);
        setSelectedReviewIds((current) => new Set(
          [...current].filter((id) =>
            items.some((item) => item.reviewItemId === id)
          ),
        ));
      }
    } catch (nextError) {
      if (
        vaultSessionId.current === sessionId
        && financeLoadRequestId.current === requestId
      ) {
        setError(commandErrorMessage(nextError));
      }
    }
  }, [api]);

  const selectMoneySource = (moneySourceId: string) => {
    if (!documentLoadsAllowed.current) {
      return;
    }
    const sessionId = vaultSessionId.current;
    const requestId = documentLoadRequestId.current + 1;
    documentLoadRequestId.current = requestId;
    selectedMoneySourceIdRef.current = moneySourceId;
    setSelectedMoneySourceId(moneySourceId);
    setSourceDocuments((current) => current.map((entry) => ({
      ...entry,
      documents: null,
    })));
    setError(null);
    setLoadingDocuments(true);
    void api.listSourceDocuments(moneySourceId).then((documents) => {
      if (
        vaultSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setSourceDocuments((current) => current.map((entry) => (
          entry.source.moneySourceId === moneySourceId
            ? { ...entry, documents }
            : entry
        )));
      }
    }).catch((nextError) => {
      if (
        vaultSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setError(commandErrorMessage(nextError));
      }
    }).finally(() => {
      if (
        vaultSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setLoadingDocuments(false);
      }
    });
  };

  const requestVaultLock = useCallback(async () => {
    const sessionId = showVaultGate("loading");
    try {
      const nextStatus = await api.lockVault();
      if (vaultSessionId.current === sessionId) {
        documentLoadsAllowed.current = nextStatus === "unlocked";
        setVaultStatus(nextStatus);
        return nextStatus === "unlocked";
      }
    } catch (nextError) {
      if (vaultSessionId.current !== sessionId) {
        return false;
      }
      try {
        const nextStatus = await api.vaultStatus();
        if (vaultSessionId.current !== sessionId) {
          return false;
        }
        documentLoadsAllowed.current = nextStatus === "unlocked";
        setVaultStatus(nextStatus);
        if (nextStatus === "unlocked") {
          setError(commandErrorMessage(nextError));
          await loadDocuments();
          await loadFinanceData();
          return vaultSessionId.current === sessionId;
        }
      } catch {
        if (vaultSessionId.current === sessionId) {
          setError(commandErrorMessage(nextError));
        }
      }
    }
    return false;
  }, [api, loadDocuments, loadFinanceData, showVaultGate]);

  useEffect(() => {
    if (vaultStatus !== "unlocked") {
      return;
    }

    let active = true;
    let timeout = window.setTimeout(
      lockAfterInactivity,
      VAULT_INACTIVITY_TIMEOUT_MS,
    );
    const resetTimeout = () => {
      window.clearTimeout(timeout);
      timeout = window.setTimeout(lockAfterInactivity, VAULT_INACTIVITY_TIMEOUT_MS);
    };
    const activityEvents = [
      "keydown",
      "pointerdown",
      "pointermove",
      "touchstart",
      "wheel",
    ];
    const activityListenerOptions = { passive: true };
    for (const event of activityEvents) {
      window.addEventListener(event, resetTimeout, activityListenerOptions);
    }

    return () => {
      active = false;
      window.clearTimeout(timeout);
      for (const event of activityEvents) {
        window.removeEventListener(event, resetTimeout);
      }
    };

    function lockAfterInactivity() {
      void requestVaultLock().then((vaultRemainsUnlocked) => {
        if (active && vaultRemainsUnlocked) {
          resetTimeout();
        }
      });
    }
  }, [requestVaultLock, vaultStatus]);

  const refreshVaultStatus = useCallback(async () => {
    const sessionId = vaultSessionId.current;
    setBusy(true);
    setVaultStatus("loading");
    setError(null);
    try {
      const access = await api.vaultAccessStatus();
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      const nextStatus = access.status;
      documentLoadsAllowed.current = nextStatus === "unlocked";
      setRememberedOnThisMac(access.rememberedOnThisMac);
      setRecoveryConfigured(access.recoveryConfigured);
      setVaultStatus(nextStatus);
      if (nextStatus === "unlocked") {
        await loadDocuments();
        await loadFinanceData();
      } else {
        showVaultGate(nextStatus);
      }
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setBusy(false);
      }
    }
  }, [api, loadDocuments, loadFinanceData, showVaultGate]);

  useEffect(() => {
    let active = true;
    let removeListener: (() => void) | undefined;
    void api.onVaultLocked(() => {
      if (active) {
        showVaultGate("locked");
      }
    }).then((remove) => {
      if (active) {
        removeListener = remove;
      } else {
        remove();
      }
    }).catch(() => {
      // Focus and visibility reconciliation remain the fail-closed fallback.
    });

    const reconcileAfterSystemTransition = () => {
      if (document.visibilityState === "visible") {
        void refreshVaultStatus();
      }
    };
    window.addEventListener("focus", reconcileAfterSystemTransition);
    document.addEventListener("visibilitychange", reconcileAfterSystemTransition);
    return () => {
      active = false;
      removeListener?.();
      window.removeEventListener("focus", reconcileAfterSystemTransition);
      document.removeEventListener("visibilitychange", reconcileAfterSystemTransition);
    };
  }, [api, refreshVaultStatus, showVaultGate]);

  useEffect(() => {
    void refreshVaultStatus();
  }, [refreshVaultStatus]);

  const run = async (action: () => Promise<void>, sessionId?: number) => {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (nextError) {
      if (sessionId === undefined || vaultSessionId.current === sessionId) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (sessionId === undefined || vaultSessionId.current === sessionId) {
        setBusy(false);
      }
    }
  };

  const submitPassword = () => {
    if (!password) {
      setError("Enter a password to continue.");
      return;
    }

    const sessionId = vaultSessionId.current;
    void run(async () => {
      const nextStatus =
        vaultStatus === "not_created"
          ? await api.createVault(password)
          : await api.unlockVault(password);
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setPassword("");
      documentLoadsAllowed.current = nextStatus === "unlocked";
      setVaultStatus(nextStatus);
      if (nextStatus === "unlocked") {
        await loadDocuments();
        await loadFinanceData();
      }
    }, sessionId);
  };

  const unlockWithKeychain = () => {
    const sessionId = vaultSessionId.current;
    void run(async () => {
      try {
        const nextStatus = await api.unlockVaultWithKeychain();
        if (vaultSessionId.current !== sessionId) {
          return;
        }
        documentLoadsAllowed.current = nextStatus === "unlocked";
        setVaultStatus(nextStatus);
        if (nextStatus === "unlocked") {
          setPassword("");
          await loadDocuments();
          await loadFinanceData();
        }
      } catch (nextError) {
        if (vaultSessionId.current !== sessionId) {
          return;
        }
        try {
          const access = await api.vaultAccessStatus();
          if (vaultSessionId.current === sessionId) {
            setRememberedOnThisMac(access.rememberedOnThisMac);
          }
        } catch {
          // Keep the original Keychain error as the user-facing outcome.
        }
        throw nextError;
      }
    }, sessionId);
  };

  const updateRemembered = (remembered: boolean) => {
    setUpdatingRemembered(true);
    void run(async () => {
      if (remembered) {
        await api.rememberVaultOnThisMac();
      } else {
        await api.forgetVaultOnThisMac();
      }
      setRememberedOnThisMac(remembered);
      setNotice(
        remembered
          ? {
              body: "This Mac can unlock your Vault through Keychain without running the password check.",
              tone: "success",
              title: "Remembered unlock enabled",
            }
          : {
              body: "Your current Vault stays open. Your password will be required after you lock or restart CanCan.",
              tone: "success",
              title: "Remembered unlock removed",
            },
      );
    }).finally(() => setUpdatingRemembered(false));
  };

  const importDocument = () => {
    setImporting(true);
    void run(async () => {
      setNotice(null);
      const outcome = await api.importSourceDocument();
      setNotice(importNotice(outcome?.status ?? "cancelled"));
      if (outcome) {
        await loadDocuments();
      }
    }).finally(() => setImporting(false));
  };

  const saveRecoveryFile = () => {
    setSavingRecoveryFile(true);
    void run(async () => {
      setNotice(null);
      if (await api.saveRecoveryFile()) {
        setRecoveryConfigured(true);
        setNotice({
          body: "Keep this bearer-secret file somewhere private and separate from your Mac.",
          tone: "success",
          title: "Recovery file saved",
        });
      } else {
        setNotice({
          body: "Your Vault is still usable. This task will stay here until saving succeeds.",
          tone: "attention",
          title: "Recovery is not configured",
        });
      }
    }).finally(() => setSavingRecoveryFile(false));
  };

  const normalizeDocument = (documentId: string) => {
    setNormalizingDocumentId(documentId);
    void run(async () => {
      const outcome = await api.normalizeSourceDocument(documentId);
      const source = sourceDocuments.find(
        (entry) => entry.source.moneySourceId === outcome.moneySourceId,
      )?.source;
      setNotice(routingNotice(outcome, source));
      await loadDocuments(
        outcome.status === "routed" ? outcome.moneySourceId : undefined,
      );
    }).finally(() => setNormalizingDocumentId(null));
  };

  const deleteDocument = (documentId: string) => {
    setDeletingDocumentId(documentId);
    void run(async () => {
      try {
        if (await api.deleteSourceDocument(documentId)) {
          setNotice({
            body: "The encrypted file was removed. Its document history and links remain in CanCan.",
            tone: "success",
            title: "Source file deleted",
          });
        }
      } finally {
        await loadDocuments();
      }
    }).finally(() => setDeletingDocumentId(null));
  };

  const saveSourceCopy = (documentId: string) => {
    const sessionId = vaultSessionId.current;
    setSavingCopyDocumentId(documentId);
    void run(async () => {
      if (
        await api.saveSourceDocumentCopy(documentId)
        && vaultSessionId.current === sessionId
      ) {
        setNotice({
          body: "The copy is outside CanCan’s encrypted Vault and is now your responsibility.",
          tone: "success",
          title: "Copy saved",
        });
      }
    }, sessionId).finally(() => {
      if (vaultSessionId.current === sessionId) {
        setSavingCopyDocumentId(null);
      }
    });
  };

  const trySavedStatementPassword = async (
    documentId: string,
    moneySourceId: string,
    requestId: number,
  ) => {
    setUnlockingDocument((current) => current?.documentId === documentId
      ? { ...current, busy: true, error: null, savedPasswordStatus: null }
      : current);
    try {
      const result = await api.trySavedStatementPassword(documentId, moneySourceId);
      if (unlockRequestId.current !== requestId) {
        return;
      }
      if (result === "unlocked") {
        setUnlockingDocument(null);
        setNotice({
          body: "The saved password worked. This statement is ready to view and route for this Vault session.",
          tone: "success",
          title: "Statement unlocked",
        });
        await loadDocuments();
        return;
      }
      setUnlockingDocument((current) => current?.documentId === documentId
        ? { ...current, busy: false, savedPasswordStatus: result }
        : current);
    } catch (nextError) {
      if (unlockRequestId.current === requestId) {
        setUnlockingDocument((current) => current?.documentId === documentId
          ? { ...current, busy: false, error: commandErrorMessage(nextError) }
          : current);
      }
    }
  };

  const selectUnlockSource = (moneySourceId: string) => {
    const current = unlockingDocument;
    if (!current) {
      return;
    }
    const requestId = unlockRequestId.current + 1;
    unlockRequestId.current = requestId;
    setUnlockingDocument({
      ...current,
      busy: false,
      error: null,
      password: "",
      savedPasswordStatus: null,
      selectedMoneySourceId: moneySourceId,
    });
    if (current.sources?.find((source) => source.moneySourceId === moneySourceId)?.hasSavedPassword) {
      void trySavedStatementPassword(current.documentId, moneySourceId, requestId);
    }
  };

  const loadUnlockSources = (documentId: string, documentTitle: string) => {
    const requestId = unlockRequestId.current + 1;
    unlockRequestId.current = requestId;
    setUnlockingDocument({
      busy: true,
      documentId,
      documentTitle,
      error: null,
      password: "",
      savedPasswordStatus: null,
      selectedMoneySourceId: "",
      sources: null,
    });
    void api.listStatementPasswordSources().then((sources) => {
      if (unlockRequestId.current !== requestId) {
        return;
      }
      const onlySource = sources.length === 1 ? sources[0] : undefined;
      const selectedMoneySourceId = onlySource?.moneySourceId ?? "";
      setUnlockingDocument((current) => current?.documentId === documentId
        ? { ...current, busy: false, selectedMoneySourceId, sources }
        : current);
      if (onlySource?.hasSavedPassword) {
        void trySavedStatementPassword(documentId, selectedMoneySourceId, requestId);
      }
    }).catch((nextError) => {
      if (unlockRequestId.current === requestId) {
        setUnlockingDocument((current) => current?.documentId === documentId
          ? { ...current, busy: false, error: commandErrorMessage(nextError), sources: [] }
          : current);
      }
    });
  };

  const openDocumentUnlock = (document: SourceDocumentSummary) => {
    loadUnlockSources(document.documentId, document.originalFilename);
  };

  const submitDocumentPassword = (updateSavedPassword: boolean) => {
    const current = unlockingDocument;
    if (!current) {
      return;
    }
    if (!current.selectedMoneySourceId) {
      setUnlockingDocument({ ...current, error: "Choose the Money Source for this statement." });
      return;
    }
    if (!current.password) {
      setUnlockingDocument({ ...current, error: "Enter the statement password to continue." });
      return;
    }
    const requestId = unlockRequestId.current + 1;
    unlockRequestId.current = requestId;
    const password = current.password;
    setUnlockingDocument({ ...current, busy: true, error: null, password: "" });
    void api.unlockSourceDocument(
      current.documentId,
      current.selectedMoneySourceId,
      password,
      updateSavedPassword,
    ).then(async () => {
      if (unlockRequestId.current !== requestId) {
        return;
      }
      setUnlockingDocument(null);
      setNotice({
        body: updateSavedPassword
          ? "The verified password replaced this Money Source’s saved password. This statement is ready to view and route."
          : "This statement is ready to view and route until the Vault locks.",
        tone: "success",
        title: "Statement unlocked",
      });
      await loadDocuments();
    }).catch((nextError) => {
      if (unlockRequestId.current === requestId) {
        setUnlockingDocument((latest) => latest?.documentId === current.documentId
          ? { ...latest, busy: false, error: commandErrorMessage(nextError), password: "" }
          : latest);
      }
    });
  };

  const loadViewerPage = (
    documentId: string,
    documentTitle: string,
    pageNumber: number,
  ) => {
    const requestId = viewerRequestId.current + 1;
    viewerRequestId.current = requestId;
    setViewingPage(true);
    void run(async () => {
      try {
        const page = await api.renderSourceDocumentPage(documentId, pageNumber);
        if (viewerRequestId.current === requestId) {
          setViewer({ documentId, documentTitle, page });
        }
      } catch (nextError) {
        if (viewerRequestId.current !== requestId) {
          return;
        }
        if (viewer !== null) {
          clearViewer();
        }
        throw nextError;
      }
    }).finally(() => {
      if (viewerRequestId.current === requestId) {
        setViewingPage(false);
      }
    });
  };

  const loadDocumentPreview = (documentId: string, documentTitle: string) => {
    const requestId = previewRequestId.current + 1;
    previewRequestId.current = requestId;
    void run(async () => {
      try {
        const nextPreview = await api.previewSourceDocument(documentId);
        if (previewRequestId.current === requestId) {
          setPreview({ documentTitle, preview: nextPreview });
        }
      } catch (nextError) {
        if (previewRequestId.current !== requestId) {
          return;
        }
        throw nextError;
      }
    });
  };

  const navigate = (view: AppView) => {
    setActiveView(view);
    setError(null);
    setNotice(null);
  };

  const openReviewDetail = (item: ReviewItemSummary) => {
    const sessionId = vaultSessionId.current;
    const requestId = reviewDetailRequestId.current + 1;
    reviewDetailRequestId.current = requestId;
    setReviewDetail({
      candidates: null,
      confirmingRemove: false,
      detail: null,
      editing: null,
      reviewItemId: item.reviewItemId,
      summary: item,
    });
    void Promise.all([
      api.getReviewDetail(item.reviewItemId),
      api.listRelationshipCandidates(item.reviewItemId, item.recordVersion),
    ]).then(([detail, candidates]) => {
      if (
        vaultSessionId.current !== sessionId
        || reviewDetailRequestId.current !== requestId
      ) {
        return;
      }
      if (detail === null) {
        setReviewDetail(null);
        setNotice({
          body: "That item is no longer waiting for review.",
          tone: "attention",
          title: "Already resolved",
        });
        void loadFinanceData();
        return;
      }
      setReviewDetail({
        candidates,
        confirmingRemove: false,
        detail,
        editing: null,
        reviewItemId: item.reviewItemId,
        summary: item,
      });
    }).catch((nextError) => {
      if (
        vaultSessionId.current === sessionId
        && reviewDetailRequestId.current === requestId
      ) {
        reviewDetailRequestId.current += 1;
        setReviewDetail(null);
        setError(commandErrorMessage(nextError));
      }
    });
  };

  const closeReviewDetail = () => {
    reviewDetailRequestId.current += 1;
    setReviewDetail(null);
  };

  const toggleReviewSelection = (reviewItemId: string) => {
    setSelectedReviewIds((current) => {
      const next = new Set(current);
      if (next.has(reviewItemId)) {
        next.delete(reviewItemId);
      } else {
        next.add(reviewItemId);
      }
      return next;
    });
  };

  const handleReviewConflict = async (reason: string | null) => {
    setNotice({
      body: reviewConflictMessage(reason),
      tone: "attention",
      title: "Couldn’t apply that change",
    });
    reviewDetailRequestId.current += 1;
    setReviewDetail(null);
    await loadFinanceData();
  };

  const startReviewEdit = () => {
    setReviewDetail((current) => current === null
      ? current
      : {
          ...current,
          confirmingRemove: false,
          editing: {
            accountBalanceDelta: "",
            amountValue: current.summary.amountValue ?? "",
            error: null,
            postedOn: current.summary.postedOn ?? "",
            saving: false,
          },
        });
  };

  const cancelReviewEdit = () => {
    setReviewDetail((current) => current === null
      ? current
      : { ...current, editing: null });
  };

  const changeReviewEdit = (
    patch: Partial<Pick<
      ReviewEditState,
      "accountBalanceDelta" | "amountValue" | "postedOn"
    >>,
  ) => {
    setReviewDetail((current) => current?.editing
      ? { ...current, editing: { ...current.editing, ...patch, error: null } }
      : current);
  };

  const setReviewEditError = (message: string) => {
    setReviewDetail((current) => current?.editing
      ? { ...current, editing: { ...current.editing, error: message, saving: false } }
      : current);
  };

  const saveReviewEdit = async () => {
    const state = reviewDetail;
    if (!state?.editing || mutatingReviewItemId !== null) {
      return;
    }
    const { editing } = state;
    const patch: Omit<EditReviewRecordArgs, "expectedRecordVersion" | "reviewItemId"> = {};
    const amountValue = editing.amountValue.trim();
    const postedOn = editing.postedOn.trim();
    const accountBalanceDelta = editing.accountBalanceDelta.trim();
    if (amountValue !== (state.summary.amountValue ?? "")) {
      if (!UNSIGNED_DECIMAL.test(amountValue)) {
        setReviewEditError("Amount must be a positive decimal, such as 128.50.");
        return;
      }
      patch.amountValue = amountValue;
    }
    if (postedOn !== (state.summary.postedOn ?? "")) {
      if (!ISO_DATE.test(postedOn) || !isRealIsoDate(postedOn)) {
        setReviewEditError("Date must use the YYYY-MM-DD format, such as 2026-07-19.");
        return;
      }
      patch.postedOn = postedOn;
    }
    if (accountBalanceDelta !== "") {
      if (!SIGNED_DECIMAL.test(accountBalanceDelta)) {
        setReviewEditError("Balance change must be a signed decimal, such as -128.50.");
        return;
      }
      patch.accountBalanceDelta = accountBalanceDelta;
    }
    if (
      patch.amountValue === undefined
      && patch.postedOn === undefined
      && patch.accountBalanceDelta === undefined
    ) {
      setReviewEditError("Change something before saving.");
      return;
    }

    const sessionId = vaultSessionId.current;
    setMutatingReviewItemId(state.reviewItemId);
    setReviewDetail((current) => current?.editing
      ? { ...current, editing: { ...current.editing, saving: true } }
      : current);
    try {
      const outcome = await api.editReviewRecord(
        state.reviewItemId,
        state.summary.recordVersion,
        patch,
      );
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      if (outcome.status === "conflict") {
        setMutatingReviewItemId(null);
        await handleReviewConflict(outcome.reason);
        return;
      }
      setNotice({
        body: "CanCan will use the corrected details from now on.",
        tone: "success",
        title: "Edit saved",
      });
      setMutatingReviewItemId(null);
      await loadFinanceData();
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      if (outcome.reviewItemId !== null) {
        const refreshed = await api.listReviewItems();
        if (vaultSessionId.current !== sessionId) {
          return;
        }
        const nextItem = refreshed.find(
          (item) => item.reviewItemId === outcome.reviewItemId,
        );
        if (nextItem) {
          openReviewDetail(nextItem);
        } else {
          closeReviewDetail();
        }
      } else {
        closeReviewDetail();
      }
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setMutatingReviewItemId(null);
        setReviewDetail((current) => current?.editing
          ? { ...current, editing: { ...current.editing, saving: false } }
          : current);
        setError(commandErrorMessage(nextError));
      }
    }
  };

  const requestReviewRemove = () => {
    setReviewDetail((current) => current === null
      ? current
      : { ...current, confirmingRemove: true, editing: null });
  };

  const cancelReviewRemove = () => {
    setReviewDetail((current) => current === null
      ? current
      : { ...current, confirmingRemove: false });
  };

  const confirmReviewRemove = async () => {
    const state = reviewDetail;
    if (!state || mutatingReviewItemId !== null) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setMutatingReviewItemId(state.reviewItemId);
    try {
      const outcome = await api.removeReviewRecord(
        state.reviewItemId,
        state.summary.recordVersion,
      );
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setMutatingReviewItemId(null);
      if (outcome.status === "conflict") {
        await handleReviewConflict(outcome.reason);
        return;
      }
      reviewDetailRequestId.current += 1;
      setReviewDetail(null);
      setSelectedReviewIds((current) => {
        const next = new Set(current);
        next.delete(state.reviewItemId);
        return next;
      });
      setNotice({
        body: "The staged record is out of the queue. Its history stays in your audit trail.",
        tone: "success",
        title: "Record removed",
      });
      await loadFinanceData();
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setMutatingReviewItemId(null);
        setError(commandErrorMessage(nextError));
      }
    }
  };

  const acceptReviewCandidate = async (candidate: RelationshipCandidateSummary) => {
    const state = reviewDetail;
    if (!state || mutatingReviewItemId !== null) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setMutatingReviewItemId(state.reviewItemId);
    try {
      const outcome = await api.acceptReviewRelationship(
        state.reviewItemId,
        state.summary.recordVersion,
        candidate.recordId,
        candidate.recordVersion,
      );
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setMutatingReviewItemId(null);
      if (outcome.status === "conflict") {
        await handleReviewConflict(outcome.reason);
        return;
      }
      setSelectedReviewIds((current) => {
        const next = new Set(current);
        next.add(state.reviewItemId);
        const linked = reviewItems?.find(
          (item) => item.recordId === candidate.recordId,
        );
        if (linked) {
          next.add(linked.reviewItemId);
        }
        return next;
      });
      setNotice({
        body: "CanCan will treat them as one event. Add both together when you’re ready.",
        tone: "success",
        title: "Linked",
      });
      await loadFinanceData();
      if (vaultSessionId.current === sessionId) {
        openReviewDetail(state.summary);
      }
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setMutatingReviewItemId(null);
        setError(commandErrorMessage(nextError));
      }
    }
  };

  const finishReviewJob = (summary: ReviewJobSummary) => {
    if (summary.status === "succeeded") {
      setReviewJob({ jobId: summary.jobId, outcomes: summary.outcomes, status: "done" });
    } else {
      setReviewJob({ jobId: summary.jobId, outcomes: summary.outcomes, status: "failed" });
    }
    setSelectedReviewIds(new Set());
    void loadFinanceData();
  };

  const pollReviewJob = (jobId: string, attempt: number) => {
    stopReviewJobPolling();
    if (attempt >= REVIEW_JOB_MAX_POLLS) {
      setReviewJob({ jobId, outcomes: [], status: "failed" });
      setSelectedReviewIds(new Set());
      void loadFinanceData();
      return;
    }
    const sessionId = vaultSessionId.current;
    reviewJobPollTimer.current = window.setTimeout(() => {
      reviewJobPollTimer.current = null;
      void api.getReviewJob(jobId).then((summary) => {
        if (vaultSessionId.current !== sessionId) {
          return;
        }
        if (summary === null) {
          setReviewJob({ jobId, outcomes: [], status: "failed" });
          setSelectedReviewIds(new Set());
          void loadFinanceData();
          return;
        }
        if (
          summary.status === "queued"
          || summary.status === "running"
        ) {
          pollReviewJob(jobId, attempt + 1);
          return;
        }
        finishReviewJob(summary);
      }).catch(() => {
        if (vaultSessionId.current === sessionId) {
          pollReviewJob(jobId, attempt + 1);
        }
      });
    }, REVIEW_JOB_POLL_INTERVAL_MS);
  };

  const enqueueReviewBatch = () => {
    if (reviewItems === null || reviewJob?.status === "running") {
      return;
    }
    const ids = reviewItems
      .filter((item) => selectedReviewIds.has(item.reviewItemId))
      .map((item) => item.reviewItemId);
    if (ids.length === 0) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setNotice(null);
    setReviewJob({ jobId: "", outcomes: [], status: "running" });
    void api.enqueueCommitReviewBatch(ids).then((summary) => {
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setReviewJob({ jobId: summary.jobId, outcomes: [], status: "running" });
      if (summary.status === "queued" || summary.status === "running") {
        pollReviewJob(summary.jobId, 0);
        return;
      }
      finishReviewJob(summary);
    }).catch((nextError) => {
      if (vaultSessionId.current === sessionId) {
        setReviewJob(null);
        setError(commandErrorMessage(nextError));
      }
    });
  };

  const undoCommittedEvent = (eventId: string) => {
    if (undoingEventId !== null) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setUndoingEventId(eventId);
    void run(async () => {
      const outcome = await api.undoCommittedEvent(eventId);
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setNotice(
        outcome.status === "undone"
          ? {
              body: "CanCan added the reversal to your recent activity.",
              tone: "success",
              title: "Undone",
            }
          : {
              body: "That event was already reversed.",
              tone: "attention",
              title: "Already undone",
            },
      );
      await loadFinanceData();
    }, sessionId).finally(() => {
      if (vaultSessionId.current === sessionId) {
        setUndoingEventId(null);
      }
    });
  };

  const chooseInboxFolder = async () => {
    if (inboxBusy) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setInboxBusy(true);
    try {
      const status = await api.chooseLocalInboxRoot();
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      if (status === null) {
        return;
      }
      setLocalInbox(status);
      setInboxConfirmingDisable(false);
      setNotice({
        body: "New statements you save to Inbox are added for you. The folder stays outside your encrypted Vault.",
        tone: "success",
        title: "CanCan Inbox is on",
      });
      await loadFinanceData();
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setInboxBusy(false);
      }
    }
  };

  const rescanInbox = async () => {
    if (inboxBusy) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setInboxBusy(true);
    try {
      const summary = await api.rescanLocalInbox();
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setLocalInbox((current) => current === null
        ? current
        : { ...current, lastScan: summary });
      setNotice({
        body: localInboxScanSummaryText(summary),
        tone: "success",
        title: "Inbox checked",
      });
      await loadFinanceData();
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setInboxBusy(false);
      }
    }
  };

  const disableInbox = async () => {
    if (inboxBusy) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setInboxBusy(true);
    try {
      const status = await api.disableLocalInbox();
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setLocalInbox(status);
      setInboxConfirmingDisable(false);
      setNotice({
        body: "Your Cancan folder and its files stay untouched. You can choose it again anytime.",
        tone: "success",
        title: "CanCan Inbox is off",
      });
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setInboxBusy(false);
      }
    }
  };

  const decideCoverage = async (
    prompt: StatementCoveragePrompt,
    action: "not_expected" | "remind_later",
    remindAfter?: string,
  ) => {
    if (attentionBusyKey !== null) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setAttentionBusyKey(coverageKey(prompt));
    try {
      await api.recordStatementCoverageDecision({
        accountId: prompt.accountId,
        action,
        documentType: prompt.documentType,
        moneySourceId: prompt.moneySourceId,
        ...(remindAfter === undefined ? {} : { remindAfter }),
        statementPeriodFrom: prompt.statementPeriodFrom,
        statementPeriodTo: prompt.statementPeriodTo,
      });
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setRemind(null);
      setNotice(action === "not_expected"
        ? {
            body: "CanCan won’t ask about that period again.",
            tone: "success",
            title: "Got it",
          }
        : {
            body: `CanCan will ask again after ${formatLedgerDate(remindAfter ?? "")}.`,
            tone: "success",
            title: "Reminder saved",
          });
      await loadFinanceData();
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setRemind((current) => current === null
          ? current
          : { ...current, saving: false });
        setNotice({
          body: commandErrorMessage(nextError),
          tone: "attention",
          title: "Couldn’t save that",
        });
        await loadFinanceData();
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setAttentionBusyKey(null);
      }
    }
  };

  const startRemind = (prompt: StatementCoveragePrompt) => {
    setRemind({ date: "", error: null, key: coverageKey(prompt), saving: false });
  };

  const saveRemind = () => {
    const state = remind;
    if (!state || state.saving || attentionBusyKey !== null) {
      return;
    }
    const prompt = coveragePrompts?.find(
      (candidate) => coverageKey(candidate) === state.key,
    );
    if (!prompt) {
      setRemind(null);
      return;
    }
    const date = state.date.trim();
    if (!ISO_DATE.test(date) || !isRealIsoDate(date)) {
      setRemind((current) => current === null
        ? current
        : { ...current, error: "Date must use the YYYY-MM-DD format, such as 2026-09-01." });
      return;
    }
    if (date <= localIsoToday()) {
      setRemind((current) => current === null
        ? current
        : { ...current, error: "Pick a future date." });
      return;
    }
    setRemind({ ...state, date, saving: true });
    void decideCoverage(prompt, "remind_later", date);
  };

  const confirmAccounts = async (prompt: AccountConfirmationPrompt) => {
    if (attentionBusyKey !== null) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setAttentionBusyKey(`account:${prompt.moneySourceId}`);
    try {
      const outcome = await api.confirmCandidateAccounts(
        prompt.moneySourceId,
        prompt.candidateAccounts.map((account) => account.accountId),
      );
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      if (outcome.status === "conflict") {
        setNotice({
          body: "CanCan reloaded the latest account list. Check it and confirm again.",
          tone: "attention",
          title: "That account list changed",
        });
      } else {
        setNotice(outcome.status === "already_confirmed"
          ? {
              body: "Those accounts were already confirmed.",
              tone: "success",
              title: "Already confirmed",
            }
          : {
              body: "Their records can now be added to your ledger.",
              tone: "success",
              title: "Accounts confirmed",
            });
      }
      await loadFinanceData();
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setNotice({
          body: commandErrorMessage(nextError),
          tone: "attention",
          title: "Couldn’t confirm those accounts",
        });
        await loadFinanceData();
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setAttentionBusyKey(null);
      }
    }
  };

  const modalOpen = viewer !== null
    || preview !== null
    || unlockingDocument !== null;
  const unlocked = vaultStatus === "unlocked";

  return (
    <AppShell>
      <VaultSpine
        activeView={activeView}
        inert={modalOpen}
        onNavigate={navigate}
        reviewCount={unlocked ? reviewItems?.length ?? null : null}
        vaultStatus={vaultStatus}
      />

      <section
        aria-hidden={modalOpen ? true : undefined}
        className="ledger"
        inert={modalOpen}
        aria-busy={vaultStatus === "loading"}
      >
        {error ? (
          <Feedback tone="error" title="Something needs your attention" body={error} action={() => void refreshVaultStatus()} />
        ) : null}

        {vaultStatus === "loading" ? (
          <VaultGate busy title="Checking your Vault" body="Confirming the local Vault state before showing evidence." />
        ) : null}

        {vaultStatus === "not_created" || vaultStatus === "locked" ? (
          <VaultGate
            busy={busy}
            body={vaultStatus === "not_created" ? "Create a local Vault before adding your first statement or export." : "Unlock your local Vault to add a file or check its routing."}
            password={password}
            rememberedOnThisMac={rememberedOnThisMac}
            title={vaultStatus === "not_created" ? "Create your Vault" : "Unlock your Vault"}
            onPasswordChange={setPassword}
            onSubmit={submitPassword}
            onUnlockWithKeychain={vaultStatus === "locked" ? unlockWithKeychain : undefined}
          />
        ) : null}

        {unlocked && activeView === "overview" ? (
          <OverviewView
            accountPrompts={accountPrompts ?? []}
            attentionBusyKey={attentionBusyKey}
            coveragePrompts={coveragePrompts ?? []}
            loading={moneyOverview === null && recentActivity === null}
            moneyOverview={moneyOverview}
            moneySources={moneySources}
            notice={notice}
            onAddFile={() => navigate("sources")}
            onCancelRemind={() => setRemind(null)}
            onChangeRemindDate={(value) => setRemind((current) => current === null
              ? current
              : { ...current, date: value, error: null })}
            onConfirmAccounts={(prompt) => void confirmAccounts(prompt)}
            onCoverageNotExpected={(prompt) => void decideCoverage(prompt, "not_expected")}
            onLock={() => void requestVaultLock()}
            onOpenReview={() => navigate("review")}
            onOpenSources={() => navigate("sources")}
            onRefresh={() => void refreshVaultStatus()}
            onSaveRemind={saveRemind}
            onStartRemind={startRemind}
            onUndo={undoCommittedEvent}
            recentActivity={recentActivity}
            remind={remind}
            reviewCount={reviewItems?.length ?? null}
            undoingEventId={undoingEventId}
          />
        ) : null}

        {unlocked && activeView === "review" ? (
          <ReviewView
            detail={reviewDetail}
            items={reviewItems}
            job={reviewJob}
            mutatingItemId={mutatingReviewItemId}
            notice={notice}
            onAcceptCandidate={(candidate) => void acceptReviewCandidate(candidate)}
            onCancelEdit={cancelReviewEdit}
            onCancelRemove={cancelReviewRemove}
            onClearSelection={() => setSelectedReviewIds(new Set())}
            onCloseDetail={closeReviewDetail}
            onConfirmRemove={() => void confirmReviewRemove()}
            onEditChange={changeReviewEdit}
            onEnqueue={enqueueReviewBatch}
            onLock={() => void requestVaultLock()}
            onOpenDetail={openReviewDetail}
            onRefresh={() => void refreshVaultStatus()}
            onRemove={requestReviewRemove}
            onSaveEdit={() => void saveReviewEdit()}
            onSelectAll={() => setSelectedReviewIds(new Set(
              reviewItems?.map((item) => item.reviewItemId) ?? [],
            ))}
            onStartEdit={startReviewEdit}
            onToggleSelect={toggleReviewSelection}
            selectedIds={selectedReviewIds}
          />
        ) : null}

        {unlocked && activeView === "sources" ? (
          <SourcesView
            busy={busy}
            deletingDocumentId={deletingDocumentId}
            importing={importing}
            inbox={localInbox}
            inboxBusy={inboxBusy}
            inboxConfirmingDisable={inboxConfirmingDisable}
            loadingDocuments={loadingDocuments}
            normalizingDocumentId={normalizingDocumentId}
            notice={notice}
            onDelete={deleteDocument}
            onImport={importDocument}
            onInboxCancelDisable={() => setInboxConfirmingDisable(false)}
            onInboxChoose={() => void chooseInboxFolder()}
            onInboxConfirmDisable={() => void disableInbox()}
            onInboxRequestDisable={() => setInboxConfirmingDisable(true)}
            onInboxRescan={() => void rescanInbox()}
            onLock={() => void requestVaultLock()}
            onNormalize={normalizeDocument}
            onOpenUnlock={openDocumentUnlock}
            onRefresh={() => void refreshVaultStatus()}
            onRememberedChange={updateRemembered}
            onSelectMoneySource={selectMoneySource}
            onSaveRecoveryFile={saveRecoveryFile}
            onSaveSourceCopy={saveSourceCopy}
            onView={(document, trigger) => {
              viewerReturnFocus.current = trigger;
              if (document.mimeType === "text/csv") {
                loadDocumentPreview(document.documentId, document.originalFilename);
              } else {
                loadViewerPage(document.documentId, document.originalFilename, 1);
              }
            }}
            recoveryConfigured={recoveryConfigured}
            rememberedOnThisMac={rememberedOnThisMac}
            savingCopyDocumentId={savingCopyDocumentId}
            savingRecoveryFile={savingRecoveryFile}
            selectedMoneySourceId={selectedMoneySourceId}
            sourceDocuments={sourceDocuments}
            unassignedDocuments={unassignedDocuments}
            updatingRemembered={updatingRemembered}
          />
        ) : null}
      </section>
      {unlocked && viewer ? (
        <DocumentViewer
          onClose={clearViewer}
          onPage={(pageNumber) => {
            if (viewer) {
              loadViewerPage(viewer.documentId, viewer.documentTitle, pageNumber);
            }
          }}
          viewer={viewer}
          viewingPage={viewingPage}
        />
      ) : null}
      {unlocked && preview ? (
        <DocumentPreview
          onClose={clearPreview}
          state={preview}
        />
      ) : null}
      {unlocked && unlockingDocument ? (
        <DocumentUnlock
          onClose={() => {
            unlockRequestId.current += 1;
            setUnlockingDocument(null);
          }}
          onPasswordChange={(nextPassword) => setUnlockingDocument((current) => current
            ? { ...current, error: null, password: nextPassword }
            : current)}
          onRetrySources={() => {
            if (unlockingDocument) {
              loadUnlockSources(unlockingDocument.documentId, unlockingDocument.documentTitle);
            }
          }}
          onSourceChange={selectUnlockSource}
          onSubmit={submitDocumentPassword}
          state={unlockingDocument}
        />
      ) : null}
    </AppShell>
  );
}

function isRealIsoDate(value: string): boolean {
  const date = new Date(`${value}T00:00:00.000Z`);
  return !Number.isNaN(date.getTime()) && date.toISOString().slice(0, 10) === value;
}
