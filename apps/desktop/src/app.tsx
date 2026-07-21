import { AppShell } from "@cancan/ui";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  RenderedDocumentPage,
  SourceDocumentImportOutcome,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
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

export interface VaultManualImportViewProps {
  busy: boolean;
  deletingDocumentId: string | null;
  error: string | null;
  importing: boolean;
  loadingDocuments: boolean;
  normalizingDocumentId: string | null;
  notice: Notice | null;
  onCloseViewer: () => void;
  onDelete: (documentId: string) => void;
  onImport: () => void;
  onLock: () => void;
  onNormalize: (documentId: string) => void;
  onPasswordChange: (password: string) => void;
  onRefresh: () => void;
  onRememberedChange: (remembered: boolean) => void;
  onSubmitPassword: () => void;
  onUnlockWithKeychain: () => void;
  onView: (
    document: SourceDocumentSummary,
    trigger: HTMLButtonElement,
  ) => void;
  onViewerPage: (pageNumber: number) => void;
  password: string;
  rememberedOnThisMac: boolean | null;
  unassignedDocuments: SourceDocumentSummary[];
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
  const [viewingPage, setViewingPage] = useState(false);
  const [updatingRemembered, setUpdatingRemembered] = useState(false);
  const viewerRequestId = useRef(0);
  const viewerReturnFocus = useRef<HTMLButtonElement | null>(null);
  const documentLoadsAllowed = useRef(false);
  const vaultSessionId = useRef(0);

  useEffect(() => () => {
    vaultSessionId.current += 1;
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
    setVaultStatus(nextStatus);
    setError(null);
    setNotice(null);
    setUnassignedDocuments([]);
    setLoadingDocuments(false);
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
      setVaultStatus(nextStatus);
      if (nextStatus === "unlocked") {
        await loadUnassignedDocuments();
      } else {
        setUnassignedDocuments([]);
        clearViewer(false);
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
      onDelete={deleteDocument}
      onImport={importDocument}
      onLock={() => void requestVaultLock()}
      onNormalize={normalizeDocument}
      onPasswordChange={setPassword}
      onRefresh={() => void refreshVaultStatus()}
      onRememberedChange={updateRemembered}
      onSubmitPassword={submitPassword}
      onUnlockWithKeychain={unlockWithKeychain}
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
      unassignedDocuments={unassignedDocuments}
      updatingRemembered={updatingRemembered}
      vaultStatus={vaultStatus}
      viewer={viewer}
      viewingPage={viewingPage}
    />
  );
}

export function VaultManualImportView(props: VaultManualImportViewProps) {
  const unlocked = props.vaultStatus === "unlocked";

  return (
    <AppShell>
      <aside aria-hidden={props.viewer ? true : undefined} className="vault-spine" inert={props.viewer !== null} aria-label="CanCan Vault">
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

      <section aria-hidden={props.viewer ? true : undefined} className="ledger" inert={props.viewer !== null} aria-busy={props.vaultStatus === "loading"}>
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
                    const routingAvailable = document.fileState === "available";
                    const deleting = props.deletingDocumentId === document.documentId;
                    const normalizing = props.normalizingDocumentId === document.documentId;
                    return (
                      <li className="evidence-row" key={document.documentId}>
                        <span className="document-kind" aria-hidden="true">{document.mimeType === "application/pdf" ? "PDF" : "CSV"}</span>
                        <div className="evidence-details">
                          <p>{document.originalFilename}</p>
                          <span>{fileStateLabel(document.fileState)}</span>
                        </div>
                        <div className="evidence-actions">
                          {document.mimeType === "application/pdf" ? (
                            <button className="button button-quiet" disabled={!routingAvailable || props.busy || props.normalizingDocumentId !== null} onClick={(event) => props.onView(document, event.currentTarget)} type="button">
                              {!routingAvailable ? "View unavailable" : "View document"}
                            </button>
                          ) : null}
                          <button className="button button-quiet" disabled={!routingAvailable || props.busy || props.normalizingDocumentId !== null} onClick={() => props.onNormalize(document.documentId)} type="button">
                            {!routingAvailable ? "Routing unavailable" : normalizing ? "Checking…" : "Check routing"}
                          </button>
                          {routingAvailable ? (
                            <button className="button button-quiet" disabled={props.busy || props.normalizingDocumentId !== null} onClick={() => props.onDelete(document.documentId)} type="button">
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
    </AppShell>
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
  return fileState === "available" ? "Ready for routing" : fileState === "deleted" ? "File deleted" : "File missing";
}
