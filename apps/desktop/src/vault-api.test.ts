import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { commandErrorMessage, createVaultApi, type TauriInvoke } from "./vault-api";

describe("Vault API", () => {
  it("allows only local and rendered in-memory images through the Tauri CSP", () => {
    const config = JSON.parse(
      readFileSync(
        new URL("../src-tauri/tauri.conf.json", import.meta.url),
        "utf8",
      ),
    ) as { app: { security: { csp: string } } };

    expect(config.app.security.csp).toContain("img-src 'self' data:");
  });

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
    await api.renderSourceDocumentPage("document-1", 2);

    expect(calls).toEqual([
      ["vault_status", undefined],
      ["create_vault", { password: "password" }],
      ["unlock_vault", { password: "password" }],
      ["lock_vault", undefined],
      ["import_source_document", undefined],
      ["list_unassigned_source_documents", undefined],
      ["normalize_source_document", { documentId: "document-1" }],
      ["render_source_document_page", { documentId: "document-1", pageNumber: 2 }],
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
      "This file is no longer available.",
    );
    expect(commandErrorMessage('{"code":"invalid_document_request"}')).toBe(
      "That PDF page isn’t available.",
    );
    expect(commandErrorMessage('{"code":"viewer_unsupported"}')).toBe(
      "Preview is only available for PDF evidence.",
    );
    expect(commandErrorMessage('{"code":"document_render_failed"}')).toBe(
      "CanCan couldn’t render that PDF page.",
    );
    expect(commandErrorMessage("private backend detail")).toBe(
      "Couldn’t complete that request. Try again.",
    );
  });
});
