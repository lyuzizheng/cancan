// @vitest-environment happy-dom

import { describe, expect, it, vi } from "vitest";

import type { TaskRow, Tasks } from "./command-contracts";
import {
  container,
  createApi,
  installAppHarness,
  mount,
} from "./test-support/app-harness";

installAppHarness();

const passwordRow: TaskRow = {
  consequence: "password_needed",
  destination: {
    documentId: "document-1",
    kind: "password",
    moneySourceId: "source-dbs",
  },
  group: "needs_action",
  rowKey: "task:password:document-1",
  timestamp: "2026-09-18 09:00:00",
  title: "June statement.pdf",
};

const readyRow: TaskRow = {
  consequence: "ready",
  destination: { documentId: "document-2", kind: "document" },
  group: "recently_completed",
  rowKey: "task:document:document-2",
  timestamp: "2026-09-18 08:00:00",
  title: "July statement.pdf",
};

/** The route the Command Center is on, from the nav's `aria-current`. */
function activeRoute(): string {
  const current = [
    ...container.querySelectorAll<HTMLButtonElement>(
      'nav[aria-label="Command Center"] button[aria-current="page"]',
    ),
  ];
  expect(current).toHaveLength(1);
  return current[0]!.textContent!.trim();
}

/** The Tasks group filter the full route is showing. */
function activeTaskFilter(): string {
  const pressed = container.querySelector<HTMLButtonElement>(
    'button[aria-pressed="true"]',
  );
  expect(pressed).not.toBeNull();
  return pressed!.textContent!.trim();
}

describe("background intake click-through", () => {
  it("opens the Tasks route on the clicked batch's row, once", async () => {
    const api = createApi({
      listTasks: vi.fn(async (): Promise<Tasks> => ({
        needsActionCount: 1,
        rows: [passwordRow, readyRow],
      })),
      takeBackgroundIntakeRoute: vi.fn(async () => passwordRow),
    });

    await mount(api);

    expect(api.takeBackgroundIntakeRoute).toHaveBeenCalledTimes(1);
    expect(activeRoute()).toContain("Tasks");
    // The row's own group is the open filter, so the row the banner named is
    // the one on screen.
    expect(activeTaskFilter()).toContain("Needs action");
    expect(container.textContent).toContain("June statement.pdf");
  });

  it("keeps the user's route when no click is pending", async () => {
    const api = createApi();
    await mount(api);

    expect(api.takeBackgroundIntakeRoute).toHaveBeenCalledTimes(1);
    expect(activeRoute()).toContain("Overview");
    expect(container.textContent).toContain("Your money, organized");
  });

  it("stays quiet when the pull cannot read a locked Vault", async () => {
    const api = createApi({
      takeBackgroundIntakeRoute: vi.fn(async () => {
        throw { code: "vault_locked" };
      }),
    });

    await mount(api);

    expect(activeRoute()).toContain("Overview");
    // The route stays pending host-side, so there is nothing to report.
    expect(container.textContent).not.toContain("Unlock your Vault to continue.");
  });

  it("lands on the group a completed batch's row lives in", async () => {
    const api = createApi({
      listTasks: vi.fn(async (): Promise<Tasks> => ({
        needsActionCount: 0,
        rows: [readyRow],
      })),
      takeBackgroundIntakeRoute: vi.fn(async () => readyRow),
    });

    await mount(api);

    expect(activeTaskFilter()).toContain("Recently completed");
    expect(container.textContent).toContain("July statement.pdf");
  });
});
