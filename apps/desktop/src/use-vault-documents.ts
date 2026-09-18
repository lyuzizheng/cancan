import { useCallback, useEffect, useRef, useState } from "react";

import type {
  MoneySourceSummary,
  SourceDocumentSummary,
  SupportedMoneySourceProviderSummary,
} from "./command-contracts";
import { commandErrorMessage, type VaultApi } from "./vault-api";
import { importNotice } from "./notices";
import type { MoneySourceDocuments } from "./sources-view";
import type { VaultSession } from "./use-vault-session";

export interface VaultDocuments {
  cancelDelete(): void;
  /** Clears the open source detail and returns the Sources list. */
  clearMoneySourceSelection(): void;
  readonly confirmingDelete: SourceDocumentSummary | null;
  /** Commits the create dialog's provider and optional name. */
  createMoneySource(): Promise<void>;
  readonly deletingDocumentId: string | null;
  /** Commits the rename dialog's display name. */
  editMoneySource(): Promise<void>;
  readonly importing: boolean;
  importDocument(): void;
  readonly loadingDocuments: boolean;
  loadDocuments(requestedSourceId?: string | null): Promise<void>;
  normalizeDocument(documentId: string): void;
  readonly normalizingDocumentId: string | null;
  /** Opens the create dialog and loads the supported-provider catalog. */
  openCreateMoneySource(): void;
  /** Opens the rename dialog for one configured source. */
  openEditMoneySource(source: MoneySourceSummary): void;
  /** Opens the saved-statement-password removal confirmation. */
  openRemoveSourcePassword(source: MoneySourceSummary): void;
  /** Removes the saved statement password for the confirmed source. */
  removeSourceStatementPassword(): Promise<void>;
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
  /** Closes whichever source dialog is open without committing. */
  closeSourceDialog(): void;
  /** Retries the provider catalog after a failed load inside the create dialog. */
  retrySourceProviders(): void;
  setSourceDialogDisplayName(displayName: string): void;
  setSourceDialogProviderKey(providerKey: string): void;
  readonly sourceDialog: SourceDialogState | null;
  readonly sourceDocuments: MoneySourceDocuments[];
  readonly unassignedDocuments: SourceDocumentSummary[];
  updateRemembered(remembered: boolean): void;
  readonly updatingRemembered: boolean;
}

/**
 * The source-management dialogs. `create` carries the supported-provider
 * catalog (`null` while loading, `[]` after a failed load so the dialog can
 * offer its retry); `rename` and `remove_password` carry their target source
 * and the editable display name.
 */
