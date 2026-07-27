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
  click,
  container,
  coveragePrompt,
  createApi,
  enterRemindDate,
  inboxDisabled,
  inboxEnabled,
  installAppHarness,
  mount,
  settle,
} from "./test-support/app-harness";

installAppHarness();

describe("App local inbox orchestration", () => {
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
    expect(container.textContent).toContain("New statements you save to Inbox are added for you");
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
    expect(container.textContent).toContain("Check now");
    expect(container.textContent).not.toContain("Turn off CanCan Inbox?");

    await click("Turn off");
    await click("Turn off");

    expect(api.disableLocalInbox).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("CanCan Inbox is off");
    expect(container.textContent).toContain("Choose Cancan folder");
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

    expect(container.textContent).toContain("Something needs your attention");
    expect(container.textContent).toContain(
      "Choose your Cancan folder again so CanCan can reach it.",
    );
  });

  it("keeps finance data available when the optional inbox status fails", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [{
        displayName: "Synthetic Bank",
        moneySourceId: "money-source-1",
        sourceType: "bank",
      }]),
      localInboxStatus: vi.fn(async (): Promise<LocalInboxStatus> => {
        throw { code: "local_inbox_storage_failed" };
      }),
    });
    await mount(api, "sources");

    expect(container.textContent).toContain("Synthetic Bank");
    expect(container.textContent).toContain("CanCan Inbox unavailable");
    expect(container.textContent).toContain(
      "CanCan couldn’t reach the local Inbox setup.",
    );
    expect(container.textContent).not.toContain("Something needs your attention");
  });
});

describe("App attention orchestration", () => {
  it("records a not-expected decision and reloads the prompts", async () => {
    const api = createApi({
      listStatementCoveragePrompts: vi.fn()
        .mockResolvedValueOnce([coveragePrompt])
        .mockResolvedValue([]),
    });
    await mount(api);

    expect(container.textContent).toContain("Needs attention");
    expect(container.textContent).toContain("June 2026 bank statement is missing");
    await click("Not expected");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.recordStatementCoverageDecision).toHaveBeenCalledWith({
      accountId: "account-dbs",
      action: "not_expected",
      documentType: "bank_statement",
      moneySourceId: "money-source-1",
      statementPeriodFrom: "2026-06-01",
      statementPeriodTo: "2026-06-30",
    });
    expect(container.textContent).toContain("CanCan won’t ask about that period again.");
    expect(container.textContent).not.toContain("June 2026 bank statement is missing");
  });

  it("blocks a reminder that is not in the future", async () => {
    const api = createApi({
      listStatementCoveragePrompts: vi.fn(async () => [coveragePrompt]),
    });
    await mount(api);

    await click("Remind later");
    await enterRemindDate("2020-01-01");
    await click("Save reminder");

    expect(api.recordStatementCoverageDecision).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Pick a future date.");
  });

  it("records a reminder for a future date", async () => {
    const api = createApi({
      listStatementCoveragePrompts: vi.fn()
        .mockResolvedValueOnce([coveragePrompt])
        .mockResolvedValue([]),
    });
    await mount(api);

    await click("Remind later");
    await enterRemindDate("2099-01-01");
    await click("Save reminder");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.recordStatementCoverageDecision).toHaveBeenCalledWith({
      accountId: "account-dbs",
      action: "remind_later",
      documentType: "bank_statement",
      moneySourceId: "money-source-1",
      remindAfter: "2099-01-01",
      statementPeriodFrom: "2026-06-01",
      statementPeriodTo: "2026-06-30",
    });
    expect(container.textContent).toContain("Reminder saved");
    expect(container.textContent).toContain("CanCan will ask again after 1 Jan 2099.");
  });

  it("explains a stale coverage decision and reloads", async () => {
    const api = createApi({
      listStatementCoveragePrompts: vi.fn(async () => [coveragePrompt]),
      recordStatementCoverageDecision: vi.fn(async (): Promise<void> => {
        throw { code: "coverage_decision_invalid" };
      }),
    });
    await mount(api);

    await click("Not expected");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("Couldn’t save that");
    expect(container.textContent).toContain(
      "That prompt changed. CanCan reloaded the latest list.",
    );
    expect(api.listStatementCoveragePrompts).toHaveBeenCalledTimes(2);
  });

  it("confirms candidate accounts and reloads", async () => {
    const api = createApi({
      listAccountConfirmationPrompts: vi.fn()
        .mockResolvedValueOnce([accountConfirmationPrompt])
        .mockResolvedValue([]),
    });
    await mount(api);

    expect(container.textContent).toContain(
      "CanCan found new accounts in your Synthetic Bank statements",
    );
    await click("These are mine");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.confirmCandidateAccounts).toHaveBeenCalledWith(
      "money-source-1",
      ["account-dbs", "account-card"],
    );
    expect(container.textContent).toContain("Accounts confirmed");
    expect(container.textContent).not.toContain("CanCan found new accounts");
  });

  it("explains a conflicting account confirmation without hiding the prompt", async () => {
    const api = createApi({
      confirmCandidateAccounts: vi.fn(
        async (): Promise<AccountConfirmationOutcome> => ({ status: "conflict" }),
      ),
      listAccountConfirmationPrompts: vi.fn(async () => [accountConfirmationPrompt]),
    });
    await mount(api);

    await click("These are mine");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("That account list changed");
    expect(container.textContent).toContain(
      "CanCan found new accounts in your Synthetic Bank statements",
    );
  });

  it("clears inbox and attention state when the Vault locks", async () => {
    const api = createApi({
      listStatementCoveragePrompts: vi.fn(async () => [coveragePrompt]),
      localInboxStatus: vi.fn(async (): Promise<LocalInboxStatus> => inboxEnabled),
    });
    await mount(api);

    expect(container.textContent).toContain("Needs attention");
    await click("Lock Vault");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain("Needs attention");
    expect(container.textContent).not.toContain("June 2026 bank statement is missing");
  });
});
