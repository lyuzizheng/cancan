// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type { SavedStatementPasswordResult } from "./command-contracts";
import {
  bodyDialog,
  click,
  clickDialogButton,
  container,
  createApi,
  deferred,
  dialogInput,
  enterDialogInput,
  installAppHarness,
  mount,
  noBodyDialog,
  settle,
  sourceDocument,
} from "./test-support/app-harness";

installAppHarness();

const protectedDocument = sourceDocument({
  attentionReason: "password_required",
  documentStatus: "needs_attention",
});

const confirmedDbs = {
  hasSavedPassword: false,
  moneySourceDisplayName: "DBS",
  moneySourceId: "source-dbs",
};

function dialogButtonLabels(): (string | undefined)[] {
  return [...bodyDialog().querySelectorAll("button")]
    .map((element) => element.textContent?.trim());
}

describe("App statement-unlock orchestration", () => {
  it("offers a first save only for the host-confirmed source", async () => {
    const unlockSourceDocument = vi.fn(async (): Promise<void> => undefined);
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: vi.fn(async () => confirmedDbs),
      unlockSourceDocument,
    });

    await mount(api, "sources");
    await click("Unlock");

    expect(api.statementPasswordDocumentContext).toHaveBeenCalledWith("document-1");
    // No saved password exists in this Vault, so the host pass is skipped.
    expect(api.trySavedStatementPasswords).not.toHaveBeenCalled();
    expect(bodyDialog().textContent).toContain("This statement belongs to DBS.");
    expect(dialogButtonLabels()).toContain("Save for this source");
    expect(dialogButtonLabels()).not.toContain("Update saved password");
    // The prompt never offers a renderer-chosen save target.
    expect(
      bodyDialog().querySelector('button[aria-label="Money Source for this statement"]'),
    ).toBeNull();

    await enterDialogInput("#statement-password", "current-password");
    await clickDialogButton("Save for this source");

    expect(unlockSourceDocument).toHaveBeenLastCalledWith(
      "document-1",
      "source-dbs",
      "current-password",
      true,
    );
    noBodyDialog();
    expect(container.textContent).toContain("Statement unlocked");
    expect(container.textContent).toContain(
      "The verified password is saved for this Money Source.",
    );
  });

  it("tries the saved password first and replaces it only for the confirmed source", async () => {
    let unlocked = false;
    const unlockSourceDocument = vi.fn(async (): Promise<void> => {
      unlocked = true;
    });
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
      statementPasswordDocumentContext: vi.fn(async () => ({
        ...confirmedDbs,
        hasSavedPassword: true,
      })),
      trySavedStatementPasswords: vi.fn(async () => "invalid" as const),
      unlockSourceDocument,
    });

    await mount(api, "sources");
    expect(container.textContent).toContain("Needs attention");
    expect(container.textContent).not.toContain("Re-run parser");
    await click("Unlock");

    expect(api.trySavedStatementPasswords).toHaveBeenCalledWith("document-1");
    expect(bodyDialog().textContent).toContain("The saved password did not work.");
    expect(dialogButtonLabels()).toContain("Update saved password");
    expect(
      bodyDialog().querySelector('button[aria-label="Money Source for this statement"]'),
    ).toBeNull();

    // A session-only unlock claims no ownership, so it sends a null target.
    await enterDialogInput("#statement-password", "wrong-password");
    await clickDialogButton("Use once");
    expect(unlockSourceDocument).toHaveBeenLastCalledWith(
      "document-1",
      null,
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
    expect(container.textContent).toContain(
      "The verified password replaced this Money Source’s saved password.",
    );
    expect(container.textContent).toContain("View document");
    expect(container.textContent).not.toContain("Parser remains unavailable");
    await click("Re-run parser");
    expect(api.reparseSourceDocument).toHaveBeenCalledWith("document-1");
  });

  it("runs one host-owned saved-password pass and keeps session-only unlock available", async () => {
    const api = createApi({
      listStatementPasswordSources: vi.fn(async () => [
        { displayName: "DBS", hasSavedPassword: true, moneySourceId: "source-dbs" },
        { displayName: "Wise", hasSavedPassword: true, moneySourceId: "source-wise" },
      ]),
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: vi.fn(async () => confirmedDbs),
      trySavedStatementPasswords: vi.fn(async () => "invalid" as const),
    });

    await mount(api, "sources");
    await click("Unlock");

    // The host tries each distinct saved password itself: the renderer sends
    // the document only and never selects the attempted secret.
    expect(api.trySavedStatementPasswords).toHaveBeenCalledTimes(1);
    expect(api.trySavedStatementPasswords).toHaveBeenCalledWith("document-1");
    expect(bodyDialog().textContent).toContain("The saved password did not work.");
    expect(dialogInput("#statement-password").disabled).toBe(false);
  });

  it("keeps session-only unlock available while the source is unconfirmed", async () => {
    const unlockSourceDocument = vi.fn(async (): Promise<void> => undefined);
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: vi.fn(async () => ({
        hasSavedPassword: false,
        moneySourceDisplayName: null,
        moneySourceId: null,
      })),
      unlockSourceDocument,
    });

    await mount(api, "sources");
    await click("Unlock");

    expect(bodyDialog().textContent).toContain(
      "This statement’s Money Source isn’t confirmed yet",
    );
    expect(dialogButtonLabels()).toContain("Use once");
    expect(dialogButtonLabels()).not.toContain("Save for this source");
    expect(dialogButtonLabels()).not.toContain("Update saved password");

    await enterDialogInput("#statement-password", "session-password");
    // Enter in the password field is the form's implicit submission: it runs
    // the session-only path, never a Keychain write.
    await act(async () => {
      dialogInput("#statement-password").closest("form")!.requestSubmit();
      await settle();
    });

    expect(unlockSourceDocument).toHaveBeenLastCalledWith(
      "document-1",
      null,
      "session-password",
      false,
    );
    noBodyDialog();
    expect(container.textContent).toContain(
      "This statement is ready to view and route until the Vault locks.",
    );
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
      statementPasswordDocumentContext: vi.fn(async () => ({
        ...confirmedDbs,
        hasSavedPassword: true,
      })),
      trySavedStatementPasswords: vi.fn(async () => {
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

  it("distinguishes a missing device-local saved password from an invalid one", async () => {
    const api = createApi({
      listStatementPasswordSources: vi.fn(async () => [{
        displayName: "DBS",
        hasSavedPassword: true,
        moneySourceId: "source-dbs",
      }]),
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: vi.fn(async () => ({
        ...confirmedDbs,
        hasSavedPassword: true,
      })),
      trySavedStatementPasswords: vi.fn(async () => "unavailable" as const),
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

  it("reports a failed password-context read with a retry that reloads it", async () => {
    const context = vi.fn()
      .mockRejectedValueOnce({ code: "document_unavailable" })
      .mockResolvedValueOnce(confirmedDbs);
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: context,
    });

    await mount(api, "sources");
    await click("Unlock");

    expect(bodyDialog().textContent).toContain(
      "Statement password details couldn’t be loaded",
    );
    expect(bodyDialog().textContent).toContain("This file is no longer available.");
    expect(bodyDialog().textContent).not.toContain("This statement belongs to DBS.");

    await clickDialogButton("Try again");

    expect(context).toHaveBeenCalledTimes(2);
    expect(bodyDialog().textContent).toContain("This statement belongs to DBS.");
    expect(dialogButtonLabels()).toContain("Save for this source");
  });

  it("surfaces a stale mismatch as a deterministic host rejection", async () => {
    const unlockSourceDocument = vi.fn(async (): Promise<void> => {
      throw { code: "statement_password_source_mismatch" };
    });
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: vi.fn(async () => confirmedDbs),
      unlockSourceDocument,
    });

    await mount(api, "sources");
    await click("Unlock");
    await enterDialogInput("#statement-password", "current-password");
    await clickDialogButton("Save for this source");

    expect(unlockSourceDocument).toHaveBeenLastCalledWith(
      "document-1",
      "source-dbs",
      "current-password",
      true,
    );
    expect(bodyDialog().textContent).toContain(
      "This statement belongs to a different Money Source. Refresh and try again.",
    );
    expect(container.textContent).not.toContain("Statement unlocked");
    // The prompt stays open on the rejection instead of guessing a new owner.
    expect(dialogInput("#statement-password").disabled).toBe(false);
    expect(dialogButtonLabels()).toContain("Save for this source");
  });

  it("surfaces a stale unconfirmed source as a deterministic host rejection", async () => {
    const unlockSourceDocument = vi.fn(async (): Promise<void> => {
      throw { code: "statement_password_source_unconfirmed" };
    });
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: vi.fn(async () => confirmedDbs),
      unlockSourceDocument,
    });

    await mount(api, "sources");
    await click("Unlock");
    await enterDialogInput("#statement-password", "current-password");
    await clickDialogButton("Save for this source");

    expect(bodyDialog().textContent).toContain(
      "Confirm this statement’s Money Source before saving its password.",
    );
    expect(container.textContent).not.toContain("Statement unlocked");
  });

  it("ignores a cancelled prompt's in-flight saved-password pass", async () => {
    const savedPassword = deferred<SavedStatementPasswordResult>();
    const api = createApi({
      listStatementPasswordSources: vi.fn(async () => [{
        displayName: "DBS",
        hasSavedPassword: true,
        moneySourceId: "source-dbs",
      }]),
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: vi.fn(async () => ({
        ...confirmedDbs,
        hasSavedPassword: true,
      })),
      trySavedStatementPasswords: vi.fn(() => savedPassword.promise),
    });

    await mount(api, "sources");
    await click("Unlock");
    expect(api.trySavedStatementPasswords).toHaveBeenCalledTimes(1);

    await clickDialogButton("Close");
    noBodyDialog();

    await act(async () => {
      savedPassword.resolve("unlocked");
      await settle();
    });

    noBodyDialog();
    expect(container.textContent).not.toContain("Statement unlocked");
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
      listUnassignedSourceDocuments: vi.fn(async () => [protectedDocument]),
      statementPasswordDocumentContext: vi.fn(async () => ({
        ...confirmedDbs,
        hasSavedPassword: true,
      })),
      onVaultLocked: vi.fn(async (handler) => {
        notifyLocked = handler;
        return () => undefined;
      }),
      trySavedStatementPasswords: vi.fn(() => savedPassword.promise),
    });

    await mount(api, "sources");
    await click("Unlock");
    expect(api.trySavedStatementPasswords).toHaveBeenCalledTimes(1);

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
});
