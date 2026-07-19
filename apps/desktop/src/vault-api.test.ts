import { describe, expect, it } from "vitest";

import { commandErrorMessage, createVaultApi, type TauriInvoke } from "./vault-api";

describe("Vault API", () => {
  it("uses only the established command names and safe request fields", async () => {
    const calls: Array<[string, Record<string, unknown> | undefined]> = [];
    const invoke = (async (command, args) => {
      calls.push([command, args]);
      return command === "list_unassigned_source_documents" ? [] : null;
    }) as TauriInvoke;
    const api = createVaultApi(invoke);

    await api.vaultStatus();
    await api.createVault("password");
    await api.unlockVault("password");
    await api.lockVault();
    await api.importSourceDocument();
    await api.listUnassignedSourceDocuments();
    await api.normalizeSourceDocument("document-1");

    expect(calls).toEqual([
      ["vault_status", undefined],
      ["create_vault", { password: "password" }],
      ["unlock_vault", { password: "password" }],
      ["lock_vault", undefined],
      ["import_source_document", undefined],
      ["list_unassigned_source_documents", undefined],
      ["normalize_source_document", { documentId: "document-1" }],
    ]);
  });

  it("maps command failures to safe user-facing copy", () => {
    expect(commandErrorMessage({ code: "vault_locked" })).toBe(
      "Unlock your Vault to continue.",
    );
    expect(commandErrorMessage('{"code":"normalizer_failed"}')).toBe(
      "CanCan could not finish the secure document check. Try again.",
    );
    expect(commandErrorMessage('{"code":"document_unavailable"}')).toBe(
      "This file is no longer available for routing.",
    );
    expect(commandErrorMessage("private backend detail")).toBe(
      "Couldn’t complete that request. Try again.",
    );
  });
});
