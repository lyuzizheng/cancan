// @vitest-environment happy-dom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  SourceDocumentImportOutcome,
  RenderedDocumentPage,
  SavedStatementPasswordResult,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
  VaultAccessStatus,
  VaultStatus,
} from "./command-contracts";
import { App } from "./app";
import type { VaultApi } from "./vault-api";

(
  globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLDivElement;
let root: Root;

const availableDocument = sourceDocument();

beforeEach(() => {
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => {
    root.unmount();
  });
  vi.clearAllTimers();
  vi.useRealTimers();
  container.remove();
});

function sourceDocument(
  overrides: Partial<SourceDocumentSummary> = {},
): SourceDocumentSummary {
  return {
    byteSize: 42,
    documentId: "document-1",
    documentStatus: "ready",
    fileState: "available",
    mimeType: "application/pdf",
    originalFilename: "June statement.pdf",
    receivedAt: "2026-07-19T00:00:00Z",
    ...overrides,
  };
}

function createApi(overrides: Partial<VaultApi> = {}) {
  const api = {
    createVault: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
    deleteSourceDocument: vi.fn(async (): Promise<boolean> => true),
    forgetVaultOnThisMac: vi.fn(async (): Promise<void> => undefined),
    importSourceDocument: vi.fn(
      async (): Promise<SourceDocumentImportOutcome | null> => null,
    ),
    listStatementPasswordSources: vi.fn(async () => []),
    listUnassignedSourceDocuments: vi.fn(
      async (): Promise<SourceDocumentSummary[]> => [],
    ),
    lockVault: vi.fn(async (): Promise<VaultStatus> => "locked"),
    normalizeSourceDocument: vi.fn(
      async (documentId: string): Promise<SourceDocumentRoutingOutcome> => ({
        accountIds: [],
        documentId,
        moneySourceId: null,
        reason: "classification_uncertain",
        status: "needs_attention",
      }),
    ),
    onVaultLocked: vi.fn(async () => () => undefined),
    renderSourceDocumentPage: vi.fn(
      async (_documentId: string, pageNumber: number): Promise<RenderedDocumentPage> => ({
        pageCount: 2,
        pageNumber,
        pngBase64: "cmVuZGVyZWQtcGFnZQ==",
      }),
    ),
    rememberVaultOnThisMac: vi.fn(async (): Promise<void> => undefined),
    saveRecoveryFile: vi.fn(async (): Promise<boolean> => false),
    trySavedStatementPassword: vi.fn(
      async (): Promise<SavedStatementPasswordResult> => "invalid",
    ),
    unlockSourceDocument: vi.fn(async (): Promise<void> => undefined),
    unlockVault: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
    unlockVaultWithKeychain: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
    vaultAccessStatus: vi.fn(
      async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "unlocked",
      }),
    ),
    vaultStatus: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
  };

  return Object.assign(api, overrides) as VaultApi & typeof api;
}

function deferred<Value>() {
  let resolve: (value: Value) => void;
  let reject: (reason?: unknown) => void;
  const promise = new Promise<Value>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });

  return { promise, reject: reject!, resolve: resolve! };
}

async function settle() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

async function mount(api: VaultApi) {
  await act(async () => {
    root.render(<App api={api} />);
    await settle();
  });
}

function button(label: string): HTMLButtonElement {
  const matches = [...container.querySelectorAll("button")].filter(
    (element) => element.textContent?.trim() === label,
  );
  expect(matches).toHaveLength(1);
  return matches[0]!;
}

function buttons(label: string): HTMLButtonElement[] {
  return [...container.querySelectorAll("button")].filter(
    (element): element is HTMLButtonElement => element.textContent?.trim() === label,
  );
}

async function click(label: string) {
  await act(async () => {
    button(label).click();
    await settle();
  });
}

async function enterPassword(password: string) {
  await enterInput("#vault-password", password);
}

