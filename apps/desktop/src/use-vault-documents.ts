import { useCallback, useEffect, useRef, useState } from "react";

import type { SourceDocumentSummary } from "./command-contracts";
import { commandErrorMessage, type VaultApi } from "./vault-api";
import { importNotice } from "./notices";
import type { MoneySourceDocuments } from "./sources-view";
import type { VaultSession } from "./use-vault-session";

export interface VaultDocuments {
  cancelDelete(): void;
  readonly confirmingDelete: SourceDocumentSummary | null;
  readonly deletingDocumentId: string | null;
  readonly importing: boolean;
  importDocument(): void;
  readonly loadingDocuments: boolean;
  loadDocuments(requestedSourceId?: string | null): Promise<void>;
  normalizeDocument(documentId: string): void;
  readonly normalizingDocumentId: string | null;
  /** Drops every session-scoped document state; the gate's reset half. */
  reset(): void;
  saveRecoveryFile(): void;
  readonly savingCopyDocumentId: string | null;
  readonly savingRecoveryFile: boolean;
  saveSourceCopy(documentId: string): void;
  selectMoneySource(moneySourceId: string): void;
  readonly selectedMoneySourceId: string | null;
  readonly selectedMoneySourceIdRef: { current: string | null };
  setConfirmingDelete(document: SourceDocumentSummary | null): void;
  deleteDocument(documentId: string): void;
  readonly sourceDocuments: MoneySourceDocuments[];
  readonly unassignedDocuments: SourceDocumentSummary[];
  updateRemembered(remembered: boolean): void;
  readonly updatingRemembered: boolean;
}

/**
 * Owns the evidence intake domain: Money Sources, their documents, unassigned
 * documents, and the intake commands that change them (import, parser re-run,
 * delete, save a copy, recovery file, Touch ID preference).
 */
