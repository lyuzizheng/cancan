import type { Dispatch, SetStateAction } from "react";

import type {
  SourceDocumentSummary,
  StatementPasswordUnlockContext,
} from "./command-contracts";
import type { DocumentUnlockState } from "./document-modals";
import type { Notice } from "./feedback";
import { commandErrorMessage, type VaultApi } from "./vault-api";

export interface StatementUnlockActions {
  loadUnlockContext(documentId: string, documentTitle: string): void;
  openDocumentUnlock(document: SourceDocumentSummary): void;
  submitDocumentPassword(updateSavedPassword: boolean): void;
}

export interface StatementUnlockActionDeps {
  api: VaultApi;
  loadDocuments: () => Promise<void>;
  setNotice: (notice: Notice) => void;
  setUnlockingDocument: Dispatch<SetStateAction<DocumentUnlockState | null>>;
  unlockingDocument: DocumentUnlockState | null;
  unlockRequestId: { current: number };
}

/**
 * Orchestration for the protected-statement unlock dialog. Every attempt is
 * fenced by an incrementing request id so a stale response — or a Vault lock
 * that bumps the id — fails closed instead of mutating a newer dialog state.
 */
export function createStatementUnlockActions(
  deps: StatementUnlockActionDeps,
): StatementUnlockActions {
  const {
    api,
    loadDocuments,
    setNotice,
    setUnlockingDocument,
    unlockingDocument,
    unlockRequestId,
  } = deps;

  const trySavedStatementPasswords = async (
    documentId: string,
    requestId: number,
  ): Promise<
    { kind: "stopped" } | { kind: "status"; savedPasswordStatus: "invalid" | "unavailable" | null }
  > => {
    try {
      const result = await api.trySavedStatementPasswords(documentId);
      if (unlockRequestId.current !== requestId) {
        return { kind: "stopped" };
      }
      if (result === "unlocked") {
        setUnlockingDocument(null);
        setNotice({
          body: "The saved password worked. This statement is ready to view and route for this Vault session.",
          tone: "success",
          title: "Statement unlocked",
        });
        await loadDocuments();
        return { kind: "stopped" };
      }
      return { kind: "status", savedPasswordStatus: result };
    } catch (nextError) {
      if (unlockRequestId.current !== requestId) {
        return { kind: "stopped" };
      }
      // The host pass reports its own failure; manual entry stays available.
      setUnlockingDocument((current) => current?.documentId === documentId
        ? { ...current, error: commandErrorMessage(nextError) }
        : current);
      return { kind: "status", savedPasswordStatus: null };
    }
  };

  /**
   * Reads the document's host-confirmed owner and saved-password state, then
   * runs the host's bounded saved-password pass when the Vault holds one.
   *
   * The confirmed owner is the only source this prompt may offer to save or
   * replace for. The Vault-wide source list is read solely as the existence
   * probe that decides whether the host pass is worth running: it never
   * informs ownership or the save target.
   */
  const loadUnlockContext = (documentId: string, documentTitle: string) => {
    const requestId = unlockRequestId.current + 1;
    unlockRequestId.current = requestId;
    setUnlockingDocument({
      busy: true,
      context: null,
      documentId,
      documentTitle,
      error: null,
      password: "",
      savedPasswordStatus: null,
    });
    void (async () => {
      let context: StatementPasswordUnlockContext;
      let hasAnySavedPassword: boolean;
      try {
        const [confirmed, sources] = await Promise.all([
          api.statementPasswordDocumentContext(documentId),
          api.listStatementPasswordSources(),
        ]);
        context = confirmed;
        hasAnySavedPassword = sources.some((source) => source.hasSavedPassword);
      } catch (nextError) {
        if (unlockRequestId.current === requestId) {
          setUnlockingDocument((current) => current?.documentId === documentId
            ? { ...current, busy: false, error: commandErrorMessage(nextError) }
            : current);
        }
        return;
      }
      if (unlockRequestId.current !== requestId) {
        return;
      }
      setUnlockingDocument((current) => current?.documentId === documentId
        ? { ...current, busy: hasAnySavedPassword, context, savedPasswordStatus: null }
        : current);
      if (!hasAnySavedPassword) {
        return;
      }
      // The host owns the bounded pass: it tries each distinct saved password
      // once, so this surface only reports that none of them unlocked.
      const outcome = await trySavedStatementPasswords(documentId, requestId);
      if (outcome.kind === "stopped" || unlockRequestId.current !== requestId) {
        return;
      }
      setUnlockingDocument((current) => current?.documentId === documentId
        ? { ...current, busy: false, savedPasswordStatus: outcome.savedPasswordStatus }
        : current);
    })();
  };

  const openDocumentUnlock = (document: SourceDocumentSummary) => {
    loadUnlockContext(document.documentId, document.originalFilename);
  };

  const submitDocumentPassword = (updateSavedPassword: boolean) => {
    const current = unlockingDocument;
    if (!current) {
      return;
    }
    if (!current.password) {
      setUnlockingDocument({ ...current, error: "Enter the statement password to continue." });
      return;
    }
    const requestId = unlockRequestId.current + 1;
    unlockRequestId.current = requestId;
    const password = current.password;
    // A save or replacement targets only the owner the host confirmed for this
    // document — never a renderer choice — and a session-only unlock claims no
    // ownership at all, so it passes `null`. The host re-checks the document's
    // confirmed owner before writing or replacing any secret.
    const moneySourceId = updateSavedPassword ? current.context?.moneySourceId ?? null : null;
    const replacesSavedPassword =
      updateSavedPassword && current.context?.hasSavedPassword === true;
    setUnlockingDocument({ ...current, busy: true, error: null, password: "" });
    void api.unlockSourceDocument(
      current.documentId,
      moneySourceId,
      password,
      updateSavedPassword,
    ).then(async () => {
      if (unlockRequestId.current !== requestId) {
        return;
      }
      setUnlockingDocument(null);
      const body = !updateSavedPassword
        ? "This statement is ready to view and route until the Vault locks."
        : replacesSavedPassword
          ? "The verified password replaced this Money Source’s saved password. This statement is ready to view and route."
          : "The verified password is saved for this Money Source. This statement is ready to view and route.";
      setNotice({ body, tone: "success", title: "Statement unlocked" });
      await loadDocuments();
    }).catch((nextError) => {
      if (unlockRequestId.current === requestId) {
        setUnlockingDocument((latest) => latest?.documentId === current.documentId
          ? { ...latest, busy: false, error: commandErrorMessage(nextError), password: "" }
          : latest);
      }
    });
  };

  return {
    loadUnlockContext,
    openDocumentUnlock,
    submitDocumentPassword,
  };
}