async function enterInput(selector: string, value: string) {
  const input = container.querySelector<HTMLInputElement>(selector);
  expect(input).not.toBeNull();
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  expect(setter).toBeDefined();

  await act(async () => {
    setter!.call(input, value);
    input!.dispatchEvent(new Event("input", { bubbles: true }));
    await settle();
  });
}

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
    expect(container.textContent).toContain(availableDocument.originalFilename);
    expect(button("Add file").disabled).toBe(false);
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
    expect(button("Add file")).toBeDefined();
  });

  it("keeps recovery optional and removes its task only after a successful save", async () => {
    const api = createApi({
      saveRecoveryFile: vi.fn(async (): Promise<boolean> => true),
    });

    await mount(api);
    expect(container.textContent).toContain("Save your recovery file");

    await click("Save recovery file");

    expect(api.saveRecoveryFile).toHaveBeenCalledTimes(1);
    expect(container.textContent).not.toContain("Save your recovery file");
    expect(container.textContent).toContain("Recovery file saved");
  });

  it("keeps the recovery task after the save picker is cancelled", async () => {
    const api = createApi();

    await mount(api);
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
    expect(container.textContent).toContain("Unlock with this Mac");
    expect(api.unlockVaultWithKeychain).not.toHaveBeenCalled();

    await enterPassword("typed-but-unused");
    await click("Unlock with this Mac");

    expect(api.unlockVaultWithKeychain).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("Add file");

    await click("Lock Vault");
    expect(container.querySelector<HTMLInputElement>("#vault-password")?.value).toBe("");
  });

  it("keeps password unlock available when Keychain presence is unknown", async () => {
    const api = createApi({
      vaultAccessStatus: vi.fn(async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: null,
        status: "locked",
      })),
    });

    await mount(api);

    expect(container.textContent).toContain("Keychain unlock is unavailable");
    expect(container.textContent).toContain("Vault password");
    expect(container.textContent).not.toContain("Unlock with this Mac");
    expect(api.unlockVaultWithKeychain).not.toHaveBeenCalled();
  });

  it("enables and removes remembered unlock without sending the key to the renderer", async () => {
    const api = createApi();

    await mount(api);
    const checkbox = container.querySelector<HTMLInputElement>(
      '.remember-vault-control input[type="checkbox"]',
    );
    expect(checkbox?.checked).toBe(false);

    await act(async () => {
      checkbox!.click();
      await settle();
    });
    expect(api.rememberVaultOnThisMac).toHaveBeenCalledWith();
    expect(checkbox?.checked).toBe(true);
    expect(container.textContent).toContain("Remembered unlock enabled");

    await act(async () => {
      checkbox!.click();
      await settle();
    });
    expect(api.forgetVaultOnThisMac).toHaveBeenCalledWith();
    expect(checkbox?.checked).toBe(false);
    expect(container.textContent).toContain("Remembered unlock removed");
  });

  it("keeps the Vault open and the opt-in off when Keychain storage fails", async () => {
    const api = createApi({
      rememberVaultOnThisMac: vi.fn(async () => {
        throw { code: "remember_failed", privateDetail: "Keychain platform detail" };
      }),
    });

    await mount(api);
    const checkbox = container.querySelector<HTMLInputElement>(
      '.remember-vault-control input[type="checkbox"]',
    );
    await act(async () => {
      checkbox!.click();
      await settle();
    });

    expect(checkbox?.checked).toBe(false);
    expect(container.textContent).toContain(
      "CanCan couldn’t save remembered unlock in this Mac’s Keychain.",
    );
    expect(container.textContent).not.toContain("Keychain platform detail");
    expect(container.textContent).toContain("Add file");
  });

  it("removes a stale remembered-unlock action after Keychain unlock fails", async () => {
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
    await click("Unlock with this Mac");

    expect(container.textContent).not.toContain("Unlock with this Mac");
    expect(container.textContent).toContain(
      "Remembered unlock is no longer available.",
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

    await mount(api);
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
    const protectedDocument = sourceDocument({ documentStatus: "password_required" });
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
        unlocked ? sourceDocument({ documentStatus: "protected_unlocked" }) : protectedDocument,
      ]),
      trySavedStatementPassword: vi.fn(async () => "invalid" as const),
      unlockSourceDocument,
    });

    await mount(api);
    expect(container.textContent).toContain("Needs attention");
    expect(container.textContent).not.toContain("Check routing");
    expect(container.textContent).toContain("Delete source file");
    await click("Unlock");

    expect(api.trySavedStatementPassword).toHaveBeenCalledWith(
      "document-1",
      "source-dbs",
    );
    expect(container.textContent).toContain(
      "The saved password did not work.",
    );
    expect(
      container.querySelector<HTMLSelectElement>("#statement-money-source")?.value,
    ).toBe("source-dbs");

    await enterInput("#statement-password", "wrong-password");
    await click("Use once");
    expect(unlockSourceDocument).toHaveBeenLastCalledWith(
      "document-1",
      "source-dbs",
      "wrong-password",
      false,
    );
    expect(container.textContent).toContain(
      "That password did not unlock this statement.",
    );
    expect(
      container.querySelector<HTMLInputElement>("#statement-password")?.value,
    ).toBe("");

    await enterInput("#statement-password", "current-password");
    await click("Update saved password");
    expect(unlockSourceDocument).toHaveBeenLastCalledWith(
      "document-1",
      "source-dbs",
      "current-password",
      true,
    );
    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.textContent).toContain("Statement unlocked");
    expect(container.textContent).toContain("View document");
    expect(container.textContent).toContain("Routing unavailable");
    expect(container.textContent).not.toContain("Check routing");
  });

  it("maps technical document statuses to the canonical primary row states", async () => {
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [
        sourceDocument({
          documentId: "protected",
          documentStatus: "protected_unlocked",
          originalFilename: "Protected.pdf",
        }),
        sourceDocument({
          documentId: "inspection-failed",
          documentStatus: "inspection_failed",
          originalFilename: "Corrupt.pdf",
        }),
        sourceDocument({
          documentId: "unavailable",
          documentStatus: "unavailable",
          originalFilename: "Unavailable.pdf",
        }),
      ]),
    });

    await mount(api);

    const statusFor = (filename: string) => [...container.querySelectorAll(".evidence-row")]
      .find((row) => row.textContent?.includes(filename))
      ?.querySelector(".doc-status")
      ?.textContent;
    expect(statusFor("Protected.pdf")).toBe("Ready");
    expect(statusFor("Corrupt.pdf")).toBe("Needs attention");
    expect(statusFor("Unavailable.pdf")).toBe("Missing");
  });

  it("distinguishes a missing device-local saved password from an invalid one", async () => {
    const api = createApi({
      listStatementPasswordSources: vi.fn(async () => [{
        displayName: "DBS",
        hasSavedPassword: true,
        moneySourceId: "source-dbs",
      }]),
      listUnassignedSourceDocuments: vi.fn(async () => [
        sourceDocument({ documentStatus: "password_required" }),
      ]),
      trySavedStatementPassword: vi.fn(async () => "unavailable" as const),
    });

    await mount(api);
    await click("Unlock");

    expect(container.textContent).toContain(
      "The saved password is not available on this Mac.",
    );
    expect(container.textContent).not.toContain(
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
        sourceDocument({ documentStatus: "password_required" }),
      ]),
    });

    await mount(api);
    await click("Unlock");
    expect(container.textContent).toContain("Money Sources couldn’t be loaded");
    expect(container.textContent).not.toContain("No Money Source is configured");

    await click("Try again");
    expect(listSources).toHaveBeenCalledTimes(2);
    expect(container.querySelector("#statement-money-source")).not.toBeNull();
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
        sourceDocument({ documentStatus: "password_required" }),
      ]),
      onVaultLocked: vi.fn(async (handler) => {
        notifyLocked = handler;
        return () => undefined;
      }),
      trySavedStatementPassword: vi.fn(() => savedPassword.promise),
    });

    await mount(api);
    await click("Unlock");
    expect(api.trySavedStatementPassword).toHaveBeenCalledTimes(1);

    await act(async () => {
      notifyLocked?.();
      savedPassword.resolve("unlocked");
      await settle();
    });

    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain("June statement.pdf");
    expect(container.textContent).not.toContain("Statement unlocked");
  });

  it("routes only available evidence, keeps ambiguous evidence unassigned, and locks the Vault", async () => {
    const firstRouting = deferred<SourceDocumentRoutingOutcome>();
    let documents = [
      availableDocument,
      sourceDocument({ documentId: "deleted", fileState: "deleted" }),
      sourceDocument({ documentId: "missing", fileState: "missing" }),
    ];
    const routed: SourceDocumentRoutingOutcome = {
      accountIds: ["account-1"],
      documentId: availableDocument.documentId,
      moneySourceId: "money-source-1",
      reason: null,
      status: "routed",
    };
    const routingOutcomes = [firstRouting.promise, Promise.resolve(routed)];
    const normalizeSourceDocument = vi.fn((documentId: string) => {
      const outcome = routingOutcomes.shift()!;
      if (routingOutcomes.length === 0) {
        documents = [];
      }
      return outcome.then((result) => ({ ...result, documentId }));
    });
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => documents),
      normalizeSourceDocument,
    });

    await mount(api);
    const unavailable = buttons("Routing unavailable");
    expect(unavailable).toHaveLength(2);
    expect(unavailable.every((element) => element.disabled)).toBe(true);
    await act(async () => {
      unavailable.forEach((element) => element.click());
      await settle();
    });
    expect(api.normalizeSourceDocument).not.toHaveBeenCalled();

    await click("Check routing");
    expect(api.normalizeSourceDocument).toHaveBeenCalledWith("document-1");
    expect(button("Checking…").disabled).toBe(true);
    expect(button("Add file").textContent).toBe("Add file");
    expect(button("Add file").disabled).toBe(true);

    await act(async () => {
      firstRouting.resolve({
        accountIds: [],
        documentId: availableDocument.documentId,
        moneySourceId: null,
        reason: "classification_uncertain",
        status: "needs_attention",
      });
      await settle();
    });
    expect(container.textContent).toContain(
      "CanCan could not match this evidence uniquely",
    );

    await click("Check routing");
    expect(container.textContent).toContain("Evidence routed");
    expect(container.textContent).toContain("No evidence needs your attention.");

    await click("Lock Vault");
    expect(api.lockVault).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("Unlock your Vault");
  });

  it("opens rendered PDF pages, navigates them, and clears pixels on close or Vault lock", async () => {
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
    });

    await mount(api);
    const viewTrigger = button("View document");
    await click("View document");

    expect(api.renderSourceDocumentPage).toHaveBeenLastCalledWith("document-1", 1);
    expect(container.querySelector('[role="dialog"]')).not.toBeNull();
    expect(container.querySelector("img")?.getAttribute("src")).toBe(
      "data:image/png;base64,cmVuZGVyZWQtcGFnZQ==",
    );
    expect(container.textContent).toContain("Page 1 of 2");

    const close = button("Close");
    const next = button("Next");
    viewTrigger.focus();
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab" }));
      await settle();
    });
    expect(document.activeElement).toBe(close);
    viewTrigger.focus();
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", shiftKey: true }));
      await settle();
    });
    expect(document.activeElement).toBe(next);
    close.focus();
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", shiftKey: true }));
      await settle();
    });
    expect(document.activeElement).toBe(next);
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab" }));
      await settle();
    });
    expect(document.activeElement).toBe(close);

    await click("Next");
    expect(api.renderSourceDocumentPage).toHaveBeenLastCalledWith("document-1", 2);
    expect(container.textContent).toContain("Page 2 of 2");

    await click("Close");
    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(document.activeElement).toBe(viewTrigger);

    await click("View document");
    await click("Lock Vault");
    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(container.textContent).toContain("Unlock your Vault");
  });

  it("deletes an available source file and keeps its tombstone visible", async () => {
    const deletedDocument = sourceDocument({ fileState: "deleted" });
    const listUnassignedSourceDocuments = vi
      .fn<() => Promise<SourceDocumentSummary[]>>()
      .mockResolvedValueOnce([availableDocument])
      .mockResolvedValueOnce([deletedDocument]);
    const api = createApi({ listUnassignedSourceDocuments });

    await mount(api);
    await click("Delete source file");

    expect(api.deleteSourceDocument).toHaveBeenCalledWith(
      availableDocument.documentId,
    );
    expect(container.textContent).toContain("Source file deleted");
    expect(container.textContent).toContain(availableDocument.originalFilename);
    expect(container.textContent).toContain("File deleted");
    expect(container.textContent).not.toContain("Delete source file");
    expect(container.textContent).toContain("View unavailable");
    expect(container.textContent).toContain("Routing unavailable");
  });

  it("keeps an available source file when deletion confirmation is cancelled", async () => {
    const listUnassignedSourceDocuments = vi.fn(async () => [availableDocument]);
    const api = createApi({
      deleteSourceDocument: vi.fn(async () => false),
      listUnassignedSourceDocuments,
    });

    await mount(api);
    await click("Delete source file");

    expect(api.deleteSourceDocument).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain(availableDocument.originalFilename);
    expect(container.textContent).toContain("Delete source file");
    expect(container.textContent).not.toContain("Source file deleted");
    expect(listUnassignedSourceDocuments).toHaveBeenCalledTimes(2);
  });

  it("shows the deleted tombstone when storage removal fails after the decision commits", async () => {
    const deletedDocument = sourceDocument({ fileState: "deleted" });
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

    await mount(api);
    await click("Delete source file");

    expect(container.textContent).toContain("File deleted");
    expect(container.textContent).toContain(
      "CanCan couldn’t finish removing this Vault file.",
    );
    expect(container.textContent).not.toContain("Delete source file");
    expect(listUnassignedSourceDocuments).toHaveBeenCalledTimes(2);
  });

  it("locks after 15 minutes of inactivity and resets the deadline on activity", async () => {
    vi.useFakeTimers();
    const api = createApi();

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(14 * 60 * 1000);
    });
    expect(api.lockVault).not.toHaveBeenCalled();

    window.dispatchEvent(new PointerEvent("pointermove"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(14 * 60 * 1000);
    });
    expect(api.lockVault).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60 * 1000);
      await settle();
    });
    expect(api.lockVault).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("Unlock your Vault");
  });

  it("clears document names and rendered pixels when the native host reports a system lock", async () => {
    let notifyVaultLocked = () => undefined;
    const removeListener = vi.fn();
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      onVaultLocked: vi.fn(async (handler) => {
        notifyVaultLocked = handler;
        return removeListener;
      }),
    });

    await mount(api);
    await click("View document");
    expect(container.querySelector("img")).not.toBeNull();

    await act(async () => {
      notifyVaultLocked();
      await settle();
    });

    expect(api.lockVault).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain(availableDocument.originalFilename);
    expect(container.querySelector("img")).toBeNull();
  });

  it("does not restore an unlocked view from a status request started before a system lock", async () => {
    let notifyVaultLocked = () => undefined;
    const access = deferred<VaultAccessStatus>();
    const api = createApi({
      onVaultLocked: vi.fn(async (handler) => {
        notifyVaultLocked = handler;
        return () => undefined;
      }),
      vaultAccessStatus: vi.fn(() => access.promise),
    });

    await mount(api);
    await act(async () => {
      notifyVaultLocked();
      access.resolve({ recoveryConfigured: false, rememberedOnThisMac: false, status: "unlocked" });
      await settle();
    });

    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain("Add file");
    expect(api.listUnassignedSourceDocuments).not.toHaveBeenCalled();
  });

  it("does not apply an unlock completion that loses a race with a system lock", async () => {
    let notifyVaultLocked = () => undefined;
    const unlock = deferred<VaultStatus>();
    const api = createApi({
      onVaultLocked: vi.fn(async (handler) => {
        notifyVaultLocked = handler;
        return () => undefined;
      }),
      unlockVault: vi.fn(() => unlock.promise),
      vaultAccessStatus: vi.fn(async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "locked",
      })),
    });

    await mount(api);
    await enterPassword("vault-password");
    await click("Unlock Vault");
    await act(async () => {
      notifyVaultLocked();
      unlock.resolve("unlocked");
      await settle();
    });

    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain("Add file");
    expect(container.querySelector<HTMLInputElement>("#vault-password")?.value).toBe("");
    expect(api.listUnassignedSourceDocuments).not.toHaveBeenCalled();
  });

  it("shows the backend Vault status after a manual lock", async () => {
    const api = createApi({
      lockVault: vi.fn(async (): Promise<VaultStatus> => "not_created"),
    });

    await mount(api);
    await click("Lock Vault");

    expect(container.textContent).toContain("Create your Vault");
    expect(container.textContent).not.toContain("Unlock your Vault");
  });

  it("shows the backend Vault status after an inactivity lock", async () => {
    vi.useFakeTimers();
    const api = createApi({
      lockVault: vi.fn(async (): Promise<VaultStatus> => "not_created"),
    });

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });

    expect(container.textContent).toContain("Create your Vault");
    expect(container.textContent).not.toContain("Unlock your Vault");
  });

  it("clears document names and rendered pixels while an inactivity lock is pending", async () => {
    vi.useFakeTimers();
    const inactivityLock = deferred<VaultStatus>();
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      lockVault: vi.fn(() => inactivityLock.promise),
    });

    await mount(api);
    await click("View document");
    expect(container.querySelector("img")).not.toBeNull();
    expect(container.textContent).toContain(availableDocument.originalFilename);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });

    expect(container.textContent).toContain("Checking your Vault");
    expect(container.textContent).not.toContain(availableDocument.originalFilename);
    expect(container.querySelector("img")).toBeNull();

    await act(async () => {
      inactivityLock.resolve("locked");
      await settle();
    });
    expect(container.textContent).toContain("Unlock your Vault");
  });

  it("blocks a document reload started by an import while an inactivity lock is pending", async () => {
    vi.useFakeTimers();
    const importDocument = deferred<SourceDocumentImportOutcome | null>();
    const inactivityLock = deferred<VaultStatus>();
    const listUnassignedSourceDocuments = vi.fn(async () => [availableDocument]);
    const api = createApi({
      importSourceDocument: vi.fn(() => importDocument.promise),
      listUnassignedSourceDocuments,
      lockVault: vi.fn(() => inactivityLock.promise),
    });

    await mount(api);
    await click("Add file");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });
    expect(container.textContent).toContain("Checking your Vault");

    await act(async () => {
      importDocument.resolve({ documentId: "document-1", status: "imported" });
      await settle();
    });

    expect(listUnassignedSourceDocuments).toHaveBeenCalledTimes(1);
    expect(container.textContent).not.toContain(availableDocument.originalFilename);

    await act(async () => {
      inactivityLock.resolve("locked");
      await settle();
    });
  });

  it("shows the locked gate when status reconciliation confirms a failed lock command took effect", async () => {
    vi.useFakeTimers();
    const vaultStatus = vi
      .fn<() => Promise<VaultStatus>>()
      .mockResolvedValueOnce("locked");
    const api = createApi({
      lockVault: vi.fn(async () => {
        throw { code: "runtime_unavailable" };
      }),
      vaultStatus,
    });

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });

    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain(
      "Couldn’t complete that request. Try again.",
    );
    expect(vaultStatus).toHaveBeenCalledTimes(1);
  });

  it("does not reconcile a pending inactivity lock after unmount", async () => {
    vi.useFakeTimers();
    const inactivityLock = deferred<VaultStatus>();
    const vaultStatus = vi.fn(async (): Promise<VaultStatus> => "unlocked");
    const api = createApi({
      lockVault: vi.fn(() => inactivityLock.promise),
      vaultStatus,
    });

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
      root.unmount();
      await settle();
    });
    root = createRoot(container);

    await act(async () => {
      inactivityLock.reject({ code: "runtime_unavailable" });
      await settle();
    });

    expect(vaultStatus).not.toHaveBeenCalled();
  });

  it("does not restore document names when an earlier list finishes after inactivity lock", async () => {
    vi.useFakeTimers();
    const documents = deferred<SourceDocumentSummary[]>();
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(() => documents.promise),
    });

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });
    expect(container.textContent).toContain("Unlock your Vault");

    await act(async () => {
      documents.resolve([availableDocument]);
      await settle();
    });
    expect(container.textContent).not.toContain(availableDocument.originalFilename);
  });

  it("does not show an earlier list failure after inactivity lock", async () => {
    vi.useFakeTimers();
    const documents = deferred<SourceDocumentSummary[]>();
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(() => documents.promise),
    });

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });
    expect(container.textContent).toContain("Unlock your Vault");

    await act(async () => {
      documents.reject({ code: "runtime_unavailable" });
      await settle();
    });
    expect(container.textContent).not.toContain(
      "Couldn’t complete that request. Try again.",
    );
  });

  it("shows a failed inactivity lock and retries after the next deadline", async () => {
    vi.useFakeTimers();
    const lockVault = vi
      .fn<() => Promise<VaultStatus>>()
      .mockRejectedValueOnce({ code: "runtime_unavailable" })
      .mockResolvedValue("locked");
    const api = createApi({ lockVault });

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });
    expect(lockVault).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain(
      "Couldn’t complete that request. Try again.",
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });
    expect(lockVault).toHaveBeenCalledTimes(2);
    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain(
      "Couldn’t complete that request. Try again.",
    );
  });

  it("does not restore page pixels when a render finishes after the viewer closes", async () => {
    const nextPage = deferred<RenderedDocumentPage>();
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      renderSourceDocumentPage: vi.fn(
        async (_documentId: string, pageNumber: number) =>
          pageNumber === 1
            ? {
                pageCount: 2,
                pageNumber: 1,
                pngBase64: "Zmlyc3QtcGFnZQ==",
              }
            : nextPage.promise,
      ),
    });

    await mount(api);
    await click("View document");
    await click("Next");
    await click("Close");
    await act(async () => {
      nextPage.resolve({
        pageCount: 2,
        pageNumber: 2,
        pngBase64: "c2Vjb25kLXBhZ2U=",
      });
      await settle();
    });

    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.querySelector("img")).toBeNull();
  });

  it("ignores a page render failure after the viewer closes", async () => {
    const nextPage = deferred<RenderedDocumentPage>();
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      renderSourceDocumentPage: vi.fn(
        async (_documentId: string, pageNumber: number) =>
          pageNumber === 1
            ? {
                pageCount: 2,
                pageNumber: 1,
                pngBase64: "Zmlyc3QtcGFnZQ==",
              }
            : nextPage.promise,
      ),
    });

    await mount(api);
    await click("View document");
    await click("Next");
    await click("Close");
    await act(async () => {
      nextPage.reject({ code: "document_render_failed" });
      await settle();
    });

    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(container.textContent).not.toContain("CanCan couldn’t render that PDF page.");
  });

  it("closes the viewer and shows a safe error when page navigation fails", async () => {
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      renderSourceDocumentPage: vi.fn(
        async (_documentId: string, pageNumber: number) => {
          if (pageNumber === 2) {
            throw { code: "document_render_failed", privateDetail: "Core Graphics detail" };
          }
          return {
            pageCount: 2,
            pageNumber: 1,
            pngBase64: "Zmlyc3QtcGFnZQ==",
          };
        },
      ),
    });

    await mount(api);
    await click("View document");
    await click("Next");

    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(container.textContent).toContain("CanCan couldn’t render that PDF page.");
    expect(container.textContent).not.toContain("Core Graphics detail");
  });

  it("shows safe command errors without exposing backend details", async () => {
    const api = createApi({
      unlockVault: vi.fn(async (): Promise<VaultStatus> => {
        throw { code: "invalid_credentials" };
      }),
      vaultAccessStatus: vi.fn(async (): Promise<VaultAccessStatus> => ({
        recoveryConfigured: false,
        rememberedOnThisMac: false,
        status: "locked",
      })),
    });

    await mount(api);
    await enterPassword("wrong-password");
    await click("Unlock Vault");

    expect(container.textContent).toContain(
      "That password did not unlock this Vault.",
    );
    expect(container.textContent).not.toContain("invalid_credentials");
  });
});
