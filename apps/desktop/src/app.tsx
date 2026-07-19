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
  error: string | null;
  importing: boolean;
  loadingDocuments: boolean;
  normalizingDocumentId: string | null;
  notice: Notice | null;
  onCloseViewer: () => void;
  onImport: () => void;
  onLock: () => void;
  onNormalize: (documentId: string) => void;
  onPasswordChange: (password: string) => void;
  onRefresh: () => void;
  onSubmitPassword: () => void;
  onView: (
    document: SourceDocumentSummary,
    trigger: HTMLButtonElement,
  ) => void;
  onViewerPage: (pageNumber: number) => void;
  password: string;
  unassignedDocuments: SourceDocumentSummary[];
  vaultStatus: VaultScreenStatus;
  viewer: DocumentViewerState | null;
  viewingPage: boolean;
}

const defaultVaultApi = createVaultApi();
const VAULT_INACTIVITY_TIMEOUT_MS = 15 * 60 * 1000;

export function App({ api = defaultVaultApi }: { api?: VaultApi }) {
  const [vaultStatus, setVaultStatus] = useState<VaultScreenStatus>("loading");
  const [busy, setBusy] = useState(true);
  const [password, setPassword] = useState("");
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
  const viewerRequestId = useRef(0);
  const viewerReturnFocus = useRef<HTMLButtonElement | null>(null);
  const vaultSessionId = useRef(0);

  const clearViewer = useCallback((restoreFocus = true) => {
    viewerRequestId.current += 1;
    setViewer(null);
    setViewingPage(false);
    if (!restoreFocus) {
      viewerReturnFocus.current = null;
    }
  }, []);

  const showVaultGate = useCallback((nextStatus: VaultStatus) => {
    vaultSessionId.current += 1;
    clearViewer(false);
    setVaultStatus(nextStatus);
    setError(null);
    setNotice(null);
    setUnassignedDocuments([]);
  }, [clearViewer]);

  useEffect(() => {
    if (viewer === null && viewerReturnFocus.current) {
      viewerReturnFocus.current.focus();
      viewerReturnFocus.current = null;
    }
  }, [viewer]);

  const loadUnassignedDocuments = useCallback(async () => {
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

  useEffect(() => {
    if (vaultStatus !== "unlocked") {
      return;
    }

    const sessionId = vaultSessionId.current;
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
    for (const event of activityEvents) {
      window.addEventListener(event, resetTimeout);
    }

    return () => {
      window.clearTimeout(timeout);
      for (const event of activityEvents) {
        window.removeEventListener(event, resetTimeout);
      }
    };

    function lockAfterInactivity() {
      void api
        .lockVault()
        .then((nextStatus) => {
          if (vaultSessionId.current === sessionId) {
            showVaultGate(nextStatus);
          }
        })
        .catch((nextError: unknown) => {
          if (vaultSessionId.current === sessionId) {
            setError(commandErrorMessage(nextError));
            resetTimeout();
          }
        });
    }
  }, [api, showVaultGate, vaultStatus]);

  const refreshVaultStatus = useCallback(async () => {
    setBusy(true);
    setVaultStatus("loading");
    setError(null);
    try {
      const nextStatus = await api.vaultStatus();
      setVaultStatus(nextStatus);
      if (nextStatus === "unlocked") {
        await loadUnassignedDocuments();
      } else {
        setUnassignedDocuments([]);
        clearViewer(false);
      }
    } catch (nextError) {
      setError(commandErrorMessage(nextError));
    } finally {
      setBusy(false);
    }
  }, [clearViewer, loadUnassignedDocuments]);

  useEffect(() => {
    void refreshVaultStatus();
  }, [refreshVaultStatus]);

  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (nextError) {
      setError(commandErrorMessage(nextError));
    } finally {
      setBusy(false);
    }
  };

  const submitPassword = () => {
    if (!password) {
      setError("Enter a password to continue.");
      return;
    }

    void run(async () => {
      const nextStatus =
        vaultStatus === "not_created"
          ? await api.createVault(password)
          : await api.unlockVault(password);
      setPassword("");
      setVaultStatus(nextStatus);
      if (nextStatus === "unlocked") {
        await loadUnassignedDocuments();
      }
    });
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
      error={error}
      importing={importing}
      loadingDocuments={loadingDocuments}
      normalizingDocumentId={normalizingDocumentId}
      notice={notice}
      onCloseViewer={clearViewer}
      onImport={importDocument}
      onLock={() =>
        void run(async () => {
          showVaultGate(await api.lockVault());
        })
      }
      onNormalize={normalizeDocument}
      onPasswordChange={setPassword}
      onRefresh={() => void refreshVaultStatus()}
      onSubmitPassword={submitPassword}
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
      unassignedDocuments={unassignedDocuments}
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
            title={props.vaultStatus === "not_created" ? "Create your Vault" : "Unlock your Vault"}
            onPasswordChange={props.onPasswordChange}
            onSubmit={props.onSubmitPassword}
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
  password,
  title,
}: {
  body: string;
  busy: boolean;
  onPasswordChange?: (password: string) => void;
  onSubmit?: () => void;
  password?: string;
  title: string;
}) {
  const acceptsPassword = onPasswordChange !== undefined && onSubmit !== undefined;
  return (
    <section className="vault-gate" aria-labelledby="vault-gate-title">
      <p className="ledger-eyebrow">Vault access</p>
      <h2 id="vault-gate-title">{title}</h2>
      <p>{body}</p>
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
