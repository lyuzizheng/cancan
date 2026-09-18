import type {
  SourceConfirmationPrompt,
  SourceDocumentSummary,
  TaskRow,
} from "./command-contracts";
import type { Notice } from "./feedback";
import type { MoneySourceDocuments } from "./sources-view";
import type { AppView } from "./vault-spine";
import type { ViewerActions } from "./viewer-actions";

export interface TaskDestinationActions {
  openTaskDestination(row: TaskRow): void;
  viewDocument(document: SourceDocumentSummary, trigger?: HTMLButtonElement): void;
  viewPromptDocument(prompt: SourceConfirmationPrompt): void;
}

export interface TaskDestinationActionDeps extends ViewerActions {
  navigate: (view: AppView) => void;
  openDocumentUnlock: (document: SourceDocumentSummary) => void;
  saveRecoveryFile: () => void;
  selectMoneySource: (moneySourceId: string) => void;
  selectedMoneySourceIdRef: { current: string | null };
  setFocusedCandidateId: (candidateId: string | null) => void;
  setNotice: (notice: Notice) => void;
  sourceDocuments: MoneySourceDocuments[];
  unassignedDocuments: SourceDocumentSummary[];
  viewerReturnFocus: { current: HTMLButtonElement | null };
}

/**
 * Deep-link routing for the frozen Tasks contract: every destination kind
 * lands on its owning surface. Receipt and inbox-issue detail panels arrive
 * with the Sources rebuild; until then those rows route to the Sources view
 * that owns them.
 */
export function createTaskDestinationActions(
  deps: TaskDestinationActionDeps,
): TaskDestinationActions {
  const {
    loadDocumentPreview,
    loadViewerPage,
    navigate,
    openDocumentUnlock,
    saveRecoveryFile,
    selectMoneySource,
    selectedMoneySourceIdRef,
    setFocusedCandidateId,
    setNotice,
    sourceDocuments,
    unassignedDocuments,
    viewerReturnFocus,
  } = deps;

  const findDocumentById = (documentId: string): SourceDocumentSummary | undefined =>
    sourceDocuments
    .flatMap((entry) => entry.detail?.documents ?? [])
      .concat(unassignedDocuments)
      .find((document) => document.documentId === documentId);

  const viewDocument: TaskDestinationActions["viewDocument"] = (document, trigger) => {
    viewerReturnFocus.current = trigger ?? null;
    if (document.mimeType === "text/csv") {
      loadDocumentPreview(document.documentId, document.originalFilename);
    } else {
      loadViewerPage(document.documentId, document.originalFilename, 1);
    }
  };

  const viewPromptDocument: TaskDestinationActions["viewPromptDocument"] = (prompt) => {
    if (prompt.latestDocumentId === null) {
      return;
    }
    const document = findDocumentById(prompt.latestDocumentId);
    if (document === undefined) {
      setNotice({
        body: "That document is still being processed. Open it from Sources once it lands.",
        tone: "attention",
        title: "Document not ready yet",
      });
      return;
    }
    setFocusedCandidateId(null);
    viewDocument(document);
  };

  const openTaskDestination: TaskDestinationActions["openTaskDestination"] = (row) => {
    const destination = row.destination;
    switch (destination.kind) {
      case "document": {
        const owner = sourceDocuments.find((entry) =>
          (entry.detail?.documents ?? []).some((doc) => doc.documentId === destination.documentId));
        navigate("sources");
        if (owner !== undefined && selectedMoneySourceIdRef.current !== owner.source.moneySourceId) {
          selectMoneySource(owner.source.moneySourceId);
        }
        return;
      }
      case "inbox_issue":
      case "receipt": {
        navigate("sources");
        return;
      }
      case "password": {
        const document = findDocumentById(destination.documentId);
        if (document !== undefined) {
          openDocumentUnlock(document);
        } else {
          navigate("sources");
          selectMoneySource(destination.moneySourceId);
        }
        return;
      }
      case "recovery_setup": {
        saveRecoveryFile();
        return;
      }
      case "review_group": {
        navigate("review");
        return;
      }
      case "source_confirmation": {
        setFocusedCandidateId(destination.moneySourceCandidateId);
        return;
      }
    }
  };

  return { openTaskDestination, viewDocument, viewPromptDocument };
}
