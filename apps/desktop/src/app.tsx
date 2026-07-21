import { AppShell } from "@cancan/ui";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  RenderedDocumentPage,
  SourceDocumentImportOutcome,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
  StatementPasswordSourceSummary,
  VaultStatus,
} from "./command-contracts";
import {
  commandErrorMessage,
  createVaultApi,
  type VaultApi,
} from "./vault-api";

type VaultScreenStatus = VaultStatus | "loading";

export interface Notice {
  body: string;
  tone: "success" | "attention";
  title: string;
}

export interface DocumentViewerState {
  documentId: string;
  documentTitle: string;
  page: RenderedDocumentPage;
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

export interface VaultManualImportViewProps {
  busy: boolean;
  deletingDocumentId: string | null;
  error: string | null;
  importing: boolean;
  loadingDocuments: boolean;
  normalizingDocumentId: string | null;
  notice: Notice | null;
  onCloseViewer: () => void;
  onCloseUnlock: () => void;
  onDelete: (documentId: string) => void;
  onImport: () => void;
  onLock: () => void;
  onNormalize: (documentId: string) => void;
  onOpenUnlock: (document: SourceDocumentSummary) => void;
  onRetryUnlockSources: () => void;
  onPasswordChange: (password: string) => void;
  onRefresh: () => void;
  onRememberedChange: (remembered: boolean) => void;
  onSaveRecoveryFile: () => void;
  onSaveSourceCopy: (documentId: string) => void;
  onSubmitPassword: () => void;
  onUnlockWithKeychain: () => void;
  onUnlockPasswordChange: (password: string) => void;
  onUnlockSourceChange: (moneySourceId: string) => void;
  onUnlockSubmit: (updateSavedPassword: boolean) => void;
  onView: (
    document: SourceDocumentSummary,
    trigger: HTMLButtonElement,
  ) => void;
  onViewerPage: (pageNumber: number) => void;
  password: string;
  rememberedOnThisMac: boolean | null;
  recoveryConfigured: boolean;
  savingRecoveryFile: boolean;
  savingCopyDocumentId: string | null;
  unassignedDocuments: SourceDocumentSummary[];
  unlockingDocument: DocumentUnlockState | null;
  updatingRemembered: boolean;
  vaultStatus: VaultScreenStatus;
  viewer: DocumentViewerState | null;
  viewingPage: boolean;
}

const defaultVaultApi = createVaultApi();
const VAULT_INACTIVITY_TIMEOUT_MS = 15 * 60 * 1000;

export function App({ api = defaultVaultApi }: { api?: VaultApi }) {
  const [vaultStatus, setVaultStatus] = useState<VaultScreenStatus>("loading");
  const [busy, setBusy] = useState(true);
  const [deletingDocumentId, setDeletingDocumentId] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [rememberedOnThisMac, setRememberedOnThisMac] = useState<boolean | null>(
    false,
  );
  const [recoveryConfigured, setRecoveryConfigured] = useState(false);
  const [unassignedDocuments, setUnassignedDocuments] = useState<
    SourceDocumentSummary[]
  >([]);
  const [loadingDocuments, setLoadingDocuments] = useState(false);
  const [importing, setImporting] = useState(false);
  const [normalizingDocumentId, setNormalizingDocumentId] = useState<string | null>(
    null,
  );
  const [notice, setNotice] = useState<Notice | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [viewer, setViewer] = useState<DocumentViewerState | null>(null);
  const [unlockingDocument, setUnlockingDocument] = useState<DocumentUnlockState | null>(null);
  const [viewingPage, setViewingPage] = useState(false);
  const [updatingRemembered, setUpdatingRemembered] = useState(false);
  const [savingRecoveryFile, setSavingRecoveryFile] = useState(false);
  const [savingCopyDocumentId, setSavingCopyDocumentId] = useState<string | null>(null);
  const viewerRequestId = useRef(0);
  const unlockRequestId = useRef(0);
  const viewerReturnFocus = useRef<HTMLButtonElement | null>(null);
  const documentLoadsAllowed = useRef(false);
  const vaultSessionId = useRef(0);

  useEffect(() => () => {
    vaultSessionId.current += 1;
    unlockRequestId.current += 1;
  }, []);

  const clearViewer = useCallback((restoreFocus = true) => {
    viewerRequestId.current += 1;
    setViewer(null);
    setViewingPage(false);
    if (!restoreFocus) {
      viewerReturnFocus.current = null;
    }
  }, []);

  const showVaultGate = useCallback((nextStatus: VaultScreenStatus) => {
    const nextSessionId = vaultSessionId.current + 1;
    vaultSessionId.current = nextSessionId;
    documentLoadsAllowed.current = false;
    clearViewer(false);
    unlockRequestId.current += 1;
    setUnlockingDocument(null);
    setVaultStatus(nextStatus);
    setError(null);
    setNotice(null);
    setPassword("");
    setUnassignedDocuments([]);
    setLoadingDocuments(false);
    setSavingCopyDocumentId(null);
    setBusy(false);
    return nextSessionId;
  }, [clearViewer]);

  useEffect(() => {
    if (viewer === null && viewerReturnFocus.current) {
      viewerReturnFocus.current.focus();
      viewerReturnFocus.current = null;
    }
  }, [viewer]);

  const loadUnassignedDocuments = useCallback(async () => {
    if (!documentLoadsAllowed.current) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setLoadingDocuments(true);
    try {
      const documents = await api.listUnassignedSourceDocuments();
      if (vaultSessionId.current === sessionId) {
        setUnassignedDocuments(documents);
      }
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setLoadingDocuments(false);
      }
    }
  }, [api]);

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
          await loadUnassignedDocuments();
          return vaultSessionId.current === sessionId;
        }
      } catch {
        if (vaultSessionId.current === sessionId) {
          setError(commandErrorMessage(nextError));
        }
      }
    }
    return false;
  }, [api, loadUnassignedDocuments, showVaultGate]);

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
        await loadUnassignedDocuments();
      } else {
        setUnassignedDocuments([]);
        clearViewer(false);
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
  }, [clearViewer, loadUnassignedDocuments]);

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
        await loadUnassignedDocuments();
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
          await loadUnassignedDocuments();
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
        await loadUnassignedDocuments();
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
      setNotice(routingNotice(await api.normalizeSourceDocument(documentId)));
      await loadUnassignedDocuments();
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
        await loadUnassignedDocuments();
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
          body: "The saved password worked. This statement is ready to view for this Vault session. Routing remains unavailable for protected statements.",
          tone: "success",
          title: "Statement unlocked",
        });
        await loadUnassignedDocuments();
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
          ? "The verified password replaced this Money Source’s saved password. This statement is ready to view; routing remains unavailable for protected statements."
          : "This statement is ready to view until the Vault locks. Routing remains unavailable for protected statements.",
        tone: "success",
        title: "Statement unlocked",
      });
      await loadUnassignedDocuments();
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

  return (
    <VaultManualImportView
      busy={busy}
      deletingDocumentId={deletingDocumentId}
      error={error}
      importing={importing}
      loadingDocuments={loadingDocuments}
      normalizingDocumentId={normalizingDocumentId}
      notice={notice}
      onCloseViewer={clearViewer}
      onCloseUnlock={() => {
        unlockRequestId.current += 1;
        setUnlockingDocument(null);
      }}
      onDelete={deleteDocument}
      onImport={importDocument}
      onLock={() => void requestVaultLock()}
      onNormalize={normalizeDocument}
      onOpenUnlock={openDocumentUnlock}
      onRetryUnlockSources={() => {
        if (unlockingDocument) {
          loadUnlockSources(unlockingDocument.documentId, unlockingDocument.documentTitle);
        }
      }}
      onPasswordChange={setPassword}
      onRefresh={() => void refreshVaultStatus()}
      onRememberedChange={updateRemembered}
      onSaveRecoveryFile={saveRecoveryFile}
      onSaveSourceCopy={saveSourceCopy}
      onSubmitPassword={submitPassword}
      onUnlockWithKeychain={unlockWithKeychain}
      onUnlockPasswordChange={(nextPassword) => setUnlockingDocument((current) => current
        ? { ...current, error: null, password: nextPassword }
        : current)}
      onUnlockSourceChange={selectUnlockSource}
      onUnlockSubmit={submitDocumentPassword}
      onView={(document, trigger) => {
        viewerReturnFocus.current = trigger;
        loadViewerPage(document.documentId, document.originalFilename, 1);
      }}
      onViewerPage={(pageNumber) => {
        if (viewer) {
          loadViewerPage(viewer.documentId, viewer.documentTitle, pageNumber);
        }
      }}
      password={password}
      rememberedOnThisMac={rememberedOnThisMac}
      recoveryConfigured={recoveryConfigured}
      savingRecoveryFile={savingRecoveryFile}
      savingCopyDocumentId={savingCopyDocumentId}
      unassignedDocuments={unassignedDocuments}
      unlockingDocument={unlockingDocument}
      updatingRemembered={updatingRemembered}
      vaultStatus={vaultStatus}
      viewer={viewer}
      viewingPage={viewingPage}
    />
  );
}

