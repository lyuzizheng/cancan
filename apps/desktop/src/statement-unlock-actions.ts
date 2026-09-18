import type { Dispatch, SetStateAction } from "react";

import type { SourceDocumentSummary } from "./command-contracts";
import type { DocumentUnlockState } from "./document-modals";
import type { Notice } from "./feedback";
import { commandErrorMessage, type VaultApi } from "./vault-api";

export interface StatementUnlockActions {
  loadUnlockSources(documentId: string, documentTitle: string): void;
  openDocumentUnlock(document: SourceDocumentSummary): void;
  selectUnlockSource(moneySourceId: string): void;
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
    void (async () => {
      let sources;
      try {
        sources = await api.listStatementPasswordSources();
      } catch (nextError) {
        if (unlockRequestId.current === requestId) {
          setUnlockingDocument((current) => current?.documentId === documentId
            ? { ...current, busy: false, error: commandErrorMessage(nextError), sources: [] }
            : current);
        }
        return;
      }
      if (unlockRequestId.current !== requestId) {
        return;
      }
      const onlySource = sources.length === 1 ? sources[0] : undefined;
      const hasSavedPassword = sources.some((source) => source.hasSavedPassword);
      setUnlockingDocument((current) => current?.documentId === documentId
        ? {
          ...current,
          busy: hasSavedPassword,
          savedPasswordStatus: null,
          selectedMoneySourceId: onlySource?.moneySourceId ?? "",
          sources,
        }
        : current);
      if (!hasSavedPassword) {
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

  return {
    loadUnlockSources,
    openDocumentUnlock,
    selectUnlockSource,
    submitDocumentPassword,
  };
}
