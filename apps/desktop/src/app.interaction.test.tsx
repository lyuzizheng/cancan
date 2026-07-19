// @vitest-environment happy-dom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  SourceDocumentImportOutcome,
  RenderedDocumentPage,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
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
    importSourceDocument: vi.fn(
      async (): Promise<SourceDocumentImportOutcome | null> => null,
    ),
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
    renderSourceDocumentPage: vi.fn(
      async (_documentId: string, pageNumber: number): Promise<RenderedDocumentPage> => ({
        pageCount: 2,
        pageNumber,
        pngBase64: "cmVuZGVyZWQtcGFnZQ==",
      }),
    ),
    unlockVault: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
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
  const input = container.querySelector<HTMLInputElement>("#vault-password");
  expect(input).not.toBeNull();
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  expect(setter).toBeDefined();

  await act(async () => {
    setter!.call(input, password);
    input!.dispatchEvent(new Event("input", { bubbles: true }));
    await settle();
  });
}

describe("App manual import orchestration", () => {
  it("unlocks the Vault and loads unassigned documents through the injected API", async () => {
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      unlockVault: vi.fn(async (): Promise<VaultStatus> => "unlocked"),
      vaultStatus: vi.fn(async (): Promise<VaultStatus> => "locked"),
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
      vaultStatus: vi.fn(async (): Promise<VaultStatus> => "not_created"),
    });

    await mount(api);
    await enterPassword("new-vault-password");
    await click("Create Vault");

    expect(api.createVault).toHaveBeenCalledWith("new-vault-password");
    expect(api.unlockVault).not.toHaveBeenCalled();
    expect(button("Add file")).toBeDefined();
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

  it("ignores an inactivity lock result after a manual lock changes the session", async () => {
    vi.useFakeTimers();
    const inactivityLock = deferred<VaultStatus>();
    const lockVault = vi
      .fn<() => Promise<VaultStatus>>()
      .mockReturnValueOnce(inactivityLock.promise)
      .mockResolvedValueOnce("not_created");
    const api = createApi({ lockVault });

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });
    await click("Lock Vault");
    expect(container.textContent).toContain("Create your Vault");

    await act(async () => {
      inactivityLock.resolve("locked");
      await settle();
    });
    expect(container.textContent).toContain("Create your Vault");
    expect(container.textContent).not.toContain("Unlock your Vault");
  });

  it("ignores an inactivity lock failure after a manual lock changes the session", async () => {
    vi.useFakeTimers();
    const inactivityLock = deferred<VaultStatus>();
    const lockVault = vi
      .fn<() => Promise<VaultStatus>>()
      .mockReturnValueOnce(inactivityLock.promise)
      .mockResolvedValueOnce("locked");
    const api = createApi({ lockVault });

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });
    await click("Lock Vault");

    await act(async () => {
      inactivityLock.reject({ code: "runtime_unavailable" });
      await settle();
    });
    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain(
      "Couldn’t complete that request. Try again.",
    );
    expect(vi.getTimerCount()).toBe(0);
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
      vaultStatus: vi.fn(async (): Promise<VaultStatus> => "locked"),
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
