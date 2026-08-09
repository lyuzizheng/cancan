// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type {
  MoneyOverview,
  RecentActivitySummary,
  RelationshipCandidateSummary,
  ReviewItemSummary,
  ReviewJobSummary,
  ReviewMutationOutcome,
} from "./command-contracts";
import {
  button,
  buttons,
  click,
  container,
  createApi,
  deferred,
  enterField,
  enterPassword,
  installAppHarness,
  linkedReviewItem,
  mount,
  navItem,
  reviewCandidate,
  reviewItem,
  settle,
} from "./test-support/app-harness";

installAppHarness();

describe("App review and overview orchestration", () => {
  it("lands on Overview with money totals, review count, and activity after unlock", async () => {
    const api = createApi({
      getMoneyOverview: vi.fn(async (): Promise<MoneyOverview> => ({
        assets: [{
          accountId: "account-1",
          accountLabel: "DBS Multiplier Account",
          asOf: "2026-07-25",
          currency: "SGD",
          value: "6245.00",
        }],
        liabilities: [{
          accountId: "account-2",
          accountLabel: "DBS Visa Card",
          asOf: "2026-07-24",
          currency: "SGD",
          value: "-512.34",
        }],
      })),
      listRecentActivity: vi.fn(async (): Promise<RecentActivitySummary[]> => [
        {
          canUndo: true,
          eventDate: "2026-07-25",
          eventId: "event-1",
          eventType: "purchase",
          sourceLabels: ["DBS"],
          spending: true,
        },
        {
          canUndo: false,
          eventDate: "2026-07-24",
          eventId: "event-2",
          eventType: "credit_card_repayment",
          sourceLabels: [],
          spending: false,
        },
      ]),
      listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => [
        reviewItem,
        linkedReviewItem,
      ]),
    });

    await mount(api);

    expect(container.textContent).toContain("Your money, organized");
    expect(container.textContent).toContain("Money Overview");
    expect(container.textContent).toContain("SGD 6,245.00");
    expect(container.textContent).toContain("SGD -512.34");
    expect(container.textContent).toContain("You're all caught up");
    expect(container.textContent).toContain("Purchase");
    expect(container.textContent).toContain("Spending");
    expect(container.textContent).toContain("25 Jul 2026 · DBS");
    expect(buttons("Undo")).toHaveLength(1);
  });

  it("switches between Overview, Sources, and Review from the vault nav", async () => {
    const api = createApi({
      listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => [
        reviewItem,
        linkedReviewItem,
      ]),
    });
    await mount(api);

    expect(navItem("Overview").getAttribute("aria-current")).toBe("page");
    expect(navItem("Review").textContent).toContain("2");

    await act(async () => {
      navItem("Review").click();
      await settle();
    });
    expect(navItem("Review").getAttribute("aria-current")).toBe("page");
    expect(container.textContent).toContain("Needs your check");
    expect(container.textContent).toContain("Looks like a card repayment");

    await act(async () => {
      navItem("Sources").click();
      await settle();
    });
    expect(navItem("Sources").getAttribute("aria-current")).toBe("page");
    expect(button("Add file")).toBeDefined();
  });

  it("opens review details with linked-record candidates", async () => {
    const api = createApi({
      listRelationshipCandidates: vi.fn(
        async (): Promise<RelationshipCandidateSummary[]> => [reviewCandidate],
      ),
      listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => [
        reviewItem,
        linkedReviewItem,
      ]),
    });
    await mount(api, "review");

    await act(async () => {
      buttons("Review details")[0]!.click();
      await settle();
    });

    expect(api.getReviewDetail).toHaveBeenCalledWith("review-1");
    expect(api.listRelationshipCandidates).toHaveBeenCalledWith("review-1", 1);
    expect(container.textContent).toContain("July statement.pdf");
    expect(container.textContent).toContain("Card repayment");
    expect(container.textContent).toContain("Looks related");
    expect(container.textContent).toContain("DBS Visa Card");
    expect(button("Accept link")).toBeDefined();
  });

  it("saves only the edited fields and reopens the queue at the new record version", async () => {
    const updatedItem: ReviewItemSummary = {
      ...reviewItem,
      amountValue: "600.00",
      recordVersion: 2,
      reviewItemId: "review-1:v2",
    };
    const api = createApi({
      editReviewRecord: vi.fn(async (): Promise<ReviewMutationOutcome> => ({
        reason: null,
        recordVersion: 2,
        reviewItemId: "review-1:v2",
        status: "updated",
      })),
      listReviewItems: vi.fn()
        .mockResolvedValueOnce([reviewItem, linkedReviewItem])
        .mockResolvedValue([updatedItem]),
    });
    await mount(api, "review");

    await act(async () => {
      buttons("Review details")[0]!.click();
      await settle();
    });
    await click("Edit record");
    await enterField("Amount", "600.00");
    await click("Save edit");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.editReviewRecord).toHaveBeenCalledWith(
      "review-1",
      1,
      { amountValue: "600.00" },
    );
    expect(container.textContent).toContain("Edit saved");
    expect(api.getReviewDetail).toHaveBeenCalledWith("review-1:v2");
    expect(container.textContent).toContain("SGD 600.00");
    expect(container.textContent).not.toContain("SGD 512.34");
  });

  it("blocks an invalid edit without calling the host", async () => {
    const api = createApi({
      listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => [
        reviewItem,
        linkedReviewItem,
      ]),
    });
    await mount(api, "review");

    await act(async () => {
      buttons("Review details")[0]!.click();
      await settle();
    });
    await click("Edit record");
    await enterField("Amount", "12.3.4");
    await click("Save edit");

    expect(container.textContent).toContain(
      "Amount must be a positive decimal, such as 128.50.",
    );
    expect(api.editReviewRecord).not.toHaveBeenCalled();
    expect(button("Save edit")).toBeDefined();
  });

  it("removes a record after a two-step confirmation and empties the queue", async () => {
    const api = createApi({
      listReviewItems: vi.fn()
        .mockResolvedValueOnce([reviewItem])
        .mockResolvedValue([]),
    });
    await mount(api, "review");

    await click("Review details");
    await click("Remove record");
    await click("Confirm remove");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.removeReviewRecord).toHaveBeenCalledWith("review-1", 1);
    expect(container.textContent).toContain("Record removed");
    expect(container.textContent).toContain("Nothing needs your check");
  });

  it("accepts a suggested relationship, selects both records, and shows the Linked notice", async () => {
    const api = createApi({
      listRelationshipCandidates: vi.fn(
        async (): Promise<RelationshipCandidateSummary[]> => [reviewCandidate],
      ),
      listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => [
        reviewItem,
        linkedReviewItem,
      ]),
    });
    await mount(api, "review");

    await act(async () => {
      buttons("Review details")[0]!.click();
      await settle();
    });
    await click("Accept link");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.acceptReviewRelationship).toHaveBeenCalledWith(
      "review-1",
      1,
      "record-2",
      1,
    );
    expect(container.textContent).toContain("Linked");
    const checkboxes = [...container.querySelectorAll<HTMLInputElement>(
      "input[type='checkbox']",
    )];
    expect(checkboxes).toHaveLength(2);
    for (const checkbox of checkboxes) {
      expect(checkbox.checked).toBe(true);
    }
  });

  it("commits the selected batch, polls the job, and reports the committed outcome", async () => {
    const committedJob: ReviewJobSummary = {
      createdAt: "2026-07-25 09:00:00",
      finishedAt: "2026-07-25 09:00:01",
      jobId: "job-1",
      outcomes: [{
        reason: null,
        recordIds: ["record-1", "record-2"],
        status: "committed",
      }],
      status: "succeeded",
    };
    const api = createApi({
      getReviewJob: vi.fn(async (): Promise<ReviewJobSummary | null> => committedJob),
      listReviewItems: vi.fn()
        .mockResolvedValueOnce([reviewItem, linkedReviewItem])
        .mockResolvedValue([]),
    });
    await mount(api, "review");

    vi.useFakeTimers();
    await click("Select all");
    await click("Add selected (2)");

    expect(api.enqueueCommitReviewBatch).toHaveBeenCalledWith([
      "review-1",
      "review-2",
    ]);
    expect(container.textContent).toContain("Adding your records…");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(600);
      await settle();
      await settle();
    });

    expect(api.getReviewJob).toHaveBeenCalledWith("job-1");
    expect(container.textContent).toContain("Added 2 records.");
    expect(container.textContent).toContain("Nothing needs your check");
  });

  it("shows a stale-change notice and reloads the queue when a save conflicts", async () => {
    const api = createApi({
      editReviewRecord: vi.fn(async (): Promise<ReviewMutationOutcome> => ({
        reason: "stale_review_item",
        recordVersion: null,
        reviewItemId: null,
        status: "conflict",
      })),
      listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => [
        reviewItem,
        linkedReviewItem,
      ]),
    });
    await mount(api, "review");

    await act(async () => {
      buttons("Review details")[0]!.click();
      await settle();
    });
    await click("Edit record");
    await enterField("Amount", "600.00");
    await click("Save edit");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("Couldn’t apply that change");
    expect(container.textContent).toContain(
      "This item changed while you worked. CanCan reloaded the latest version.",
    );
    expect(api.listReviewItems).toHaveBeenCalledTimes(2);
  });

  it("undoes the last ledger step from Overview and reloads activity", async () => {
    const api = createApi({
      listRecentActivity: vi.fn(async (): Promise<RecentActivitySummary[]> => [{
        canUndo: true,
        eventDate: "2026-07-25",
        eventId: "event-1",
        eventType: "purchase",
        sourceLabels: ["DBS"],
        spending: true,
      }]),
    });
    await mount(api);

    await click("Undo");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.undoCommittedEvent).toHaveBeenCalledWith("event-1");
    expect(container.textContent).toContain("Undone");
    expect(api.listRecentActivity).toHaveBeenCalledTimes(2);
  });

  it("clears finance and review data when the native host reports a lock", async () => {
    let notifyLocked: (() => void) | undefined;
    const listReviewItems = vi.fn(
      async (): Promise<ReviewItemSummary[]> => [reviewItem, linkedReviewItem],
    );
    const api = createApi({
      listReviewItems,
      onVaultLocked: vi.fn(async (handler) => {
        notifyLocked = handler;
        return () => undefined;
      }),
    });
    await mount(api);

    expect(container.textContent).toContain("Money Overview");
    expect(navItem("Review").textContent).toContain("2");

    await act(async () => {
      notifyLocked?.();
      await settle();
    });

    expect(container.textContent).toContain("Unlock your Vault");
    expect(container.textContent).not.toContain("Money Overview");
    expect(container.textContent).not.toContain("Looks like a card repayment");

    listReviewItems.mockClear();
    await enterPassword("vault-password");
    await click("Unlock Vault");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("Money Overview");
    expect(navItem("Review").textContent).toContain("2");
    expect(listReviewItems).toHaveBeenCalled();
  });

  it("invalidates an in-flight review edit when focus reconciliation finds the Vault locked", async () => {
    const edit = deferred<ReviewMutationOutcome>();
    const api = createApi({
      editReviewRecord: vi.fn(() => edit.promise),
      listReviewItems: vi.fn(async (): Promise<ReviewItemSummary[]> => [
        reviewItem,
        linkedReviewItem,
      ]),
      vaultAccessStatus: vi.fn()
        .mockResolvedValueOnce({
          recoveryConfigured: false,
          rememberedOnThisMac: false,
          status: "unlocked",
        })
        .mockResolvedValueOnce({
          recoveryConfigured: false,
          rememberedOnThisMac: false,
          status: "locked",
        }),
    });
    await mount(api, "review");

    await act(async () => {
      buttons("Review details")[0]!.click();
      await settle();
    });
    await click("Edit record");
    await enterField("Amount", "600.00");
    await click("Save edit");

    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      await settle();
    });
    expect(container.textContent).toContain("Unlock your Vault");

    await act(async () => {
      edit.resolve({
        reason: null,
        recordVersion: 2,
        reviewItemId: null,
        status: "updated",
      });
      await settle();
    });
    await enterPassword("vault-password");
    await click("Unlock Vault");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("Your money, organized");
    expect(container.textContent).not.toContain("Edit saved");
  });
});
