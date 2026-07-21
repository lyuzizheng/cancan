import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import {
  commandErrorMessage,
  createVaultApi,
  type TauriInvoke,
  type TauriListen,
} from "./vault-api";

describe("Vault API", () => {
  it("allows only local and rendered in-memory images through the Tauri CSP", () => {
    const config = JSON.parse(
      readFileSync(
        new URL("../src-tauri/tauri.conf.json", import.meta.url),
        "utf8",
      ),
    ) as { app: { security: { csp: string } } };

    const imageSources = config.app.security.csp
      .split(";")
      .map((directive) => directive.trim())
      .find((directive) => directive.startsWith("img-src "));

    expect(imageSources).toBe("img-src 'self' data:");
  });

  it("uses only the established command names and safe request fields", async () => {
    const calls: Array<[string, Record<string, unknown> | undefined]> = [];
    const invoke = (async (command, args) => {
      calls.push([command, args]);
      return command === "list_unassigned_source_documents" ? [] : null;
    }) as TauriInvoke;
    const listenedEvents: string[] = [];
    const subscribe = (async (event) => {
      listenedEvents.push(event);
      return () => undefined;
    }) as TauriListen;
    const api = createVaultApi(invoke, subscribe);

    await api.vaultStatus();
    await api.vaultAccessStatus();
    await api.createVault("password");
    await api.unlockVault("password");
    await api.unlockVaultWithKeychain();
    await api.rememberVaultOnThisMac();
    await api.forgetVaultOnThisMac();
    await api.listStatementPasswordSources();
    await api.trySavedStatementPassword("document-1", "source-dbs");
    await api.unlockSourceDocument(
      "document-1",
      "source-dbs",
      "statement-password",
      true,
    );
    await api.removeStatementPassword("source-dbs");
    await api.lockVault();
    await api.saveRecoveryFile();
    await api.deleteSourceDocument("document-1");
    await api.importSourceDocument();
    await api.listUnassignedSourceDocuments();
    const removeVaultLockListener = await api.onVaultLocked(() => undefined);
    removeVaultLockListener();
    await api.normalizeSourceDocument("document-1");
    await api.renderSourceDocumentPage("document-1", 2);

    expect(calls).toEqual([
      ["vault_status", undefined],
      ["vault_access_status", undefined],
      ["create_vault", { password: "password" }],
      ["unlock_vault", { password: "password" }],
      ["unlock_vault_with_keychain", undefined],
      ["remember_vault_on_this_mac", undefined],
      ["forget_vault_on_this_mac", undefined],
      ["list_statement_password_sources", undefined],
      [
        "try_saved_statement_password",
        { documentId: "document-1", moneySourceId: "source-dbs" },
      ],
      [
        "unlock_source_document",
        {
          documentId: "document-1",
          moneySourceId: "source-dbs",
          password: "statement-password",
          updateSavedPassword: true,
        },
      ],
      ["remove_statement_password", { moneySourceId: "source-dbs" }],
      ["lock_vault", undefined],
      ["save_recovery_file", undefined],
      ["delete_source_document", { documentId: "document-1" }],
      ["import_source_document", undefined],
      ["list_unassigned_source_documents", undefined],
      ["normalize_source_document", { documentId: "document-1" }],
      ["render_source_document_page", { documentId: "document-1", pageNumber: 2 }],
    ]);
    expect(listenedEvents).toEqual(["vault-locked"]);
  });

  it("maps command failures to safe user-facing copy", () => {
    expect(commandErrorMessage({ code: "vault_locked" })).toBe(
      "Unlock your Vault to continue.",
    );
    expect(commandErrorMessage({ code: "remembered_unlock_unavailable" })).toBe(
      "Remembered unlock is no longer available. Use your Vault password instead.",
    );
    expect(commandErrorMessage({ code: "remember_failed" })).toBe(
      "CanCan couldn’t save remembered unlock in this Mac’s Keychain.",
    );
    expect(
      commandErrorMessage({ code: "statement_password_save_failed" }),
    ).toBe("CanCan couldn’t save that statement password in this Mac’s Keychain.");
    expect(
      commandErrorMessage({ code: "statement_password_remove_failed" }),
    ).toBe(
      "CanCan couldn’t remove that statement password from this Mac’s Keychain.",
    );
    expect(commandErrorMessage({ code: "recovery_status_failed" })).toBe(
      "The recovery file was saved, but CanCan couldn’t record setup. Keep the file private and try again.",
    );
    expect(commandErrorMessage('{"code":"normalizer_failed"}')).toBe(
      "CanCan could not finish the secure document check. Try again.",
    );
    expect(commandErrorMessage('{"code":"document_unavailable"}')).toBe(
      "This file is no longer available.",
    );
    expect(commandErrorMessage('{"code":"invalid_document_request"}')).toBe(
      "That document request isn’t valid.",
    );
    expect(commandErrorMessage('{"code":"viewer_unsupported"}')).toBe(
      "Preview isn’t available for this evidence.",
    );
    expect(commandErrorMessage('{"code":"document_render_failed"}')).toBe(
      "CanCan couldn’t render that PDF page.",
    );
    expect(commandErrorMessage('{"code":"delete_source_failed"}')).toBe(
      "CanCan couldn’t finish removing this Vault file. Refresh its status before trying again.",
    );
    expect(commandErrorMessage("private backend detail")).toBe(
      "Couldn’t complete that request. Try again.",
    );
  });
});
