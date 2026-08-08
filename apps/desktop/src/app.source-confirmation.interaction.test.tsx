// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type { TaskFilter, TaskRow, Tasks } from "./command-contracts";
import {
  button,
  buttons,
  click,
  container,
  createApi,
  installAppHarness,
  moneySource,
  mount,
  navItem,
  otherMoneySource,
  settle,
  sourceConfirmationPrompt,
  sourceDocument,
} from "./test-support/app-harness";

installAppHarness();

const sourceConfirmationRow: TaskRow = {
  consequence: "new_source_detected",
  destination: {
    kind: "source_confirmation",
    moneySourceCandidateId: sourceConfirmationPrompt.candidateId,
  },
  group: "needs_action",
  rowKey: "task:new_source:candidate-dbs",
  timestamp: "2026-07-19 09:00",
  title: "New source detected: DBS",
};

function tasksWith(...rows: TaskRow[]) {
  return vi.fn(async (): Promise<Tasks> => ({
    needsActionCount: rows.filter((row) => row.group === "needs_action").length,
    rows,
  }));
}

function focusedDialog(): HTMLElement {
  const dialog = [...document.body.querySelectorAll<HTMLElement>('[role="dialog"]')]
    .find((element) => !container.contains(element));
  expect(dialog).toBeDefined();
  return dialog!;
}

function dialogButton(label: string): HTMLButtonElement {
  const matches = [...focusedDialog().querySelectorAll("button")].filter(
    (element) => element.textContent?.trim() === label,
  );
  expect(matches).toHaveLength(1);
  return matches[0]!;
}

async function clickDialogButton(label: string) {
  await act(async () => {
    dialogButton(label).click();
    await settle();
  });
}

async function closeFocusedDialog() {
  const close = focusedDialog().querySelector<HTMLButtonElement>(
    'button[aria-label="Close"]',
  );
  expect(close).not.toBeNull();
  await act(async () => {
    close!.click();
    await settle();
  });
}

async function openFocusedDialog(rowTitle = sourceConfirmationRow.title) {
  const row = [...container.querySelectorAll("button")].find(
    (element) => element.textContent?.includes(rowTitle),
  );
  expect(row).toBeDefined();
  await act(async () => {
    row!.click();
    await settle();
  });
}

async function changeSelect(select: HTMLSelectElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    HTMLSelectElement.prototype,
    "value",
  )?.set;
  expect(setter).toBeDefined();
  await act(async () => {
    setter!.call(select, value);
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await settle();
  });
}

