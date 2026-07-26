// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type {
  RenderedDocumentPage,
  SourceDocumentImportOutcome,
  SourceDocumentSummary,
  VaultAccessStatus,
  VaultStatus,
} from "./command-contracts";
import {
  availableDocument,
  click,
  container,
  createApi,
  deferred,
  enterPassword,
  installAppHarness,
  mount,
  recreateRoot,
  root,
  settle,
} from "./test-support/app-harness";

installAppHarness();

describe("App session lock and viewer race orchestration", () => {
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

    await mount(api, "sources");
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

    await mount(api, "sources");
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

    await mount(api, "sources");
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
    recreateRoot();

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

    await mount(api, "sources");
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

    await mount(api, "sources");
    await click("View document");
    await click("Next");
    await click("Close");
    await act(async () => {
      nextPage.reject({ code: "document_render_failed" });
      await settle();
    });

    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(container.textContent).not.toContain("CanCan couldn’t render that document.");
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

    await mount(api, "sources");
    await click("View document");
    await click("Next");

    expect(container.querySelector('[role="dialog"]')).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(container.textContent).toContain("CanCan couldn’t render that document.");
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
