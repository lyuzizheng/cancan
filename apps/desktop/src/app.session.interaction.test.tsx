// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type {
  RenderedDocumentPage,
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
  settle,
} from "./test-support/app-harness";

installAppHarness();

describe("App vault-locked event and viewer race orchestration", () => {
  it("does not lock automatically after inactivity", async () => {
    vi.useFakeTimers();
    const api = createApi();

    await mount(api);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15 * 60 * 1000);
      await settle();
    });

    expect(api.lockVault).not.toHaveBeenCalled();
    expect(container.textContent).not.toContain("Unlock your Vault");
  });

  it("clears document names and rendered pixels when the host reports the Vault is locked", async () => {
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

  it("does not restore an unlocked view from a status request started before the host reports the Vault is locked", async () => {
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

  it("does not apply an unlock completion that loses a race with a host-reported lock", async () => {
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