export function VaultManualImportView(props: VaultManualImportViewProps) {
  const unlocked = props.vaultStatus === "unlocked";
  const modalOpen = props.viewer !== null || props.unlockingDocument !== null;

  return (
    <AppShell>
      <aside aria-hidden={modalOpen ? true : undefined} className="vault-spine" inert={modalOpen} aria-label="CanCan Vault">
        <div className="vault-brand">
          <span className="vault-mark" aria-hidden="true">C</span>
          <span>CanCan</span>
        </div>
        <div className="vault-spine-status">
          <p className="vault-spine-label">Local Vault</p>
          <p className="vault-spine-state">
            <span className={`vault-status-light vault-status-${props.vaultStatus}`} aria-hidden="true" />
            {vaultStatusLabel(props.vaultStatus)}
          </p>
          <p className="vault-spine-copy">Evidence stays encrypted on this Mac.</p>
        </div>
        <p className="vault-spine-footnote">Manual import</p>
      </aside>

      <section aria-hidden={modalOpen ? true : undefined} className="ledger" inert={modalOpen} aria-busy={props.vaultStatus === "loading"}>
        <header className="ledger-header">
          <div>
            <p className="ledger-eyebrow">Sources / Evidence</p>
            <h1>Secure file intake</h1>
          </div>
          {unlocked ? (
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
          ) : null}
        </header>

        {props.error ? (
          <Feedback tone="error" title="Something needs your attention" body={props.error} action={props.onRefresh} />
        ) : null}

        {props.vaultStatus === "loading" ? (
          <VaultGate busy title="Checking your Vault" body="Confirming the local Vault state before showing evidence." />
        ) : null}

        {props.vaultStatus === "not_created" || props.vaultStatus === "locked" ? (
          <VaultGate
            busy={props.busy}
            body={props.vaultStatus === "not_created" ? "Create a local Vault before adding your first statement or export." : "Unlock your local Vault to add a file or check its routing."}
            password={props.password}
            rememberedOnThisMac={props.rememberedOnThisMac}
            title={props.vaultStatus === "not_created" ? "Create your Vault" : "Unlock your Vault"}
            onPasswordChange={props.onPasswordChange}
            onSubmit={props.onSubmitPassword}
            onUnlockWithKeychain={props.vaultStatus === "locked" ? props.onUnlockWithKeychain : undefined}
          />
        ) : null}

        {unlocked ? (
          <section className="intake-content" aria-label="Manual import">
            {!props.recoveryConfigured ? (
              <section className="todo-panel" aria-labelledby="todo-heading">
                <div className="todo-panel-heading">
                  <div>
                    <p className="ledger-eyebrow">Vault setup</p>
                    <h2 id="todo-heading">To do</h2>
                  </div>
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
              <p className="ledger-eyebrow">Encrypted capture</p>
              <h2>Add a statement or export</h2>
              <p>Choose a PDF or CSV. CanCan saves it in your Vault before checking its configured source.</p>
            </section>

            {props.notice ? <Feedback {...props.notice} /> : null}

            <section className="attention-panel" aria-labelledby="attention-heading">
              <div className="attention-panel-heading">
                <div>
                  <p className="ledger-eyebrow">Source routing</p>
                  <h2 id="attention-heading">Needs attention</h2>
                </div>
                <span className="attention-count" aria-label={`${props.unassignedDocuments.length} documents`}>
                  {props.unassignedDocuments.length}
                </span>
              </div>

              {props.loadingDocuments ? <p className="panel-status" role="status">Refreshing evidence…</p> : null}
              {!props.loadingDocuments && props.unassignedDocuments.length === 0 ? (
                <p className="panel-status">No evidence needs your attention.</p>
              ) : null}
              {props.unassignedDocuments.length > 0 ? (
                <ul className="evidence-list">
                  {props.unassignedDocuments.map((document) => {
                    const passwordRequired = document.documentStatus === "password_required";
                    const protectedUnlocked = document.documentStatus === "protected_unlocked";
                    const fileAvailable = document.fileState === "available";
                    const viewingAvailable = fileAvailable
                      && (document.documentStatus === "ready" || protectedUnlocked);
                    const routingAvailable = fileAvailable && document.documentStatus === "ready";
                    const deleting = props.deletingDocumentId === document.documentId;
                    const normalizing = props.normalizingDocumentId === document.documentId;
                    const savingCopy = props.savingCopyDocumentId === document.documentId;
                    return (
                      <li className="evidence-row" key={document.documentId}>
                        <span className="document-kind" aria-hidden="true">{document.mimeType === "application/pdf" ? "PDF" : "CSV"}</span>
                        <div className="evidence-details">
                          <p>{document.originalFilename}</p>
                          <span>{documentStatusLabel(document)}</span>
                        </div>
                        <div className="evidence-actions">
                          {passwordRequired ? (
                            <button className="button button-primary" disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onOpenUnlock(document)} type="button">
                              Unlock
                            </button>
                          ) : document.mimeType === "application/pdf" ? (
                            <button className="button button-quiet" disabled={!viewingAvailable || props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={(event) => props.onView(document, event.currentTarget)} type="button">
                              {!viewingAvailable ? "View unavailable" : "View document"}
                            </button>
                          ) : null}
                          {!passwordRequired && document.documentStatus !== "inspection_failed" ? (
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
                            <button className="button button-quiet" disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onDelete(document.documentId)} type="button">
                              {deleting ? "Deleting…" : "Delete source file"}
                            </button>
                          ) : null}
                        </div>
                      </li>
                    );
                  })}
                </ul>
              ) : null}
            </section>
          </section>
        ) : null}
      </section>
      {unlocked && props.viewer ? (
        <DocumentViewer
          onClose={props.onCloseViewer}
          onPage={props.onViewerPage}
          viewer={props.viewer}
          viewingPage={props.viewingPage}
        />
      ) : null}
      {unlocked && props.unlockingDocument ? (
        <DocumentUnlock
          onClose={props.onCloseUnlock}
          onPasswordChange={props.onUnlockPasswordChange}
          onRetrySources={props.onRetryUnlockSources}
          onSourceChange={props.onUnlockSourceChange}
          onSubmit={props.onUnlockSubmit}
          state={props.unlockingDocument}
        />
      ) : null}
    </AppShell>
  );
}

function DocumentUnlock({
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

function DocumentViewer({
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
            <p className="ledger-eyebrow">Encrypted evidence</p>
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

function VaultGate({
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
      <p className="ledger-eyebrow">Vault access</p>
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

function Feedback({ action, body, title, tone }: Omit<Notice, "tone"> & { action?: () => void; tone: Notice["tone"] | "error" }) {
  return (
    <section className={`feedback feedback-${tone}`} aria-live="polite">
      <div><p className="feedback-title">{title}</p><p>{body}</p></div>
      {action ? <button className="button button-quiet" onClick={action} type="button">Try again</button> : null}
    </section>
  );
}

export function importNotice(status: SourceDocumentImportOutcome["status"] | "cancelled"): Notice {
  const notices: Record<SourceDocumentImportOutcome["status"] | "cancelled", Notice> = {
    imported: { tone: "success", title: "Added to your Vault", body: "Your file is safely stored. Check its routing when you’re ready." },
    already_present: { tone: "success", title: "Already in CanCan", body: "This exact file is already safely stored in your Vault." },
    restored: { tone: "success", title: "Evidence restored", body: "Your saved evidence is available in the Vault again." },
    cancelled: { tone: "attention", title: "No file was imported", body: "You can add a PDF or CSV whenever you’re ready." },
  };
  return notices[status];
}

export function routingNotice(outcome: SourceDocumentRoutingOutcome): Notice {
  return outcome.status === "routed"
    ? { tone: "success", title: "Evidence routed", body: "CanCan matched this evidence to one configured source and account." }
    : { tone: "attention", title: "Needs attention", body: "CanCan could not match this evidence uniquely, so it was not assigned." };
}

function vaultStatusLabel(status: VaultScreenStatus) {
  return status === "unlocked" ? "Vault unlocked" : status === "locked" ? "Vault locked" : status === "not_created" ? "Vault setup needed" : "Checking Vault";
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
