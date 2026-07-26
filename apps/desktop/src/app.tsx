import { AppShell } from "@cancan/ui";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  EditReviewRecordArgs,
  MoneyOverview,
  MoneySourceSummary,
  RecentActivitySummary,
  RelationshipCandidateSummary,
  RenderedDocumentPage,
  ReviewItemSummary,
  ReviewJobSummary,
  SourceDocumentImportOutcome,
  SourceDocumentPreview,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
  StatementPasswordSourceSummary,
} from "./command-contracts";
import { Feedback, type Notice } from "./feedback";
import { reviewConflictMessage } from "./format";
import { OverviewView } from "./overview";
import {
  ReviewView,
  type ReviewDetailState,
  type ReviewEditState,
  type ReviewJobPanelState,
} from "./review";
import {
  commandErrorMessage,
  createVaultApi,
  type VaultApi,
} from "./vault-api";
import { VaultSpine, type AppView, type VaultScreenStatus } from "./vault-spine";

export type { Notice } from "./feedback";
export type { VaultScreenStatus } from "./vault-spine";

export interface DocumentViewerState {
  documentId: string;
  documentTitle: string;
  page: RenderedDocumentPage;
}

export interface DocumentPreviewState {
  documentTitle: string;
  preview: SourceDocumentPreview;
}

export interface DocumentUnlockState {
  busy: boolean;
  documentId: string;
  documentTitle: string;
  error: string | null;
  password: string;
  savedPasswordStatus: "invalid" | "unavailable" | null;
  selectedMoneySourceId: string;
  sources: StatementPasswordSourceSummary[] | null;
}

export interface MoneySourceDocuments {
  documents: SourceDocumentSummary[] | null;
  source: MoneySourceSummary;
}

export interface SourcesViewProps {
  busy: boolean;
  deletingDocumentId: string | null;
  importing: boolean;
  loadingDocuments: boolean;
  normalizingDocumentId: string | null;
  notice: Notice | null;
  onDelete: (documentId: string) => void;
  onImport: () => void;
  onLock: () => void;
  onNormalize: (documentId: string) => void;
  onOpenUnlock: (document: SourceDocumentSummary) => void;
  onRefresh: () => void;
  onRememberedChange: (remembered: boolean) => void;
  onSelectMoneySource: (moneySourceId: string) => void;
  onSaveRecoveryFile: () => void;
  onSaveSourceCopy: (documentId: string) => void;
  onView: (
    document: SourceDocumentSummary,
    trigger: HTMLButtonElement,
  ) => void;
  recoveryConfigured: boolean;
  rememberedOnThisMac: boolean | null;
  savingCopyDocumentId: string | null;
  savingRecoveryFile: boolean;
  selectedMoneySourceId: string | null;
  sourceDocuments: MoneySourceDocuments[];
  unassignedDocuments: SourceDocumentSummary[];
  updatingRemembered: boolean;
}

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
      const [items, overview, activity] = await Promise.all([
        api.listReviewItems(),
        api.getMoneyOverview(),
        api.listRecentActivity(),
      ]);
      if (
        vaultSessionId.current === sessionId
        && financeLoadRequestId.current === requestId
      ) {
        setReviewItems(items);
        setMoneyOverview(overview);
        setRecentActivity(activity);
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
        documentLoadRequestId.current += 1;
        financeLoadRequestId.current += 1;
        reviewDetailRequestId.current += 1;
        stopReviewJobPolling();
        setUnassignedDocuments([]);
        setSourceDocuments([]);
        selectedMoneySourceIdRef.current = null;
        setSelectedMoneySourceId(null);
        setReviewItems(null);
        setSelectedReviewIds(new Set());
        setReviewDetail(null);
        setReviewJob(null);
        setMoneyOverview(null);
        setRecentActivity(null);
        clearViewer(false);
        clearPreview();
        unlockRequestId.current += 1;
        setUnlockingDocument(null);
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
  }, [clearViewer, clearPreview, loadDocuments, loadFinanceData, stopReviewJobPolling]);

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
            loading={moneyOverview === null && recentActivity === null}
            moneyOverview={moneyOverview}
            notice={notice}
            onLock={() => void requestVaultLock()}
            onOpenReview={() => navigate("review")}
            onOpenSources={() => navigate("sources")}
            onRefresh={() => void refreshVaultStatus()}
            onUndo={undoCommittedEvent}
            recentActivity={recentActivity}
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
            loadingDocuments={loadingDocuments}
            normalizingDocumentId={normalizingDocumentId}
            notice={notice}
            onDelete={deleteDocument}
            onImport={importDocument}
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