export type SourceDialogState =
  | {
      busy: boolean;
      displayName: string;
      error: string | null;
      kind: "create";
      providerKey: string;
      providers: SupportedMoneySourceProviderSummary[] | null;
    }
  | {
      busy: boolean;
      displayName: string;
      error: string | null;
      kind: "rename";
      source: MoneySourceSummary;
    }
  | {
      busy: boolean;
      error: string | null;
      kind: "remove_password";
      source: MoneySourceSummary;
    };

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
  const [sourceDialog, setSourceDialog] = useState<SourceDialogState | null>(null);
  const sourceDialogRequestId = useRef(0);
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
    setSourceDialog(null);
    sourceDialogRequestId.current += 1;
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
      let selectedDetailError: unknown = null;
      const selectedDetailPromise = selectedSourceId === null
        ? Promise.resolve(null)
        : api.getMoneySourceDetail(selectedSourceId).catch((nextError: unknown) => {
            selectedDetailError = nextError;
            return null;
          });
      const [sources, unassigned, selectedDetailForId] = await Promise.all([
        api.listMoneySources(),
        api.listUnassignedSourceDocuments(),
        selectedDetailPromise,
      ]);
      const selectedSource = selectedSourceId === null
        ? undefined
        : sources.find((source) => source.moneySourceId === selectedSourceId);
      if (selectedSource && selectedDetailError !== null) {
        throw selectedDetailError;
      }
      const selectedDetail = selectedSource ? selectedDetailForId : null;
      if (
        currentSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        const nextSelectedSourceId = selectedSource?.moneySourceId ?? null;
        selectedMoneySourceIdRef.current = nextSelectedSourceId;
        setSelectedMoneySourceId(nextSelectedSourceId);
        setSourceDocuments(sources.map((source) => ({
          detail: source.moneySourceId === nextSelectedSourceId
            ? selectedDetail
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
      detail: null,
    })));
    setError(null);
    setLoadingDocuments(true);
    void api.getMoneySourceDetail(moneySourceId).then((detail) => {
      if (
        currentSessionId.current === sessionId
        && documentLoadRequestId.current === requestId
      ) {
        setSourceDocuments((current) => current.map((entry) => (
          entry.source.moneySourceId === moneySourceId
            ? { ...entry, detail }
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

  const clearMoneySourceSelection = useCallback(() => {
    const requestId = documentLoadRequestId.current + 1;
    documentLoadRequestId.current = requestId;
    selectedMoneySourceIdRef.current = null;
    setSelectedMoneySourceId(null);
    setSourceDocuments((current) => current.map((entry) => ({
      ...entry,
      detail: null,
    })));
  }, []);
  const closeSourceDialog = useCallback(() => {
    sourceDialogRequestId.current += 1;
    setSourceDialog(null);
  }, []);

  const setSourceDialogDisplayName = useCallback((displayName: string) => {
    setSourceDialog((current) => current === null || current.kind === "remove_password"
      ? current
      : { ...current, displayName });
  }, []);

  const setSourceDialogProviderKey = useCallback((providerKey: string) => {
    setSourceDialog((current) => current?.kind === "create"
      ? { ...current, providerKey }
      : current);
  }, []);

  const loadSourceProviders = useCallback(() => {
    const requestId = sourceDialogRequestId.current + 1;
    sourceDialogRequestId.current = requestId;
    setSourceDialog((current) => current?.kind === "create"
      ? { ...current, error: null, providers: null }
      : current);
    void api.listSupportedMoneySourceProviders().then((providers) => {
      if (sourceDialogRequestId.current === requestId) {
        setSourceDialog((current) => current?.kind === "create"
          ? { ...current, providers }
          : current);
      }
    }).catch((nextError: unknown) => {
      if (sourceDialogRequestId.current === requestId) {
        setSourceDialog((current) => current?.kind === "create"
          ? { ...current, error: commandErrorMessage(nextError), providers: [] }
          : current);
      }
    });
  }, [api]);

  const openCreateMoneySource = useCallback(() => {
    setSourceDialog({
      busy: false,
      displayName: "",
      error: null,
      kind: "create",
      providerKey: "",
      providers: null,
    });
    loadSourceProviders();
  }, [loadSourceProviders]);

  const retrySourceProviders = useCallback(() => {
    loadSourceProviders();
  }, [loadSourceProviders]);

  const openEditMoneySource = useCallback((source: MoneySourceSummary) => {
    sourceDialogRequestId.current += 1;
    setSourceDialog({
      busy: false,
      displayName: source.displayName,
      error: null,
      kind: "rename",
      source,
    });
  }, []);

  const openRemoveSourcePassword = useCallback((source: MoneySourceSummary) => {
    sourceDialogRequestId.current += 1;
    setSourceDialog({
      busy: false,
      error: null,
      kind: "remove_password",
      source,
    });
  }, []);

  /**
   * One guarded command for the three source dialogs: the dialog stays open
   * with its own error line on failure, and closes only after the command and
   * the document reload both succeed. The request id fences the whole
   * mutation, so a close or a newer dialog can never be overwritten by a
   * stale settle.
   */
  const runSourceDialogCommand = useCallback(async (
    command: () => Promise<unknown>,
    afterSuccess: () => Promise<void>,
  ) => {
    const requestId = sourceDialogRequestId.current;
    setSourceDialog((current) => current === null
      ? current
      : { ...current, busy: true, error: null });
    const result = await runGuarded(command, {
      onError: (message) => {
        if (sourceDialogRequestId.current === requestId) {
          setSourceDialog((current) => current === null
            ? current
            : { ...current, error: message });
        }
      },
    });
    if (result === null || sourceDialogRequestId.current !== requestId) {
      setSourceDialog((current) => current === null
        ? current
        : { ...current, busy: false });
      return;
    }
    await afterSuccess();
    if (sourceDialogRequestId.current === requestId) {
      setSourceDialog(null);
    }
  }, [runGuarded]);

  const createMoneySource = useCallback(async () => {
    const dialog = sourceDialog;
    if (dialog?.kind !== "create" || dialog.providerKey === "") {
      return;
    }
    const displayName = dialog.displayName.trim();
    let createdId: string | null = null;
    await runSourceDialogCommand(
      async () => {
        const created = await api.createMoneySource(
          dialog.providerKey,
          displayName === "" ? null : displayName,
        );
        createdId = created.moneySourceId;
      },
      async () => {
        setNotice({
          body: "CanCan routes new evidence for this provider to it automatically.",
          tone: "success",
          title: "Money Source added",
        });
        await loadDocuments();
        if (createdId !== null) {
          selectMoneySource(createdId);
        }
      },
    );
  }, [api, loadDocuments, runSourceDialogCommand, selectMoneySource, setNotice, sourceDialog]);

  const editMoneySource = useCallback(async () => {
    const dialog = sourceDialog;
    if (dialog?.kind !== "rename") {
      return;
    }
    const displayName = dialog.displayName.trim();
    if (displayName === "" || displayName === dialog.source.displayName) {
      return;
    }
    await runSourceDialogCommand(
      () => api.editMoneySource(dialog.source.moneySourceId, displayName),
      async () => {
        setNotice({
          body: "The source keeps its provider, documents, and account links.",
          tone: "success",
          title: "Money Source renamed",
        });
        await loadDocuments();
      },
    );
  }, [api, loadDocuments, runSourceDialogCommand, setNotice, sourceDialog]);

  const removeSourceStatementPassword = useCallback(async () => {
    const dialog = sourceDialog;
    if (dialog?.kind !== "remove_password") {
      return;
    }
    await runSourceDialogCommand(
      () => api.removeStatementPassword(dialog.source.moneySourceId),
      async () => {
        setNotice({
          body: "Protected statements for this source ask for their password again.",
          tone: "success",
          title: "Saved statement password removed",
        });
        await loadDocuments();
      },
    );
  }, [api, loadDocuments, runSourceDialogCommand, setNotice, sourceDialog]);

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
    clearMoneySourceSelection,
    closeSourceDialog,
    confirmingDelete,
    createMoneySource,
    deleteDocument,
    deletingDocumentId,
    editMoneySource,
    importDocument,
    importing,
    loadDocuments,
    loadingDocuments,
    normalizeDocument,
    normalizingDocumentId,
    openCreateMoneySource,
    openEditMoneySource,
    openRemoveSourcePassword,
    removeSourceStatementPassword,
    reset,
    retrySourceProviders,
    saveRecoveryFile,
    saveSourceCopy,
    savingCopyDocumentId,
    savingRecoveryFile,
    selectMoneySource,
    selectedMoneySourceId,
    selectedMoneySourceIdRef,
    setConfirmingDelete,
    setSourceDialogDisplayName,
    setSourceDialogProviderKey,
    sourceDialog,
    sourceDocuments,
    unassignedDocuments,
    updateRemembered,
    updatingRemembered,
  };
}