export function useVaultDocuments({
  api,
  session,
}: {
  api: VaultApi;
  session: VaultSession;
}): VaultDocuments {
  const [unassignedDocuments, setUnassignedDocuments] = useState<
    SourceDocumentSummary[]
  >([]);
  const [sourceDocuments, setSourceDocuments] = useState<MoneySourceDocuments[]>([]);
  const [selectedMoneySourceId, setSelectedMoneySourceId] = useState<string | null>(
    null,
  );
  const [loadingDocuments, setLoadingDocuments] = useState(false);
  const [importing, setImporting] = useState(false);
  const [normalizingDocumentId, setNormalizingDocumentId] = useState<string | null>(
    null,
  );
  const [deletingDocumentId, setDeletingDocumentId] = useState<string | null>(null);
  const [confirmingDelete, setConfirmingDelete] = useState<SourceDocumentSummary | null>(
    null,
  );
  const [savingCopyDocumentId, setSavingCopyDocumentId] = useState<string | null>(null);
  const [savingRecoveryFile, setSavingRecoveryFile] = useState(false);
  const [updatingRemembered, setUpdatingRemembered] = useState(false);
  const documentLoadRequestId = useRef(0);
  const selectedMoneySourceIdRef = useRef<string | null>(null);

  useEffect(() => () => {
    documentLoadRequestId.current += 1;
  }, []);

  const {
    documentsAllowed,
    isCurrent,
    runGuarded,
    sessionId: currentSessionId,
    setError,
    dismissTouchIdOffer,
    setNotice,
    setRememberedOnThisMac,
    setRecoveryConfigured,
  } = session;

  const reset = useCallback(() => {
    documentLoadRequestId.current += 1;
    setUnassignedDocuments([]);
    setSourceDocuments([]);
    selectedMoneySourceIdRef.current = null;
    setSelectedMoneySourceId(null);
    setLoadingDocuments(false);
    setDeletingDocumentId(null);
    setConfirmingDelete(null);
    setSavingCopyDocumentId(null);
  }, []);

  const cancelDelete = useCallback(() => setConfirmingDelete(null), []);

  const loadDocuments = useCallback(async (requestedSourceId?: string | null) => {
    if (!documentsAllowed.current) {
      return;
    }
    const sessionId = currentSessionId.current;
    const requestId = documentLoadRequestId.current + 1;
    const selectedSourceId = requestedSourceId === undefined
      ? selectedMoneySourceIdRef.current
      : requestedSourceId;
    documentLoadRequestId.current = requestId;
    setLoadingDocuments(true);
    try {
      // The selected-source fetch is speculative: it runs in parallel with
      // the source list, so a stale id's failure is discarded, while a real
      // failure for a still-valid source is re-thrown below.
      let selectedDocumentsError: unknown = null;
      const selectedDocumentsPromise = selectedSourceId === null
        ? Promise.resolve(null)
        : api.listSourceDocuments(selectedSourceId).catch((nextError: unknown) => {
            selectedDocumentsError = nextError;
            return null;
          });
      const [sources, unassigned, selectedDocumentsForId] = await Promise.all([
        api.listMoneySources(),
        api.listUnassignedSourceDocuments(),
        selectedDocumentsPromise,
      ]);
      const selectedSource = selectedSourceId === null
        ? undefined
        : sources.find((source) => source.moneySourceId === selectedSourceId);
      if (selectedSource && selectedDocumentsError !== null) {
        throw selectedDocumentsError;
      }
      const selectedDocuments = selectedSource ? selectedDocumentsForId : null;
      if (
        currentSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        const nextSelectedSourceId = selectedSource?.moneySourceId ?? null;
        selectedMoneySourceIdRef.current = nextSelectedSourceId;
        setSelectedMoneySourceId(nextSelectedSourceId);
        setSourceDocuments(sources.map((source) => ({
          documents: source.moneySourceId === nextSelectedSourceId
            ? selectedDocuments
            : null,
          source,
        })));
        setUnassignedDocuments(unassigned);
      }
    } catch (nextError) {
      if (
        currentSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (
        currentSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setLoadingDocuments(false);
      }
    }
  }, [api, currentSessionId, documentsAllowed, setError]);

  const selectMoneySource = useCallback((moneySourceId: string) => {
    if (!documentsAllowed.current) {
      return;
    }
    const sessionId = currentSessionId.current;
    const requestId = documentLoadRequestId.current + 1;
    documentLoadRequestId.current = requestId;
    selectedMoneySourceIdRef.current = moneySourceId;
    setSelectedMoneySourceId(moneySourceId);
    setSourceDocuments((current) => current.map((entry) => ({
      ...entry,
      documents: null,
    })));
    setError(null);
    setLoadingDocuments(true);
    void api.listSourceDocuments(moneySourceId).then((documents) => {
      if (
        currentSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setSourceDocuments((current) => current.map((entry) => (
          entry.source.moneySourceId === moneySourceId
            ? { ...entry, documents }
            : entry
        )));
      }
    }).catch((nextError) => {
      if (
        currentSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setError(commandErrorMessage(nextError));
      }
    }).finally(() => {
      if (
        currentSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setLoadingDocuments(false);
      }
    });
  }, [api, currentSessionId, documentsAllowed, setError]);

  const updateRemembered = useCallback((remembered: boolean) => {
    setUpdatingRemembered(true);
    void runGuarded(async () => {
      if (remembered) {
        await api.rememberVaultOnThisMac();
      } else {
        await api.forgetVaultOnThisMac();
      }
      setRememberedOnThisMac(remembered);
      // Any successful Touch ID toggle resolves the post-password offer.
      dismissTouchIdOffer();
      setNotice(
        remembered
          ? {
              body: "CanCan can unlock your Vault with Touch ID without running the password check.",
              tone: "success",
              title: "Touch ID unlock enabled",
            }
          : {
              body: "Your current Vault stays open. Your password will be required after you lock or restart CanCan.",
              tone: "success",
              title: "Touch ID unlock removed",
            },
      );
    }, { busy: true, onSettled: () => setUpdatingRemembered(false) });
  }, [api, dismissTouchIdOffer, runGuarded, setNotice, setRememberedOnThisMac]);

  const importDocument = useCallback(() => {
    setImporting(true);
    void runGuarded(async () => {
      setNotice(null);
      const outcome = await api.importSourceDocument();
      setNotice(importNotice(outcome?.status ?? "cancelled"));
      if (outcome) {
        await loadDocuments();
      }
    }, { busy: true, onSettled: () => setImporting(false) });
  }, [api, loadDocuments, runGuarded, setNotice]);

  const saveRecoveryFile = useCallback(() => {
    setSavingRecoveryFile(true);
    void runGuarded(async () => {
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
    }, { busy: true, onSettled: () => setSavingRecoveryFile(false) });
  }, [api, runGuarded, setNotice, setRecoveryConfigured]);

  const normalizeDocument = useCallback((documentId: string) => {
    setNormalizingDocumentId(documentId);
    void runGuarded(async () => {
      await api.reparseSourceDocument(documentId);
      setNotice({
        body: "CanCan queued a new parser run. The source status will update when it finishes.",
        tone: "success",
        title: "Parser re-run started",
      });
      await loadDocuments();
    }, { busy: true, onSettled: () => setNormalizingDocumentId(null) });
  }, [api, loadDocuments, runGuarded, setNotice]);

  const deleteDocument = useCallback((documentId: string) => {
    setDeletingDocumentId(documentId);
    void runGuarded(async () => {
      try {
        if (await api.deleteSourceDocument(documentId)) {
          setNotice({
            body: "The encrypted file was removed. Its document history and links remain in CanCan.",
            tone: "success",
            title: "Source file deleted",
          });
        }
      } finally {
        await loadDocuments();
      }
    }, {
      busy: true,
      onSettled: () => {
        setDeletingDocumentId(null);
        setConfirmingDelete(null);
      },
    });
  }, [api, loadDocuments, runGuarded, setNotice]);

  const saveSourceCopy = useCallback((documentId: string) => {
    const sessionId = currentSessionId.current;
    setSavingCopyDocumentId(documentId);
    void runGuarded(async () => {
      if (
        await api.saveSourceDocumentCopy(documentId)
        && isCurrent(sessionId)
      ) {
        setNotice({
          body: "The copy is outside CanCan’s encrypted Vault and is now your responsibility.",
          tone: "success",
          title: "Copy saved",
        });
      }
    }, {
      busy: true,
      sessionId,
      onSettled: () => setSavingCopyDocumentId(null),
    });
  }, [api, currentSessionId, isCurrent, runGuarded, setNotice]);

  return {
    cancelDelete,
    confirmingDelete,
    deleteDocument,
    deletingDocumentId,
    importDocument,
    importing,
    loadDocuments,
    loadingDocuments,
    normalizeDocument,
    normalizingDocumentId,
    reset,
    saveRecoveryFile,
    saveSourceCopy,
    savingCopyDocumentId,
    savingRecoveryFile,
    selectMoneySource,
    selectedMoneySourceId,
    selectedMoneySourceIdRef,
    setConfirmingDelete,
    sourceDocuments,
    unassignedDocuments,
    updateRemembered,
    updatingRemembered,
  };
}