export function SourcesView(props: SourcesViewProps) {
  return (
    <>
      <header className="ledger-header">
        <div>
          <p className="ledger-eyebrow">Sources / Evidence</p>
          <h1>Secure file intake</h1>
        </div>
        <div className="ledger-actions">
          <label className="remember-vault-control">
            <input
              checked={props.rememberedOnThisMac === true}
              disabled={props.busy || props.normalizingDocumentId !== null || props.rememberedOnThisMac === null}
              onChange={(event) => props.onRememberedChange(event.target.checked)}
              type="checkbox"
            />
            <span>{props.updatingRemembered ? "Updating Keychain…" : props.rememberedOnThisMac === null ? "Keychain unavailable" : "Remember on this Mac"}</span>
          </label>
          <button className="button button-quiet" disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onLock} type="button">
            Lock Vault
          </button>
          <button className="button button-primary" disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onImport} type="button">
            {props.importing ? "Opening picker…" : "Add file"}
          </button>
        </div>
      </header>

      <section className="intake-content" aria-label="Manual import">
        {!props.recoveryConfigured ? (
          <section className="todo-panel" aria-labelledby="todo-heading">
            <div className="todo-panel-heading">
              <h2 id="todo-heading"><span className="panel-dot panel-dot-amber" aria-hidden="true" />To do</h2>
              <span className="attention-count" aria-label="1 task">1</span>
            </div>
            <ul className="todo-list">
              <li className="todo-row">
                <div>
                  <p className="todo-title">Save your recovery file</p>
                  <p className="todo-copy">Use it to recover your Vault if you lose access to this Mac or forget your password. Anyone with the file can recover compatible Vault data, so store it privately.</p>
                </div>
                <button className="button button-primary" disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onSaveRecoveryFile} type="button">
                  {props.savingRecoveryFile ? "Saving…" : "Save recovery file"}
                </button>
              </li>
            </ul>
          </section>
        ) : null}

        <section className="intake-intro">
          <h2>Add a statement or export</h2>
          <p>Choose a PDF, CSV, PNG, or JPEG. CanCan saves it in your Vault before checking its configured source.</p>
        </section>

        {props.notice ? <Feedback {...props.notice} /> : null}

        <section className="source-panel" aria-labelledby="sources-heading">
          <div className="source-panel-heading">
            <h2 id="sources-heading"><span className="panel-dot panel-dot-emerald" aria-hidden="true" />Money Sources</h2>
            <span className="source-count" aria-label={`${props.sourceDocuments.length} ${props.sourceDocuments.length === 1 ? "source" : "sources"}`}>
              {props.sourceDocuments.length}
            </span>
          </div>
          {props.loadingDocuments ? <p className="panel-status" role="status">Refreshing sources…</p> : null}
          {!props.loadingDocuments && props.sourceDocuments.length === 0 ? (
            <p className="panel-status">No Money Sources are configured yet.</p>
          ) : null}
          {props.sourceDocuments.map(({ documents, source }) => {
            const selected = props.selectedMoneySourceId === source.moneySourceId;
            return (
              <section className="source-documents" key={source.moneySourceId}>
                <div className="source-documents-heading">
                  <div>
                    <h3>{source.displayName}</h3>
                    <p>{sourceTypeLabel(source.sourceType)}</p>
                  </div>
                  <button
                    aria-expanded={selected}
                    aria-label={`View documents for ${source.displayName}`}
                    className="button button-quiet"
                    disabled={props.loadingDocuments}
                    onClick={() => props.onSelectMoneySource(source.moneySourceId)}
                    type="button"
                  >
                    {selected && documents !== null
                      ? "Refresh documents"
                      : "View documents"}
                  </button>
                </div>
              {selected && documents === null && props.loadingDocuments ? (
                <p className="panel-status" role="status">Loading documents…</p>
              ) : null}
              {selected && documents?.length === 0 ? (
                <p className="panel-status">No routed documents yet.</p>
              ) : null}
              {selected && documents && documents.length > 0 ? (
                <EvidenceDocumentGroups documents={documents} props={props} showRouting={false} />
              ) : null}
              </section>
            );
          })}
        </section>

        <section className="attention-panel" aria-labelledby="attention-heading">
          <div className="attention-panel-heading">
            <h2 id="attention-heading"><span className="panel-dot panel-dot-amber" aria-hidden="true" />Needs attention</h2>
            <span className="attention-count" aria-label={`${props.unassignedDocuments.length} documents`}>
              {props.unassignedDocuments.length}
            </span>
          </div>

          {!props.loadingDocuments && props.unassignedDocuments.length === 0 ? (
            <p className="panel-status">No evidence needs your attention.</p>
          ) : null}
          {props.unassignedDocuments.length > 0 ? (
            <EvidenceDocumentGroups documents={props.unassignedDocuments} props={props} showRouting />
          ) : null}
        </section>
      </section>
    </>
  );
}

