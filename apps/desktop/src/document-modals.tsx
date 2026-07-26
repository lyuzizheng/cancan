import { useEffect, useRef } from "react";

import type {
  RenderedDocumentPage,
  SourceDocumentPreview,
  StatementPasswordSourceSummary,
} from "./command-contracts";
import { Feedback } from "./feedback";

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
