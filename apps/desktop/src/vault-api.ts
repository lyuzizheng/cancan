import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  DeleteSourceDocumentArgs,
  NormalizeSourceDocumentArgs,
  RenderedDocumentPage,
  RenderSourceDocumentPageArgs,
  SourceDocumentImportOutcome,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
  VaultAccessStatus,
  VaultPasswordArgs,
  VaultStatus,
} from "./command-contracts";

export type TauriInvoke = <
  Result,
  Args extends Record<string, unknown> | undefined = undefined,
>(
  command: string,
  args?: Args,
) => Promise<Result>;

export type TauriListen = (
  event: string,
  handler: () => void,
) => Promise<() => void>;

export interface VaultApi {
  createVault(password: string): Promise<VaultStatus>;
  deleteSourceDocument(documentId: string): Promise<boolean>;
  forgetVaultOnThisMac(): Promise<void>;
  importSourceDocument(): Promise<SourceDocumentImportOutcome | null>;
  listUnassignedSourceDocuments(): Promise<SourceDocumentSummary[]>;
  lockVault(): Promise<VaultStatus>;
  normalizeSourceDocument(
    documentId: string,
  ): Promise<SourceDocumentRoutingOutcome>;
  onVaultLocked(handler: () => void): Promise<() => void>;
  rememberVaultOnThisMac(): Promise<void>;
  renderSourceDocumentPage(
    documentId: string,
    pageNumber: number,
  ): Promise<RenderedDocumentPage>;
  saveRecoveryFile(): Promise<boolean>;
  unlockVault(password: string): Promise<VaultStatus>;
  unlockVaultWithKeychain(): Promise<VaultStatus>;
  vaultAccessStatus(): Promise<VaultAccessStatus>;
  vaultStatus(): Promise<VaultStatus>;
}

const tauriInvoke: TauriInvoke = (command, args) =>
  invoke(command, args as Record<string, unknown> | undefined);
const tauriListen: TauriListen = (event, handler) => listen(event, handler);

export function createVaultApi(
  call: TauriInvoke = tauriInvoke,
  subscribe: TauriListen = tauriListen,
): VaultApi {
  return {
    vaultAccessStatus: () => call<VaultAccessStatus>("vault_access_status"),
    vaultStatus: () => call<VaultStatus>("vault_status"),
    createVault: (password) => {
      const args: VaultPasswordArgs = { password };
      return call<VaultStatus, VaultPasswordArgs>("create_vault", args);
    },
    unlockVault: (password) => {
      const args: VaultPasswordArgs = { password };
      return call<VaultStatus, VaultPasswordArgs>("unlock_vault", args);
    },
    unlockVaultWithKeychain: () =>
      call<VaultStatus>("unlock_vault_with_keychain"),
    rememberVaultOnThisMac: () => call<void>("remember_vault_on_this_mac"),
    forgetVaultOnThisMac: () => call<void>("forget_vault_on_this_mac"),
    lockVault: () => call<VaultStatus>("lock_vault"),
    saveRecoveryFile: () => call<boolean>("save_recovery_file"),
    deleteSourceDocument: (documentId) => {
      const args: DeleteSourceDocumentArgs = { documentId };
      return call<boolean, DeleteSourceDocumentArgs>(
        "delete_source_document",
        args,
      );
    },
    importSourceDocument: () =>
      call<SourceDocumentImportOutcome | null>("import_source_document"),
    listUnassignedSourceDocuments: () =>
      call<SourceDocumentSummary[]>("list_unassigned_source_documents"),
    normalizeSourceDocument: (documentId) => {
      const args: NormalizeSourceDocumentArgs = { documentId };
      return call<SourceDocumentRoutingOutcome, NormalizeSourceDocumentArgs>(
        "normalize_source_document",
        args,
      );
    },
    onVaultLocked: (handler) => subscribe("vault-locked", handler),
    renderSourceDocumentPage: (documentId, pageNumber) => {
      const args: RenderSourceDocumentPageArgs = { documentId, pageNumber };
      return call<RenderedDocumentPage, RenderSourceDocumentPageArgs>(
        "render_source_document_page",
        args,
      );
    },
  };
}

export function commandErrorMessage(error: unknown): string {
  switch (commandErrorCode(error)) {
    case "invalid_credentials":
      return "That password did not unlock this Vault.";
    case "password_required":
      return "Enter a password to continue.";
    case "remembered_unlock_unavailable":
      return "Remembered unlock is no longer available. Use your Vault password instead.";
    case "remembered_unlock_failed":
      return "CanCan couldn’t access remembered unlock in this Mac’s Keychain.";
    case "remember_failed":
      return "CanCan couldn’t save remembered unlock in this Mac’s Keychain.";
    case "forget_failed":
      return "CanCan couldn’t remove remembered unlock from this Mac’s Keychain.";
    case "vault_locked":
      return "Unlock your Vault to continue.";
    case "vault_not_created":
      return "Create your Vault before adding evidence.";
    case "recovery_already_configured":
      return "A recovery file is already configured for this Vault.";
    case "recovery_location_invalid":
      return "Save the recovery file somewhere outside your CanCan Vault.";
    case "recovery_create_failed":
      return "CanCan couldn’t create a recovery file safely.";
    case "recovery_save_failed":
      return "CanCan couldn’t save the recovery file to that location.";
    case "recovery_status_failed":
      return "The recovery file was saved, but CanCan couldn’t record setup. Keep the file private and try again.";
    case "unsupported_document":
      return "Choose a PDF or CSV file.";
    case "normalizer_failed":
      return "CanCan could not finish the secure document check. Try again.";
    case "document_unavailable":
      return "This file is no longer available.";
    case "invalid_document_request":
      return "That document request isn’t valid.";
    case "viewer_unsupported":
      return "Preview isn’t available for this evidence.";
    case "document_render_failed":
      return "CanCan couldn’t render that PDF page.";
    case "delete_source_failed":
      return "CanCan couldn’t finish removing this Vault file. Refresh its status before trying again.";
    default:
      return "Couldn’t complete that request. Try again.";
  }
}

function commandErrorCode(error: unknown): string | null {
  if (typeof error === "object" && error !== null && "code" in error) {
    const { code } = error;
    return typeof code === "string" ? code : null;
  }

  if (typeof error !== "string") {
    return null;
  }

  try {
    const parsed: unknown = JSON.parse(error);
    return commandErrorCode(parsed);
  } catch {
    return null;
  }
}