function EvidenceDocumentGroups({
  documents,
  props,
  showRouting,
}: {
  documents: SourceDocumentSummary[];
  props: SourcesViewProps;
  showRouting: boolean;
}) {
  return groupEvidenceByMonth(documents).map((group) => (
    <section className="evidence-group" key={group.key}>
      <p className="evidence-group-label">
        {group.label}
        <span className="evidence-group-count">
          {group.documents.length} {group.documents.length === 1 ? "document" : "documents"}
        </span>
      </p>
      <ul className="evidence-list">
        {group.documents.map((document) => {
          const passwordRequired = document.documentStatus === "password_required";
          const protectedUnlocked = document.documentStatus === "protected_unlocked";
          const fileAvailable = document.fileState === "available";
          const viewingAvailable = fileAvailable
            && (document.documentStatus === "ready" || protectedUnlocked);
          const routingAvailable = fileAvailable
            && (document.documentStatus === "ready" || protectedUnlocked);
          const attentionRequired = passwordRequired
            || document.documentStatus === "inspection_failed";
          const deleting = props.deletingDocumentId === document.documentId;
          const normalizing = props.normalizingDocumentId === document.documentId;
          const savingCopy = props.savingCopyDocumentId === document.documentId;
          return (
            <li className="evidence-row" key={document.documentId}>
              <span className="document-kind" aria-hidden="true">{document.mimeType === "application/pdf" ? "PDF" : document.mimeType === "text/csv" ? "CSV" : document.mimeType === "image/png" ? "PNG" : "JPEG"}</span>
              <div className="evidence-details">
                <p>{document.originalFilename}</p>
                <span className="evidence-meta">{evidenceMeta(document)}</span>
              </div>
              <p className={`doc-status doc-status-${attentionRequired ? "attention" : document.fileState}`}>
                <span className="doc-status-dot" aria-hidden="true" />
                {documentStatusLabel(document)}
              </p>
              <div className="evidence-actions">
                {passwordRequired ? (
                  <button className="button button-primary" disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onOpenUnlock(document)} type="button">
                    Unlock
                  </button>
                ) : (
                  <button className="button button-quiet" disabled={!viewingAvailable || props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={(event) => props.onView(document, event.currentTarget)} type="button">
                    {!viewingAvailable ? "View unavailable" : "View document"}
                  </button>
                )}
                {showRouting && !passwordRequired && document.documentStatus !== "inspection_failed" ? (
                  <button className="button button-quiet" disabled={!routingAvailable || props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onNormalize(document.documentId)} type="button">
                    {!routingAvailable ? "Routing unavailable" : normalizing ? "Checking…" : "Check routing"}
                  </button>
                ) : null}
                {fileAvailable ? (
                  <button className="button button-quiet" disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onSaveSourceCopy(document.documentId)} type="button">
                    {savingCopy ? "Saving copy…" : "Save a copy"}
                  </button>
                ) : null}
                {fileAvailable ? (
                  <button className="button button-quiet button-danger" disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onDelete(document.documentId)} type="button">
                    {deleting ? "Deleting…" : "Delete source file"}
                  </button>
                ) : null}
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  ));
}

export function DocumentUnlock({
  onClose,
  onPasswordChange,
  onRetrySources,
  onSourceChange,
  onSubmit,
  state,
}: {
  onClose: () => void;
  onPasswordChange: (password: string) => void;
  onRetrySources: () => void;
  onSourceChange: (moneySourceId: string) => void;
  onSubmit: (updateSavedPassword: boolean) => void;
  state: DocumentUnlockState;
}) {
  const hasSources = state.sources !== null && state.sources.length > 0;
  const dialog = useRef<HTMLElement | null>(null);

  useEffect(() => {
    const containKeyboardFocus = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
        return;
      }
      if (event.key !== "Tab") {
        return;
      }
      const focusable = [...(dialog.current?.querySelectorAll<HTMLElement>(
        "button:not(:disabled), input:not(:disabled), select:not(:disabled)",
      ) ?? [])];
      const first = focusable[0];
      const last = focusable.at(-1);
      if (!first || !last) {
        event.preventDefault();
      } else if (!dialog.current?.contains(document.activeElement)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", containKeyboardFocus);
    return () => window.removeEventListener("keydown", containKeyboardFocus);
  }, [onClose]);

  return (
    <div className="viewer-backdrop">
      <section aria-labelledby="document-unlock-title" aria-modal="true" className="document-unlock" ref={dialog} role="dialog">
        <header className="document-unlock-header">
          <div>
            <p className="ledger-eyebrow">Protected statement</p>
            <h2 id="document-unlock-title">Unlock {state.documentTitle}</h2>
          </div>
          <button autoFocus className="button button-quiet" onClick={onClose} type="button">Close</button>
        </header>
        <div className="document-unlock-body">
          {state.sources === null ? <p className="panel-status" role="status">Loading Money Sources…</p> : null}
          {state.sources?.length === 0 && state.error ? (
            <Feedback
              body={state.error}
              title="Money Sources couldn’t be loaded"
              tone="attention"
            />
          ) : null}
          {state.sources?.length === 0 && state.error ? (
            <button className="button button-primary" onClick={onRetrySources} type="button">Try again</button>
          ) : null}
          {state.sources?.length === 0 && !state.error ? (
            <Feedback
              body="Set up a Money Source before unlocking this protected statement. CanCan needs the source to scope saved passwords safely."
              title="No Money Source is configured"
              tone="attention"
            />
          ) : null}
          {hasSources ? (
            <form onSubmit={(event) => { event.preventDefault(); onSubmit(false); }}>
              <label htmlFor="statement-money-source">Money Source</label>
              <select
                disabled={state.busy}
                id="statement-money-source"
                onChange={(event) => onSourceChange(event.target.value)}
                value={state.selectedMoneySourceId}
              >
                <option value="">Choose a Money Source</option>
                {state.sources?.map((source) => (
                  <option key={source.moneySourceId} value={source.moneySourceId}>
                    {source.displayName}{source.hasSavedPassword ? " — saved password" : ""}
                  </option>
                ))}
              </select>
              <label htmlFor="statement-password">Statement password</label>
              <input
                autoComplete="off"
                disabled={state.busy || !state.selectedMoneySourceId}
                id="statement-password"
                onChange={(event) => onPasswordChange(event.target.value)}
                type="password"
                value={state.password}
              />
              {state.busy ? <p className="panel-status" role="status">Trying the statement password locally…</p> : null}
              {state.savedPasswordStatus === "invalid" ? <p className="unlock-hint">The saved password did not work. Enter the current password below.</p> : null}
              {state.savedPasswordStatus === "unavailable" ? <p className="unlock-hint">The saved password is not available on this Mac. Enter it again below.</p> : null}
              {state.error ? <p className="unlock-error" role="alert">{state.error}</p> : null}
              <div className="document-unlock-actions">
                <button className="button button-quiet" disabled={state.busy || !state.password} type="submit">Use once</button>
                <button className="button button-primary" disabled={state.busy || !state.password} onClick={() => onSubmit(true)} type="button">Update saved password</button>
              </div>
              <p className="unlock-footnote">Use once is forgotten when the Vault locks. Updating saves one verified password for this Money Source in this Mac’s Keychain.</p>
            </form>
          ) : null}
        </div>
      </section>
    </div>
  );
}

export function DocumentViewer({
  onClose,
  onPage,
  viewer,
  viewingPage,
}: {
  onClose: () => void;
  onPage: (pageNumber: number) => void;
  viewer: DocumentViewerState;
  viewingPage: boolean;
}) {
  const dialog = useRef<HTMLElement | null>(null);

  useEffect(() => {
    const containKeyboardFocus = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
        return;
      }
      if (event.key !== "Tab") {
        return;
      }
      const focusable = [...(dialog.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
      const first = focusable[0];
      const last = focusable.at(-1);
      if (!first || !last) {
        event.preventDefault();
      } else if (!dialog.current?.contains(document.activeElement)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", containKeyboardFocus);
    return () => window.removeEventListener("keydown", containKeyboardFocus);
  }, [onClose]);

  return (
    <div className="viewer-backdrop">
      <section aria-labelledby="document-viewer-title" aria-modal="true" className="document-viewer" ref={dialog} role="dialog">
        <header className="document-viewer-header">
          <div>
            <h2 id="document-viewer-title">{viewer.documentTitle}</h2>
          </div>
          <button autoFocus className="button button-quiet" onClick={onClose} type="button">Close</button>
        </header>
        <div className="document-page" aria-busy={viewingPage}>
          <img
            alt={`Page ${viewer.page.pageNumber} of ${viewer.page.pageCount}`}
            src={`data:image/png;base64,${viewer.page.pngBase64}`}
          />
          {viewingPage ? <p className="viewer-loading" role="status">Rendering page…</p> : null}
        </div>
        <footer className="document-viewer-footer">
          <button className="button button-quiet" disabled={viewingPage || viewer.page.pageNumber === 1} onClick={() => onPage(viewer.page.pageNumber - 1)} type="button">Previous</button>
          <p>Page {viewer.page.pageNumber} of {viewer.page.pageCount}</p>
          <button className="button button-quiet" disabled={viewingPage || viewer.page.pageNumber === viewer.page.pageCount} onClick={() => onPage(viewer.page.pageNumber + 1)} type="button">Next</button>
        </footer>
      </section>
    </div>
  );
}

export function DocumentPreview({
  onClose,
  state,
}: {
  onClose: () => void;
  state: DocumentPreviewState;
}) {
  const dialog = useRef<HTMLElement | null>(null);

  useEffect(() => {
    const containKeyboardFocus = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
        return;
      }
      if (event.key !== "Tab") {
        return;
      }
      const focusable = [...(dialog.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
      const first = focusable[0];
      const last = focusable.at(-1);
      if (!first || !last) {
        event.preventDefault();
      } else if (!dialog.current?.contains(document.activeElement)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", containKeyboardFocus);
    return () => window.removeEventListener("keydown", containKeyboardFocus);
  }, [onClose]);

  const { lineCount, previewLines, previewText, truncated } = state.preview;
  const lineUnit = lineCount === 1 ? "line" : "lines";
  return (
    <div className="viewer-backdrop">
      <section aria-labelledby="document-preview-title" aria-modal="true" className="document-viewer" ref={dialog} role="dialog">
        <header className="document-viewer-header">
          <div>
            <p className="ledger-eyebrow">Encrypted evidence</p>
            <h2 id="document-preview-title">{state.documentTitle}</h2>
          </div>
          <button autoFocus className="button button-quiet" onClick={onClose} type="button">Close</button>
        </header>
        <div className="document-page document-preview-page">
          {lineCount === 0 ? (
            <p className="panel-status">This file is empty.</p>
          ) : (
            <pre className="document-preview-text">{previewText}</pre>
          )}
        </div>
        <footer className="document-viewer-footer">
          <p>
            {truncated
              ? `Preview truncated. Displaying content from ${previewLines} of ${lineCount} ${lineUnit}; the final displayed line may be partial. Save a copy to view the full file.`
              : `${lineCount} ${lineUnit}`}
          </p>
        </footer>
      </section>
    </div>
  );
}

export function VaultGate({
  body,
  busy,
  onPasswordChange,
  onSubmit,
  onUnlockWithKeychain,
  password,
  rememberedOnThisMac = false,
  title,
}: {
  body: string;
  busy: boolean;
  onPasswordChange?: (password: string) => void;
  onSubmit?: () => void;
  onUnlockWithKeychain?: () => void;
  password?: string;
  rememberedOnThisMac?: boolean | null;
  title: string;
}) {
  const acceptsPassword = onPasswordChange !== undefined && onSubmit !== undefined;
  return (
    <section className="vault-gate" aria-labelledby="vault-gate-title">
      <h2 id="vault-gate-title">{title}</h2>
      <p>{body}</p>
      {rememberedOnThisMac === null ? (
        <p className="vault-keychain-status">Keychain unlock is unavailable. Use your Vault password.</p>
      ) : rememberedOnThisMac && onUnlockWithKeychain ? (
        <button className="button button-quiet vault-keychain-unlock" disabled={busy} onClick={onUnlockWithKeychain} type="button">
          Unlock with this Mac
        </button>
      ) : null}
      {acceptsPassword ? (
        <form className="vault-password-form" onSubmit={(event) => { event.preventDefault(); onSubmit(); }}>
          <label htmlFor="vault-password">Vault password</label>
          <div className="vault-password-controls">
            <input autoComplete="off" disabled={busy} id="vault-password" onChange={(event) => onPasswordChange(event.target.value)} type="password" value={password} />
            <button className="button button-primary" disabled={busy} type="submit">{busy ? "Working…" : title === "Create your Vault" ? "Create Vault" : "Unlock Vault"}</button>
          </div>
        </form>
      ) : <p className="panel-status" role="status">Checking Vault status…</p>}
    </section>
  );
}

export function importNotice(status: SourceDocumentImportOutcome["status"] | "cancelled"): Notice {
  const notices: Record<SourceDocumentImportOutcome["status"] | "cancelled", Notice> = {
    imported: { tone: "success", title: "Added to your Vault", body: "Your file is safely stored. Check its routing when you’re ready." },
    already_present: { tone: "success", title: "Already in CanCan", body: "This exact file is already safely stored in your Vault." },
    restored: { tone: "success", title: "Evidence restored", body: "Your saved evidence is available in the Vault again." },
    cancelled: { tone: "attention", title: "No file was imported", body: "You can add a PDF, CSV, PNG, or JPEG whenever you’re ready." },
  };
  return notices[status];
}

export function routingNotice(
  outcome: SourceDocumentRoutingOutcome,
  source?: MoneySourceSummary,
): Notice {
  return outcome.status === "routed"
    ? {
        tone: "success",
        title: "Evidence routed",
        body: source
          ? `CanCan matched this evidence to ${source.displayName}.`
          : "CanCan matched this evidence to one configured source and account.",
      }
    : { tone: "attention", title: "Needs attention", body: "CanCan could not match this evidence uniquely, so it was not assigned." };
}

function sourceTypeLabel(sourceType: string) {
  return sourceType.replaceAll("_", " ").replace(/^./, (letter) => letter.toUpperCase());
}

function fileStateLabel(fileState: SourceDocumentSummary["fileState"]) {
  return fileState === "available" ? "Ready" : fileState === "deleted" ? "File deleted" : "Missing";
}

function documentStatusLabel(document: SourceDocumentSummary) {
  switch (document.documentStatus) {
    case "password_required":
      return "Needs attention";
    case "protected_unlocked":
      return "Ready";
    case "inspection_failed":
      return "Needs attention";
    case "unavailable":
      return "Missing";
    default:
      return fileStateLabel(document.fileState);
  }
}

const META_MONTHS = [
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
] as const;

const GROUP_MONTHS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
] as const;

const SQLITE_UTC_TIMESTAMP = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?$/;

export function parseReceivedAt(receivedAt: string) {
  const timestamp = SQLITE_UTC_TIMESTAMP.test(receivedAt)
    ? `${receivedAt.replace(" ", "T")}Z`
    : receivedAt;
  return new Date(timestamp);
}

function evidenceMeta(document: SourceDocumentSummary) {
  return `Added ${formatMetaDate(document.receivedAt)} · ${formatByteSize(document.byteSize)}`;
}

function formatMetaDate(iso: string) {
  const date = parseReceivedAt(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return `${date.getUTCDate()} ${META_MONTHS[date.getUTCMonth()]} ${date.getUTCFullYear()}`;
}

function formatByteSize(bytes: number) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export interface EvidenceMonthGroup {
  documents: SourceDocumentSummary[];
  key: string;
  label: string;
}

export function groupEvidenceByMonth(
  documents: SourceDocumentSummary[],
): EvidenceMonthGroup[] {
  const groups = new Map<string, EvidenceMonthGroup>();
  for (const document of documents) {
    const date = parseReceivedAt(document.receivedAt);
    const valid = !Number.isNaN(date.getTime());
    const key = valid
      ? `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}`
      : "unknown";
    const existing = groups.get(key);
    if (existing) {
      existing.documents.push(document);
    } else {
      groups.set(key, {
        documents: [document],
        key,
        label: valid
          ? `${GROUP_MONTHS[date.getUTCMonth()]} ${date.getUTCFullYear()}`
          : "Unknown date",
      });
    }
  }
  return [...groups.values()].sort((a, b) => {
    if (a.key === "unknown") {
      return 1;
    }
    if (b.key === "unknown") {
      return -1;
    }
    return b.key.localeCompare(a.key);
  });
}
