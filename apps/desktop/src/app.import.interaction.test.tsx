// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type {
  MoneySourceSummary,
  RenderedDocumentPage,
  SavedStatementPasswordResult,
  SourceDocumentImportOutcome,
  SourceDocumentPreview,
  SourceDocumentSummary,
  VaultAccessStatus,
  VaultStatus,
} from "./command-contracts";
import {
  availableDocument,
  bodyDialog,
  button,
  buttons,
  click,
  clickDialogButton,
  container,
  createApi,
  deferred,
  dialogButton,
  dialogInput,
  enterDialogInput,
  enterPassword,
  installAppHarness,
  moneySource,
  mount,
  navItem,
  noBodyDialog,
  otherMoneySource,
  settle,
  sourceDocument,
} from "./test-support/app-harness";

installAppHarness();
describe("App manual import orchestration", () => {
  it("unlocks the Vault and loads unassigned documents through the injected API", async () => {
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      unlockVault: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
      vaultAccessStatus: vi.fn(async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "locked",
      })),
    });

    await mount(api);
    expect(
      container.querySelector<HTMLInputElement>("#vault-password")?.autocomplete,
    ).toBe("off");
    expect(button("Unlock Vault").disabled).toBe(false);

    await enterPassword("vault-password");
    await click("Unlock Vault");

    expect(api.unlockVault).toHaveBeenCalledWith("vault-password");
    expect(api.createVault).not.toHaveBeenCalled();

    await act(async () => {
      navItem("Sources").click();
      await settle();
    });
    expect(container.textContent).toContain(availableDocument.originalFilename);
    expect(button("Add file").disabled).toBe(false);
  });

  it("loads routed documents only for the selected Money Source", async () => {
    const listSourceDocuments = vi.fn(async (moneySourceId: string) => (
      moneySourceId === moneySource.moneySourceId ? [availableDocument] : []
    ));
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource, otherMoneySource]),
      listSourceDocuments,
    });

    await mount(api, "sources");

    expect(container.textContent).toContain(moneySource.displayName);
    expect(container.textContent).toContain(otherMoneySource.displayName);
    expect(listSourceDocuments).not.toHaveBeenCalled();

    const selectSource = container.querySelector<HTMLButtonElement>(
      `[aria-label="View documents for ${moneySource.displayName}"]`,
    );
    expect(selectSource).not.toBeNull();
    await act(async () => {
      selectSource!.click();
      await settle();
    });

    expect(listSourceDocuments).toHaveBeenCalledTimes(1);
    expect(listSourceDocuments).toHaveBeenCalledWith(moneySource.moneySourceId);
    expect(listSourceDocuments).not.toHaveBeenCalledWith(
      otherMoneySource.moneySourceId,
    );
    expect(container.textContent).toContain(availableDocument.originalFilename);
  });

  it("queues a parser re-run for ready routed evidence and guards processing evidence", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceDocuments: vi.fn(async () => [
        sourceDocument({ documentId: "routed" }),
        sourceDocument({ documentId: "processing", documentStatus: "processing" }),
      ]),
    });

    await mount(api, "sources");
    await act(async () => {
      container.querySelector<HTMLButtonElement>(
        `[aria-label="View documents for ${moneySource.displayName}"]`,
      )!.click();
      await settle();
    });

    expect(button("Parser unavailable").disabled).toBe(true);
    await click("Re-run parser");
    expect(api.reparseSourceDocument).toHaveBeenCalledWith("routed");
  });

  it("does not leave a failed Money Source selection in a loading state", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceDocuments: vi.fn(async () => {
        throw new Error("source documents unavailable");
      }),
    });

    await mount(api, "sources");
    const selectSource = container.querySelector<HTMLButtonElement>(
      `[aria-label="View documents for ${moneySource.displayName}"]`,
    );
    expect(selectSource).not.toBeNull();
    await act(async () => {
      selectSource!.click();
      await settle();
    });

    expect(container.textContent).toContain("Something needs your attention");
    expect(container.textContent).not.toContain("Loading documents…");
    expect(selectSource!.disabled).toBe(false);
  });

  it("clears a failed Money Source selection error when another selection succeeds", async () => {
    const listSourceDocuments = vi
      .fn<(moneySourceId: string) => Promise<SourceDocumentSummary[]>>()
      .mockRejectedValueOnce(new Error("source documents unavailable"))
      .mockResolvedValueOnce([availableDocument]);
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource, otherMoneySource]),
      listSourceDocuments,
    });

    await mount(api, "sources");
    await act(async () => {
      container.querySelector<HTMLButtonElement>(
        `[aria-label="View documents for ${moneySource.displayName}"]`,
      )!.click();
      await settle();
    });
    expect(container.textContent).toContain("Something needs your attention");

    await act(async () => {
      container.querySelector<HTMLButtonElement>(
        `[aria-label="View documents for ${otherMoneySource.displayName}"]`,
      )!.click();
      await settle();
    });

    expect(container.textContent).not.toContain("Something needs your attention");
    expect(container.textContent).toContain(availableDocument.originalFilename);
  });

  it("creates a Vault when setup is needed", async () => {
    const api = createApi({
      vaultAccessStatus: vi.fn(async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "not_created",
      })),
    });

    await mount(api);
    await enterPassword("new-vault-password");
    await click("Create Vault");

    expect(api.createVault).toHaveBeenCalledWith("new-vault-password");
    expect(api.unlockVault).not.toHaveBeenCalled();

    await act(async () => {
      navItem("Sources").click();
      await settle();
    });
    expect(button("Add file")).toBeDefined();
  });

  it("keeps recovery optional and removes its task only after a successful save", async () => {
    const api = createApi({
      saveRecoveryFile: vi.fn(async (): Promise<boolean> => true),
    });

    await mount(api, "sources");
    expect(container.textContent).toContain("Save your recovery file");

    await click("Save recovery file");

    expect(api.saveRecoveryFile).toHaveBeenCalledTimes(1);
    expect(container.textContent).not.toContain("Save your recovery file");
    expect(container.textContent).toContain("Recovery file saved");
  });

  it("keeps the recovery task after the save picker is cancelled", async () => {
    const api = createApi();

    await mount(api, "sources");
    await click("Save recovery file");

    expect(container.textContent).toContain("Save your recovery file");
    expect(container.textContent).toContain("Recovery is not configured");
  });

  it("unlocks through Keychain only after an explicit user action and clears the unused password", async () => {
    const api = createApi({
      vaultAccessStatus: vi.fn(async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: true,
        status: "locked",
      })),
    });

    await mount(api);
    expect(container.textContent).toContain("Unlock with Touch ID");
    expect(api.unlockVaultWithKeychain).not.toHaveBeenCalled();

    await enterPassword("typed-but-unused");
    await click("Unlock with Touch ID");

    expect(api.unlockVaultWithKeychain).toHaveBeenCalledTimes(1);
    expect(container.textContent).not.toContain("Unlock with Touch ID");

    await click("Lock Vault");
    expect(container.querySelector<HTMLInputElement>("#vault-password")?.value).toBe("");
  });

  it("keeps password unlock available when Touch ID presence is unknown", async () => {
    const api = createApi({
      vaultAccessStatus: vi.fn(async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: null,
        status: "locked",
      })),
    });

    await mount(api);

    expect(container.textContent).toContain("Touch ID unlock is unavailable");
    expect(container.textContent).toContain("Vault password");
    expect(container.textContent).not.toContain("Unlock with Touch ID");
    expect(api.unlockVaultWithKeychain).not.toHaveBeenCalled();
  });

  it("states the live-Vault tradeoff plainly on the unlock gate", async () => {
    const api = createApi({
      vaultAccessStatus: vi.fn(async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "locked",
      })),
    });

    await mount(api);

    expect(container.textContent).toContain(
      "background intake keeps working and your Mac’s login session protects the live Vault",
    );
    expect(container.textContent).toContain("locks only when you lock it or quit");
  });

  it("enables and removes Touch ID unlock without sending the key to the renderer", async () => {
    const api = createApi();

    await mount(api, "sources");
    const checkbox = container.querySelector<HTMLInputElement>(
      'input[type="checkbox"]',
    );
    expect(checkbox?.checked).toBe(false);

    await act(async () => {
      checkbox!.click();
      await settle();
    });
    expect(api.rememberVaultOnThisMac).toHaveBeenCalledWith();
    expect(checkbox?.checked).toBe(true);
    expect(container.textContent).toContain("Touch ID unlock enabled");

    await act(async () => {
      checkbox!.click();
      await settle();
    });
    expect(api.forgetVaultOnThisMac).toHaveBeenCalledWith();
    expect(checkbox?.checked).toBe(false);
    expect(container.textContent).toContain("Touch ID unlock removed");
  });

  it("keeps the Vault open and the opt-in off when Touch ID storage fails", async () => {
    const api = createApi({
      rememberVaultOnThisMac: vi.fn(async () => {
        throw { code: "remember_failed", privateDetail: "Keychain platform detail" };
      }),
    });

    await mount(api, "sources");
    const checkbox = container.querySelector<HTMLInputElement>(
      'input[type="checkbox"]',
    );
    await act(async () => {
      checkbox!.click();
      await settle();
    });

    expect(checkbox?.checked).toBe(false);
    expect(container.textContent).toContain(
      "CanCan couldn’t enable Touch ID unlock.",
    );
    expect(container.textContent).not.toContain("Keychain platform detail");
    expect(container.textContent).toContain("Add file");
  });

  it("removes a stale Touch ID unlock action after Touch ID unlock fails", async () => {
    const vaultAccessStatus = vi
      .fn<() => Promise<VaultAccessStatus>>()
      .mockResolvedValueOnce({ recoveryConfigured: false, rememberedOnThisMac: true, status: "locked" })
      .mockResolvedValueOnce({ recoveryConfigured: false, rememberedOnThisMac: false, status: "locked" });
    const api = createApi({
      unlockVaultWithKeychain: vi.fn(async (): Promise<VaultStatus> => {
        throw { code: "remembered_unlock_unavailable" };
      }),
      vaultAccessStatus,
    });

    await mount(api);
    await click("Unlock with Touch ID");

    expect(container.textContent).not.toContain("Unlock with Touch ID");
    expect(container.textContent).toContain(
      "Touch ID unlock is no longer available.",
    );
    expect(container.textContent).toContain("Vault password");
  });

  it("reports cancellation and every safe import result while only the picker shows its busy label", async () => {
    const picker = deferred<SourceDocumentImportOutcome | null>();
    const importOutcomes: Array<Promise<SourceDocumentImportOutcome | null>> = [
      picker.promise,
      Promise.resolve({ documentId: "document-1", status: "imported" }),
      Promise.resolve({ documentId: "document-1", status: "already_present" }),
      Promise.resolve({ documentId: "document-1", status: "restored" }),
    ];
    const api = createApi({
      importSourceDocument: vi.fn(() => importOutcomes.shift()!),
    });

    await mount(api, "sources");
    await click("Add file");

    expect(api.importSourceDocument).toHaveBeenCalledTimes(1);
    expect(button("Opening picker…").disabled).toBe(true);

    await act(async () => {
      picker.resolve(null);
      await settle();
    });
    expect(container.textContent).toContain("No file was imported");

    await click("Add file");
    expect(container.textContent).toContain("Added to your Vault");
    await click("Add file");
    expect(container.textContent).toContain("Already in CanCan");
    await click("Add file");
    expect(container.textContent).toContain("Evidence restored");
  });

  it("tries a source-scoped saved password before offering use-once or verified replacement", async () => {
    let unlocked = false;
    const protectedDocument = sourceDocument({
      attentionReason: "password_required",
      documentStatus: "needs_attention",
    });
    const unlockSourceDocument = vi.fn(
      async (
        _documentId: string,
        _moneySourceId: string,
        _password: string,
        _updateSavedPassword: boolean,
      ): Promise<void> => {
        unlocked = true;
      },
    );
    unlockSourceDocument.mockRejectedValueOnce({
      code: "statement_password_invalid",
    });
    const api = createApi({
      listStatementPasswordSources: vi.fn(async () => [{
        displayName: "DBS",
        hasSavedPassword: true,
        moneySourceId: "source-dbs",
      }]),
      listUnassignedSourceDocuments: vi.fn(async () => [
        unlocked ? sourceDocument() : protectedDocument,
      ]),
      trySavedStatementPassword: vi.fn(async () => "invalid" as const),
      unlockSourceDocument,
    });

    await mount(api, "sources");
    expect(container.textContent).toContain("Needs attention");
    expect(container.textContent).not.toContain("Re-run parser");
    expect(container.textContent).toContain("Delete source file");
    await click("Unlock");

    expect(api.trySavedStatementPassword).toHaveBeenCalledWith(
      "document-1",
      "source-dbs",
    );
    expect(bodyDialog().textContent).toContain(
      "The saved password did not work.",
    );
    expect(
      bodyDialog().querySelector<HTMLButtonElement>(
        'button[aria-label="Money Source for this statement"]',
      )?.textContent,
    ).toContain("DBS");

    await enterDialogInput("#statement-password", "wrong-password");
    await clickDialogButton("Use once");
    expect(unlockSourceDocument).toHaveBeenLastCalledWith(
      "document-1",
      "source-dbs",
      "wrong-password",
      false,
    );
    expect(bodyDialog().textContent).toContain(
      "That password did not unlock this statement.",
    );
    expect(dialogInput("#statement-password").value).toBe("");

    await enterDialogInput("#statement-password", "current-password");
    await clickDialogButton("Update saved password");
    expect(unlockSourceDocument).toHaveBeenLastCalledWith(
      "document-1",
      "source-dbs",
      "current-password",
      true,
    );
    noBodyDialog();
    expect(container.textContent).toContain("Statement unlocked");
    expect(container.textContent).toContain("View document");
    expect(container.textContent).not.toContain("Parser remains unavailable");
    await click("Re-run parser");
    expect(api.reparseSourceDocument).toHaveBeenCalledWith("document-1");
  });

  it("reports a saved-password unlock as routeable for the Vault session", async () => {
    let unlocked = false;
    const api = createApi({
      listStatementPasswordSources: vi.fn(async () => [{
        displayName: "DBS",
        hasSavedPassword: true,
        moneySourceId: "source-dbs",
      }]),
      listUnassignedSourceDocuments: vi.fn(async () => [
        sourceDocument({
          attentionReason: unlocked ? null : "password_required",
          documentStatus: unlocked ? "ready" : "needs_attention",
        }),
      ]),
      trySavedStatementPassword: vi.fn(async () => {
        unlocked = true;
        return "unlocked" as const;
      }),
    });

    await mount(api, "sources");
    await click("Unlock");

    expect(container.textContent).toContain(
      "This statement is ready to view and route for this Vault session.",
    );
    expect(container.textContent).not.toContain("Parser remains unavailable");
    await click("Re-run parser");
    expect(api.reparseSourceDocument).toHaveBeenCalledWith("document-1");
  });

  it("maps technical document statuses to the canonical primary row states", async () => {
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [
        sourceDocument({
          documentId: "protected",
          documentStatus: "ready",
          originalFilename: "Protected.pdf",
        }),
        sourceDocument({
          documentId: "inspection-failed",
          attentionReason: "inspection_failed",
          documentStatus: "needs_attention",
          originalFilename: "Corrupt.pdf",
        }),
        sourceDocument({
          documentId: "unavailable",
          documentStatus: "missing",
          fileState: "missing",
          originalFilename: "Unavailable.pdf",
        }),
      ]),
    });

    await mount(api, "sources");

    const rowFor = (filename: string) => [...container.querySelectorAll("li")]
      .find((row) => row.textContent?.includes(filename));
    expect(rowFor("Protected.pdf")?.textContent).toContain("Ready");
    expect(rowFor("Corrupt.pdf")?.textContent).toContain("Needs attention");
    expect(rowFor("Unavailable.pdf")?.textContent).toContain("Missing");
  });

  it("distinguishes a missing device-local saved password from an invalid one", async () => {
    const api = createApi({
      listStatementPasswordSources: vi.fn(async () => [{
        displayName: "DBS",
        hasSavedPassword: true,
        moneySourceId: "source-dbs",
      }]),
      listUnassignedSourceDocuments: vi.fn(async () => [
        sourceDocument({
          attentionReason: "password_required",
          documentStatus: "needs_attention",
        }),
      ]),
      trySavedStatementPassword: vi.fn(async () => "unavailable" as const),
    });

    await mount(api, "sources");
    await click("Unlock");

    expect(bodyDialog().textContent).toContain(
      "The saved password is not available on this Mac.",
    );
    expect(bodyDialog().textContent).not.toContain(
      "The saved password did not work.",
    );
  });

  it("distinguishes a Money Source load failure from an empty configured-source list", async () => {
    const listSources = vi.fn()
      .mockRejectedValueOnce({ code: "list_sources_failed" })
      .mockResolvedValueOnce([{
        displayName: "DBS",
        hasSavedPassword: false,
        moneySourceId: "source-dbs",
      }]);
    const api = createApi({
      listStatementPasswordSources: listSources,
      listUnassignedSourceDocuments: vi.fn(async () => [
        sourceDocument({
          attentionReason: "password_required",
          documentStatus: "needs_attention",
        }),
      ]),
    });

    await mount(api, "sources");
    await click("Unlock");
    expect(bodyDialog().textContent).toContain("Money Sources couldn’t be loaded");
    expect(bodyDialog().textContent).not.toContain("No Money Source is configured");

    await clickDialogButton("Try again");
    expect(listSources).toHaveBeenCalledTimes(2);
    expect(
      bodyDialog().querySelector('button[aria-label="Money Source for this statement"]'),
    ).not.toBeNull();
  });

  it("does not restore protected-document UI after a native Vault lock", async () => {
    const savedPassword = deferred<SavedStatementPasswordResult>();
    let notifyLocked: (() => void) | undefined;
    const api = createApi({
      listStatementPasswordSources: vi.fn(async () => [{
        displayName: "DBS",
        hasSavedPassword: true,
        moneySourceId: "source-dbs",
      }]),
      listUnassignedSourceDocuments: vi.fn(async () => [
        sourceDocument({
          attentionReason: "password_required",
          documentStatus: "needs_attention",
        }),
      ]),
      onVaultLocked: vi.fn(async (handler) => {
        notifyLocked = handler;
        return () => undefined;
      }),
      trySavedStatementPassword: vi.fn(() => savedPassword.promise),
    });

    await mount(api, "sources");
    await click("Unlock");
    expect(api.trySavedStatementPassword).toHaveBeenCalledTimes(1);

    await act(async () => {
      notifyLocked?.();
      savedPassword.resolve("unlocked");
      await settle();
    });

    noBodyDialog();
    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain("June statement.pdf");
    expect(container.textContent).not.toContain("Statement unlocked");
  });

  it("queues a parser re-run only for available evidence", async () => {
    const reparse = deferred<void>();
    let documents = [
      availableDocument,
      sourceDocument({
        documentId: "deleted",
        documentStatus: "file_deleted",
        fileState: "deleted",
      }),
      sourceDocument({
        documentId: "missing",
        documentStatus: "missing",
        fileState: "missing",
      }),
    ];
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => documents),
      reparseSourceDocument: vi.fn(() => reparse.promise),
    });

    await mount(api, "sources");
    const unavailable = buttons("Parser unavailable");
    expect(unavailable).toHaveLength(2);
    expect(unavailable.every((element) => element.disabled)).toBe(true);
    await act(async () => {
      unavailable.forEach((element) => element.click());
      await settle();
    });
    expect(api.reparseSourceDocument).not.toHaveBeenCalled();

    await click("Re-run parser");
    expect(api.reparseSourceDocument).toHaveBeenCalledWith("document-1");
    expect(button("Re-running…").disabled).toBe(true);
    expect(button("Add file").textContent).toBe("Add file");
    expect(button("Add file").disabled).toBe(true);

    await act(async () => {
      documents = [sourceDocument({ documentStatus: "processing" })];
      reparse.resolve();
      await settle();
      await settle();
    });
    expect(container.textContent).toContain("Parser re-run started");
    expect(container.textContent).toContain("Processing");
  });

  it("does not let an older same-session refresh restore stale unassigned evidence", async () => {
    const staleUnassigned = deferred<SourceDocumentSummary[]>();
    const reparse = deferred<void>();
    const listUnassignedSourceDocuments = vi
      .fn<() => Promise<SourceDocumentSummary[]>>()
      .mockResolvedValueOnce([availableDocument])
      .mockImplementationOnce(() => staleUnassigned.promise)
      .mockResolvedValueOnce([]);
    const api = createApi({
      listUnassignedSourceDocuments,
      reparseSourceDocument: vi.fn(() => reparse.promise),
    });

    await mount(api, "sources");
    await click("Re-run parser");
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      await settle();
    });
    await act(async () => {
      reparse.resolve();
      await settle();
    });

    expect(container.textContent).toContain("No evidence needs your attention.");
    expect(buttons("Re-run parser")).toHaveLength(0);

    await act(async () => {
      staleUnassigned.resolve([availableDocument]);
      await settle();
    });

    expect(container.textContent).not.toContain(availableDocument.originalFilename);
    expect(container.textContent).toContain("No evidence needs your attention.");
    expect(buttons("Re-run parser")).toHaveLength(0);
  });

  it("does not restore source or document names after refresh confirms the Vault is locked", async () => {
    const staleSources = deferred<MoneySourceSummary[]>();
    const staleUnassigned = deferred<SourceDocumentSummary[]>();
    const vaultAccessStatus = vi
      .fn<() => Promise<VaultAccessStatus>>()
      .mockResolvedValueOnce({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "unlocked",
      })
      .mockResolvedValueOnce({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "locked",
      });
    const api = createApi({
      listMoneySources: vi.fn(() => staleSources.promise),
      listUnassignedSourceDocuments: vi.fn(() => staleUnassigned.promise),
      vaultAccessStatus,
    });

    await mount(api, "sources");
    expect(api.listMoneySources).toHaveBeenCalledTimes(1);
    expect(api.listUnassignedSourceDocuments).toHaveBeenCalledTimes(1);

    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      await settle();
    });
    expect(container.textContent).toContain("Unlock your Vault");

    await act(async () => {
      staleSources.resolve([moneySource]);
      staleUnassigned.resolve([availableDocument]);
      await settle();
    });

    expect(container.textContent).not.toContain(moneySource.displayName);
    expect(container.textContent).not.toContain(availableDocument.originalFilename);
  });

  it("opens rendered PDF pages, navigates them, and clears pixels on close or Vault lock", async () => {
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
    });

    await mount(api, "sources");
    const viewTrigger = button("View document");
    await click("View document");

    expect(api.renderSourceDocumentPage).toHaveBeenLastCalledWith("document-1", 1);
    const dialog = bodyDialog();
    expect(dialog.querySelector("img")?.getAttribute("src")).toBe(
      "data:image/png;base64,cmVuZGVyZWQtcGFnZQ==",
    );
    expect(dialog.textContent).toContain("Page 1 of 2");
    expect(container.querySelector("main > section")?.hasAttribute("inert")).toBe(true);
    expect(container.querySelector("main > aside")?.hasAttribute("inert")).toBe(true);

    const close = dialogButton("Close");
    const next = dialogButton("Next");
    expect(dialog.contains(document.activeElement)).toBe(true);
    close.focus();
    await act(async () => {
      close.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "Tab" }));
      await settle();
    });
    expect(document.activeElement).toBe(next);
    await act(async () => {
      next.dispatchEvent(
        new KeyboardEvent("keydown", { bubbles: true, key: "Tab", shiftKey: true }),
      );
      await settle();
    });
    expect(document.activeElement).toBe(close);

    await clickDialogButton("Next");
    expect(api.renderSourceDocumentPage).toHaveBeenLastCalledWith("document-1", 2);
    expect(bodyDialog().textContent).toContain("Page 2 of 2");

    await clickDialogButton("Close");
    noBodyDialog();
    expect(document.body.querySelector("img")).toBeNull();
    expect(document.activeElement).toBe(viewTrigger);
    expect(api.closeSourceDocumentView).toHaveBeenCalledWith("document-1");

    await click("View document");
    await click("Lock Vault");
    noBodyDialog();
    expect(document.body.querySelector("img")).toBeNull();
    expect(container.textContent).toContain("Unlock your Vault");
  });

  it("opens an image in the rendered viewer", async () => {
    const imageDocument = sourceDocument({
      documentId: "document-image",
      mimeType: "image/png",
      originalFilename: "phone-statement.png",
    });
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [imageDocument]),
      renderSourceDocumentPage: vi.fn(
        async (): Promise<RenderedDocumentPage> => ({
          pageCount: 1,
          pageNumber: 1,
          pngBase64: "cmVuZGVyZWQtaW1hZ2U=",
        }),
      ),
    });

    await mount(api, "sources");
    expect(container.textContent).toContain("PNG");
    await click("View document");

    expect(api.renderSourceDocumentPage).toHaveBeenCalledWith("document-image", 1);
    expect(api.previewSourceDocument).not.toHaveBeenCalled();
    expect(bodyDialog().querySelector("img")?.getAttribute("src")).toBe(
      "data:image/png;base64,cmVuZGVyZWQtaW1hZ2U=",
    );
    expect(bodyDialog().textContent).toContain("Page 1 of 1");
  });

  it("opens a bounded CSV preview and clears it on close or Vault lock", async () => {
    const csvDocument = sourceDocument({
      documentId: "document-csv",
      mimeType: "text/csv",
      originalFilename: "wise-export.csv",
    });
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [csvDocument]),
      previewSourceDocument: vi.fn(
        async (): Promise<SourceDocumentPreview> => ({
          lineCount: 1,
          previewLines: 1,
          previewText: "memo,partial content",
          truncated: true,
        }),
      ),
    });

    await mount(api, "sources");
    const viewTrigger = button("View document");
    await click("View document");

    expect(api.previewSourceDocument).toHaveBeenCalledWith("document-csv");
    expect(api.renderSourceDocumentPage).not.toHaveBeenCalled();
    expect(bodyDialog().textContent).toContain("memo,partial content");
    expect(bodyDialog().textContent).toContain(
      "Preview truncated. Displaying content from 1 of 1 line; the final displayed line may be partial. Save a copy to view the full file.",
    );

    await clickDialogButton("Close");
    noBodyDialog();
    expect(document.activeElement).toBe(viewTrigger);
    expect(api.closeSourceDocumentView).toHaveBeenCalledWith("document-csv");

    await click("View document");
    await click("Lock Vault");
    noBodyDialog();
    expect(document.body.textContent).not.toContain("memo,partial content");
    expect(container.textContent).toContain("Unlock your Vault");
  });

  it("ignores a delayed CSV preview completion after native Vault lock", async () => {
    const csvDocument = sourceDocument({
      documentId: "document-csv",
      mimeType: "text/csv",
      originalFilename: "wise-export.csv",
    });
    const preview = deferred<SourceDocumentPreview>();
    let notifyLocked: (() => void) | undefined;
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [csvDocument]),
      onVaultLocked: vi.fn(async (handler) => {
        notifyLocked = handler;
        return () => undefined;
      }),
      previewSourceDocument: vi.fn(() => preview.promise),
    });

    await mount(api, "sources");
    await click("View document");
    expect(api.previewSourceDocument).toHaveBeenCalledWith("document-csv");

    await act(async () => {
      notifyLocked?.();
      preview.resolve({
        lineCount: 2,
        previewLines: 2,
        previewText: "date,amount\n2026-07-01,10.00",
        truncated: false,
      });
      await settle();
    });

    noBodyDialog();
    expect(container.textContent).toContain("Unlock your Vault");
    expect(document.body.textContent).not.toContain("date,amount");
  });

  it("does not surface a preview failure that lands after native Vault lock", async () => {
    const csvDocument = sourceDocument({
      documentId: "document-csv",
      mimeType: "text/csv",
      originalFilename: "wise-export.csv",
    });
    const preview = deferred<SourceDocumentPreview>();
    let notifyLocked: (() => void) | undefined;
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [csvDocument]),
      onVaultLocked: vi.fn(async (handler) => {
        notifyLocked = handler;
        return () => undefined;
      }),
      previewSourceDocument: vi.fn(() => preview.promise),
    });

    await mount(api, "sources");
    await click("View document");
    expect(api.previewSourceDocument).toHaveBeenCalledWith("document-csv");

    await act(async () => {
      notifyLocked?.();
      preview.reject({ code: "vault_locked" });
      await settle();
    });

    noBodyDialog();
    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain("Unlock your Vault to continue.");
  });

  it("deletes an available source file and keeps its tombstone visible", async () => {
    const deletedDocument = sourceDocument({
      documentStatus: "file_deleted",
      fileState: "deleted",
    });
    const listUnassignedSourceDocuments = vi
      .fn<() => Promise<SourceDocumentSummary[]>>()
      .mockResolvedValueOnce([availableDocument])
      .mockResolvedValueOnce([deletedDocument]);
    const api = createApi({ listUnassignedSourceDocuments });

    await mount(api, "sources");
    await click("Delete source file");

    expect(api.deleteSourceDocument).not.toHaveBeenCalled();
    expect(bodyDialog().textContent).toContain(
      `Delete the current Vault copy of ${availableDocument.originalFilename}?`,
    );
    expect(bodyDialog().textContent).toContain(
      "The document registry entry, record history, audit trail, and ledger links remain.",
    );

    await clickDialogButton("Delete source file");

    expect(api.deleteSourceDocument).toHaveBeenCalledWith(
      availableDocument.documentId,
    );
    noBodyDialog();
    expect(container.textContent).toContain("Source file deleted");
    expect(container.textContent).toContain(availableDocument.originalFilename);
    expect(container.textContent).toContain("File deleted");
    expect(container.textContent).not.toContain("Delete source file");
    expect(container.textContent).toContain("View unavailable");
    expect(container.textContent).toContain("Parser unavailable");
  });

  it("reports a saved source copy but keeps cancellation silent", async () => {
    const outcomes = [false, true];
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      saveSourceDocumentCopy: vi.fn(async () => outcomes.shift()!),
    });

    await mount(api, "sources");
    await click("Save a copy");
    expect(container.textContent).not.toContain("Copy saved");

    await click("Save a copy");
    expect(api.saveSourceDocumentCopy).toHaveBeenNthCalledWith(
      2,
      availableDocument.documentId,
    );
    expect(container.textContent).toContain("Copy saved");
    expect(container.textContent).toContain(
      "outside CanCan’s encrypted Vault",
    );
  });

  it("ignores a delayed source-copy completion after native Vault lock", async () => {
    const copy = deferred<boolean>();
    let notifyLocked: (() => void) | undefined;
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      onVaultLocked: vi.fn(async (handler) => {
        notifyLocked = handler;
        return () => undefined;
      }),
      saveSourceDocumentCopy: vi.fn(() => copy.promise),
    });

    await mount(api, "sources");
    await click("Save a copy");
    expect(container.textContent).toContain("Saving copy…");

    await act(async () => {
      notifyLocked?.();
      copy.resolve(true);
      await settle();
    });

    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain("Copy saved");
    expect(container.textContent).not.toContain("Saving copy…");
    expect(container.textContent).not.toContain(availableDocument.originalFilename);
  });

  it("keeps an available source file when deletion confirmation is cancelled", async () => {
    const listUnassignedSourceDocuments = vi.fn(async () => [availableDocument]);
    const api = createApi({ listUnassignedSourceDocuments });

    await mount(api, "sources");
    await click("Delete source file");
    await clickDialogButton("Cancel");

    expect(api.deleteSourceDocument).not.toHaveBeenCalled();
    noBodyDialog();
    expect(container.textContent).toContain(availableDocument.originalFilename);
    expect(container.textContent).toContain("Delete source file");
    expect(container.textContent).not.toContain("Source file deleted");
    expect(listUnassignedSourceDocuments).toHaveBeenCalledTimes(1);
  });

  it("shows the deleted tombstone when storage removal fails after the decision commits", async () => {
    const deletedDocument = sourceDocument({
      documentStatus: "file_deleted",
      fileState: "deleted",
    });
    const listUnassignedSourceDocuments = vi
      .fn<() => Promise<SourceDocumentSummary[]>>()
      .mockResolvedValueOnce([availableDocument])
      .mockResolvedValueOnce([deletedDocument]);
    const api = createApi({
      deleteSourceDocument: vi.fn(async () => {
        throw { code: "delete_source_failed" };
      }),
      listUnassignedSourceDocuments,
    });

    await mount(api, "sources");
    await click("Delete source file");
    await clickDialogButton("Delete source file");

    noBodyDialog();
    expect(container.textContent).toContain("File deleted");
    expect(container.textContent).toContain(
      "CanCan couldn’t finish removing this Vault file.",
    );
    expect(container.textContent).not.toContain("Delete source file");
    expect(listUnassignedSourceDocuments).toHaveBeenCalledTimes(2);
  });
});
