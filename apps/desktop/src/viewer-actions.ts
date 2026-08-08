import type { DocumentPreviewState, DocumentViewerState } from "./document-modals";
import type { VaultApi } from "./vault-api";

export interface ViewerActions {
  loadDocumentPreview(documentId: string, documentTitle: string): void;
  loadViewerPage(documentId: string, documentTitle: string, pageNumber: number): void;
}

export interface ViewerActionDeps {
  api: VaultApi;
  clearViewer: () => void;
  previewRequestId: { current: number };
  run: (action: () => Promise<void>) => Promise<void>;
  setPreview: (state: DocumentPreviewState) => void;
  setViewer: (state: DocumentViewerState) => void;
  setViewingPage: (viewing: boolean) => void;
  viewer: DocumentViewerState | null;
  viewerRequestId: { current: number };
}

/**
 * Evidence viewing orchestration — PDF pages and CSV previews load through
 * the host with request-id guards so stale renders never overwrite a newer
 * viewer state.
 */
export function createViewerActions(deps: ViewerActionDeps): ViewerActions {
  const {
    api,
    clearViewer,
    previewRequestId,
    run,
    setPreview,
    setViewer,
    setViewingPage,
    viewer,
    viewerRequestId,
  } = deps;

  const loadViewerPage: ViewerActions["loadViewerPage"] = (
    documentId,
    documentTitle,
    pageNumber,
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

  const loadDocumentPreview: ViewerActions["loadDocumentPreview"] = (
    documentId,
    documentTitle,
  ) => {
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

  return { loadDocumentPreview, loadViewerPage };
}
