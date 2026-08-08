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

  return {
    loadUnlockSources,
    openDocumentUnlock,
    selectUnlockSource,
    submitDocumentPassword,
  };
}