describe("App source confirmation", () => {
  it("surfaces a detected source as a task row and confirms it from the focused dialog", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [sourceConfirmationPrompt]),
      listTasks: tasksWith(sourceConfirmationRow),
    });
    await mount(api);

    expect(container.textContent).toContain("New source detected: DBS");
    expect(navItem("Tasks").textContent).toContain("1");
    expect(container.textContent).not.toContain("June statement.pdf · 2 documents waiting");

    await openFocusedDialog();

    expect(focusedDialog().textContent).toContain("New source detected: DBS");
    expect(focusedDialog().textContent).toContain("June statement.pdf · 2 documents waiting");

    await clickDialogButton("Create source and continue");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.confirmSourceCandidate).toHaveBeenCalledWith(
      "candidate-dbs",
      1,
      "DBS",
      "bank",
    );
    expect(container.textContent).toContain("DBS is ready");
    expect(api.listSourceConfirmationPrompts).toHaveBeenCalledTimes(2);
  });

  it("keeps a candidate unassigned and parks its evidence", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [sourceConfirmationPrompt]),
      listTasks: tasksWith(sourceConfirmationRow),
    });
    await mount(api);

    await openFocusedDialog();
    await clickDialogButton("Keep unassigned");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.parkSourceCandidate).toHaveBeenCalledWith("candidate-dbs", 1);
    expect(container.textContent).toContain("Kept unassigned");
    expect(api.listSourceConfirmationPrompts).toHaveBeenCalledTimes(2);
  });

  it("routes evidence to a chosen existing source", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource, otherMoneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [sourceConfirmationPrompt]),
      listTasks: tasksWith(sourceConfirmationRow),
    });
    await mount(api);

    await openFocusedDialog();
    await clickDialogButton("Choose an existing source");
    const select = focusedDialog().querySelector<HTMLSelectElement>(
      'select[aria-label="Existing Money Source for DBS"]',
    );
    expect(select).not.toBeNull();
    await changeSelect(select!, otherMoneySource.moneySourceId);

    await clickDialogButton("Use Another Bank");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.confirmSourceCandidate).toHaveBeenCalledWith(
      "candidate-dbs",
      1,
      "Another Bank",
      "bank",
    );
  });

  it("opens the waiting document from the focused dialog", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [sourceConfirmationPrompt]),
      listTasks: tasksWith(sourceConfirmationRow),
      listUnassignedSourceDocuments: vi.fn(async () => [sourceDocument()]),
    });
    await mount(api);

    await openFocusedDialog();
    await clickDialogButton("View document");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.renderSourceDocumentPage).toHaveBeenCalledWith("document-1", 1);
    expect(container.querySelector('[role="dialog"]')).not.toBeNull();
  });

  it("closes the focused dialog without deciding", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [sourceConfirmationPrompt]),
      listTasks: tasksWith(sourceConfirmationRow),
    });
    await mount(api);

    await openFocusedDialog();
    await closeFocusedDialog();

    expect(
      [...document.body.querySelectorAll('[role="dialog"]')].filter(
        (element) => !container.contains(element),
      ),
    ).toHaveLength(0);
  });

  it("opens the unlock modal from a password task row", async () => {
    const passwordRow: TaskRow = {
      consequence: "password_needed",
      destination: { documentId: "document-1", kind: "password", moneySourceId: "money-source-1" },
      group: "needs_action",
      rowKey: "task:password:document-1",
      timestamp: "2026-07-19 09:00",
      title: "June statement.pdf",
    };
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listTasks: tasksWith(passwordRow),
      listUnassignedSourceDocuments: vi.fn(async () => [sourceDocument()]),
    });
    await mount(api);

    await openFocusedDialog("June statement.pdf");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("Unlock June statement.pdf");
    expect(api.listStatementPasswordSources).toHaveBeenCalled();
  });

  it("switches task groups on the full Tasks route", async () => {
    const parkedRow: TaskRow = {
      consequence: "password_parked",
      destination: { documentId: "document-4", kind: "document" },
      group: "parked",
      rowKey: "task:parked:document-4",
      timestamp: "2026-07-10 08:00",
      title: "Old export.csv",
    };
    const api = createApi({
      listTasks: vi.fn(async (filter: TaskFilter): Promise<Tasks> => filter === "command_center"
        ? { needsActionCount: 1, rows: [sourceConfirmationRow] }
        : { needsActionCount: 1, rows: [sourceConfirmationRow, parkedRow] }),
    });
    await mount(api);

    await click("View all");

    expect(navItem("Tasks").getAttribute("aria-current")).toBe("page");
    expect(container.textContent).toContain("New source detected: DBS");
    expect(container.textContent).not.toContain("Old export.csv");

    const parkedChip = [...container.querySelectorAll<HTMLButtonElement>("button[aria-pressed]")]
      .find((element) => element.textContent?.startsWith("Parked"));
    expect(parkedChip).toBeDefined();
    await act(async () => {
      parkedChip!.click();
      await settle();
    });

    expect(container.textContent).toContain("Old export.csv");
    expect(container.textContent).not.toContain("New source detected: DBS");
  });

  it("surfaces a kept-unassigned candidate on Sources and hides it from the overview", async () => {
    const parkedPrompt = { ...sourceConfirmationPrompt, status: "kept_unassigned" as const };
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [parkedPrompt]),
    });
    await mount(api);

    expect(container.textContent).not.toContain("Kept unassigned: DBS");

    await mount(api, "sources");

    expect(container.textContent).toContain("Kept unassigned: DBS");
    expect(container.textContent).toContain("stays parked and unassigned");
    expect(buttons("Create source and continue")).toHaveLength(1);
    expect(button("Create source and continue")).toBeDefined();

    await click("Create source and continue");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(api.confirmSourceCandidate).toHaveBeenCalledWith(
      "candidate-dbs",
      1,
      "DBS",
      "bank",
    );
  });

  it("shows the host error and reloads when confirmation fails", async () => {
    const api = createApi({
      confirmSourceCandidate: vi.fn(async () => {
        throw { code: "source_confirmation_unavailable" };
      }),
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [sourceConfirmationPrompt]),
      listTasks: tasksWith(sourceConfirmationRow),
    });
    await mount(api);

    await openFocusedDialog();
    await clickDialogButton("Create source and continue");
    await act(async () => {
      await settle();
      await settle();
    });

    expect(container.textContent).toContain("Couldn’t confirm that source");
    expect(container.textContent).toContain(
      "CanCan couldn’t update that money source confirmation. Try again.",
    );
    expect(api.listSourceConfirmationPrompts).toHaveBeenCalledTimes(2);
  });
});
