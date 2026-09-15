import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { SourceDocumentSummary } from "./command-contracts";
import type {
  DocumentPreviewState,
  DocumentUnlockState,
  DocumentViewerState,
} from "./document-modals";
import { createStatementUnlockActions } from "./statement-unlock-actions";
import { createViewerActions } from "./viewer-actions";
import type { VaultApi } from "./vault-api";
import type { VaultSession } from "./use-vault-session";

export interface EvidenceOverlays {
  clearPreview(): void;
  clearViewer(restoreFocus?: boolean): void;
  closeUnlock(): void;
  loadDocumentPreview(documentId: string, documentTitle: string): void;
  loadUnlockSources(documentId: string, documentTitle: string): void;
  loadViewerPage(documentId: string, documentTitle: string, pageNumber: number): void;
  openDocumentUnlock(document: SourceDocumentSummary): void;
  readonly preview: DocumentPreviewState | null;
  /** Drops every session-scoped overlay; the gate's reset half. */
  reset(): void;
  retryUnlockSources(): void;
  selectUnlockSource(moneySourceId: string): void;
  setUnlockPassword(password: string): void;
  submitDocumentPassword(updateSavedPassword: boolean): void;
  readonly unlockingDocument: DocumentUnlockState | null;
  readonly viewer: DocumentViewerState | null;
  readonly viewerReturnFocus: { current: HTMLButtonElement | null };
  readonly viewingPage: boolean;
}

/**
 * Owns the evidence overlays — the document viewer, the CSV/PDF preview, and
 * the protected-statement unlock dialog — plus the focus return to the button
 * that opened them. PDF pages and previews load through the host with
 * request-id guards so a stale response never overwrites newer state.
 */
export function useEvidenceOverlays({
  api,
  loadDocuments,
  session,
}: {
  api: VaultApi;
  loadDocuments: () => Promise<void>;
  session: VaultSession;
}): EvidenceOverlays {
  const [viewer, setViewer] = useState<DocumentViewerState | null>(null);
  const [preview, setPreview] = useState<DocumentPreviewState | null>(null);
  const [unlockingDocument, setUnlockingDocument] = useState<DocumentUnlockState | null>(
    null,
  );
  const [viewingPage, setViewingPage] = useState(false);
  const viewerRequestId = useRef(0);
  const previewRequestId = useRef(0);
  const unlockRequestId = useRef(0);
  const viewerReturnFocus = useRef<HTMLButtonElement | null>(null);

  const { run, setNotice } = session;

  useEffect(() => () => {
    unlockRequestId.current += 1;
    previewRequestId.current += 1;
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

  const closeUnlock = useCallback(() => {
    unlockRequestId.current += 1;
    setUnlockingDocument(null);
  }, []);

  const setUnlockPassword = useCallback((password: string) => {
    setUnlockingDocument((current) => current
      ? { ...current, error: null, password }
      : current);
  }, []);

  const reset = useCallback(() => {
    unlockRequestId.current += 1;
    previewRequestId.current += 1;
    clearViewer(false);
    clearPreview();
    setUnlockingDocument(null);
  }, [clearPreview, clearViewer]);

  useEffect(() => {
    if (viewer === null && preview === null && viewerReturnFocus.current) {
      viewerReturnFocus.current.focus();
      viewerReturnFocus.current = null;
    }
  }, [viewer, preview]);

  const viewerActions = useMemo(() => createViewerActions({
    api,
    clearViewer,
    previewRequestId,
    run,
    setPreview,
    setViewer,
    setViewingPage,
    viewer,
    viewerRequestId,
  }), [api, clearViewer, run, viewer]);

  const unlockActions = useMemo(() => createStatementUnlockActions({
    api,
    loadDocuments,
    setNotice,
    setUnlockingDocument,
    unlockingDocument,
    unlockRequestId,
  }), [api, loadDocuments, setNotice, unlockingDocument]);

  const retryUnlockSources = useCallback(() => {
    if (unlockingDocument) {
      unlockActions.loadUnlockSources(
        unlockingDocument.documentId,
        unlockingDocument.documentTitle,
      );
    }
  }, [unlockActions, unlockingDocument]);

  return {
    clearPreview,
    clearViewer,
    closeUnlock,
    loadDocumentPreview: viewerActions.loadDocumentPreview,
    loadUnlockSources: unlockActions.loadUnlockSources,
    loadViewerPage: viewerActions.loadViewerPage,
    openDocumentUnlock: unlockActions.openDocumentUnlock,
    preview,
    reset,
    retryUnlockSources,
    selectUnlockSource: unlockActions.selectUnlockSource,
    setUnlockPassword,
    submitDocumentPassword: unlockActions.submitDocumentPassword,
    unlockingDocument,
    viewer,
    viewerReturnFocus,
    viewingPage,
  };
}
