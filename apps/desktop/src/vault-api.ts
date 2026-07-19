import { invoke } from "@tauri-apps/api/core";

import type {
  NormalizeSourceDocumentArgs,
  SourceDocumentImportOutcome,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
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

export interface VaultApi {
  createVault(password: string): Promise<VaultStatus>;
  importSourceDocument(): Promise<SourceDocumentImportOutcome | null>;
  listUnassignedSourceDocuments(): Promise<SourceDocumentSummary[]>;
  lockVault(): Promise<VaultStatus>;
  normalizeSourceDocument(
    documentId: string,
  ): Promise<SourceDocumentRoutingOutcome>;
  unlockVault(password: string): Promise<VaultStatus>;
  vaultStatus(): Promise<VaultStatus>;
}

const tauriInvoke: TauriInvoke = (command, args) =>
  invoke(command, args as Record<string, unknown> | undefined);

export function createVaultApi(call: TauriInvoke = tauriInvoke): VaultApi {
  return {
    vaultStatus: () => call<VaultStatus>("vault_status"),
    createVault: (password) => {
      const args: VaultPasswordArgs = { password };
      return call<VaultStatus, VaultPasswordArgs>("create_vault", args);
    },
    unlockVault: (password) => {
      const args: VaultPasswordArgs = { password };
      return call<VaultStatus, VaultPasswordArgs>("unlock_vault", args);
    },
    lockVault: () => call<VaultStatus>("lock_vault"),
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
  };
}

export function commandErrorMessage(error: unknown): string {
  switch (commandErrorCode(error)) {
    case "invalid_credentials":
      return "That password did not unlock this Vault.";
    case "password_required":
      return "Enter a password to continue.";
    case "vault_locked":
      return "Unlock your Vault to continue.";
    case "vault_not_created":
      return "Create your Vault before adding evidence.";
    case "unsupported_document":
      return "Choose a PDF or CSV file.";
    case "normalizer_failed":
      return "CanCan could not finish the secure document check. Try again.";
    case "document_unavailable":
      return "This file is no longer available for routing.";
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
