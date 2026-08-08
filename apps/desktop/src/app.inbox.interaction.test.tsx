// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type {
  AccountConfirmationOutcome,
  LocalInboxScanSummary,
  LocalInboxStatus,
} from "./command-contracts";
import {
  accountConfirmationPrompt,
  availableDocument,
  click,
  container,
  createApi,
  inboxDisabled,
  inboxEnabled,
  installAppHarness,
  mount,
  settle,
} from "./test-support/app-harness";

installAppHarness();

describe("App local inbox orchestration", () => {
  it("keeps finance data visible when Inbox status fails and retries that module", async () => {
    const localInboxStatus = vi.fn()
      .mockRejectedValueOnce({ code: "local_inbox_storage_failed" })
      .mockResolvedValue(inboxDisabled);
    const api = createApi({
      listUnassignedSourceDocuments: vi.fn(async () => [availableDocument]),
      localInboxStatus,
    });

    await mount(api, "sources");

    expect(container.textContent).toContain("CanCan Inbox couldn’t be checked");
    expect(container.textContent).toContain(availableDocument.originalFilename);
    await click("Try again");
    await act(async () => {
      await settle();
    });

    expect(localInboxStatus).toHaveBeenCalledTimes(2);
    expect(container.textContent).toContain("Add statements without opening CanCan");
    expect(container.textContent).toContain(availableDocument.originalFilename);
  });

  it("turns the inbox on from Sources and reloads finance data", async () => {
    const api = createApi({
      chooseLocalInboxRoot: vi.fn(async (): Promise<LocalInboxStatus> => inboxEnabled),
    });
    await mount(api, "sources");

    expect(container.textContent).toContain("Add statements without opening CanCan");
    await click("Choose Cancan folder");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.chooseLocalInboxRoot).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("CanCan Inbox is on");
    expect(api.listRecentActivity).toHaveBeenCalledTimes(2);
  });

  it("leaves the inbox untouched when the folder picker is cancelled", async () => {
    const api = createApi();
    await mount(api, "sources");

    await click("Choose Cancan folder");

    expect(api.chooseLocalInboxRoot).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("Add statements without opening CanCan");
    expect(container.textContent).not.toContain("Something needs your attention");
  });

  it("rescans the inbox and shows the sanitized summary", async () => {
    const scanSummary: LocalInboxScanSummary = {
      alreadyPresent: 12,
      deferred: 1,
      imported: 3,
      suppressed: 2,
    };
    const api = createApi({
      localInboxStatus: vi.fn()
        .mockResolvedValueOnce(inboxEnabled)
        .mockResolvedValue({ ...inboxEnabled, lastScan: scanSummary }),
      rescanLocalInbox: vi.fn(async (): Promise<LocalInboxScanSummary> => scanSummary),
    });
    await mount(api, "sources");

    await click("Check now");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.rescanLocalInbox).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("Inbox checked");
    expect(container.textContent).toContain(
      "Last check: 3 added; 12 already in CanCan; 1 to try again later; 2 kept deleted.",
    );
  });

  it("requires a second explicit step before turning the inbox off", async () => {
    const api = createApi({
      disableLocalInbox: vi.fn(async (): Promise<LocalInboxStatus> => inboxDisabled),
      localInboxStatus: vi.fn(async (): Promise<LocalInboxStatus> => inboxEnabled),
    });
    await mount(api, "sources");

    await click("Turn off");
    expect(container.textContent).toContain(
      "Turn off CanCan Inbox? Your folder and files stay untouched.",
    );
    expect(api.disableLocalInbox).not.toHaveBeenCalled();

    await click("Keep");
    expect(container.textContent).not.toContain(
      "Turn off CanCan Inbox? Your folder and files stay untouched.",
    );
    expect(container.textContent).toContain("Turn off");
    await click("Turn off");
    await click("Turn off");

    expect(api.disableLocalInbox).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("CanCan Inbox is off");
  });

  it("shows the host's plain-language error when a rescan is refused", async () => {
    const api = createApi({
      localInboxStatus: vi.fn(async (): Promise<LocalInboxStatus> => inboxEnabled),
      rescanLocalInbox: vi.fn(async (): Promise<LocalInboxScanSummary> => {
        throw { code: "local_inbox_reauthorization_required" };
      }),
    });
    await mount(api, "sources");

    await click("Check now");

    expect(container.textContent).toContain(
      "Choose your Cancan folder again so CanCan can reach it.",
    );
  });
});

describe("App candidate-account decisions", () => {
  it("saves one decision for every current candidate and reloads", async () => {
    const api = createApi({
      listAccountConfirmationPrompts: vi.fn()
        .mockResolvedValueOnce([accountConfirmationPrompt])
        .mockResolvedValue([]),
    });
    await mount(api, "sources");

    await click("Save choices");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.decideCandidateAccounts).toHaveBeenCalledWith(
      "money-source-1",
      "proposal-version-1",
      [
        { accountId: "account-dbs", action: "accept" },
        { accountId: "account-card", action: "accept" },
      ],
    );
    expect(container.textContent).toContain("Account choices saved");
  });

  it("restores a dismissed account explicitly", async () => {
    const prompt = {
      ...accountConfirmationPrompt,
      candidateAccounts: [],
      dismissedAccounts: [accountConfirmationPrompt.candidateAccounts[0]!],
    };
    const api = createApi({
      listAccountConfirmationPrompts: vi.fn(async () => [prompt]),
    });
    await mount(api, "sources");

    await click("Restore");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.restoreDismissedCandidateAccount).toHaveBeenCalledWith("account-dbs");
    expect(container.textContent).toContain("Account restored");
  });

  it("keeps the prompt when the candidate set changed", async () => {
    const api = createApi({
      decideCandidateAccounts: vi.fn(
        async (): Promise<AccountConfirmationOutcome> => ({ status: "conflict" }),
      ),
      listAccountConfirmationPrompts: vi.fn(async () => [accountConfirmationPrompt]),
    });
    await mount(api, "sources");

    await click("Save choices");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("That account list changed");
    expect(container.textContent).toContain("CanCan found new accounts");
  });

  it("reports an idempotent account decision without claiming a new save", async () => {
    const api = createApi({
      decideCandidateAccounts: vi.fn(
        async (): Promise<AccountConfirmationOutcome> => ({ status: "already_confirmed" }),
      ),
      listAccountConfirmationPrompts: vi.fn(async () => [accountConfirmationPrompt]),
    });
    await mount(api, "sources");

    await click("Save choices");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("Account choices already saved");
  });
});
