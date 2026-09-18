// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type { TaskRow, Tasks } from "./command-contracts";
import {
  container,
  createApi,
  deferred,
  installAppHarness,
  mount,
  settle,
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

  it("opens the row when a click reaches an already-live window", async () => {
    // The window was never destroyed, so there is no unlock to pull the route
    // on: the host's click event is the only thing that can run the pull.
    let clickArrived = () => undefined;
    const pending: TaskRow[] = [];
    const api = createApi({
      listTasks: vi.fn(async (): Promise<Tasks> => ({
        needsActionCount: 1,
        rows: [passwordRow],
      })),
      onBackgroundIntakeRoute: vi.fn(async (handler) => {
        clickArrived = handler;
        return () => undefined;
      }),
      takeBackgroundIntakeRoute: vi.fn(async () => pending.shift() ?? null),
    });

    await mount(api);
    // The mount pull found nothing and left the user on Overview.
    expect(api.takeBackgroundIntakeRoute).toHaveBeenCalledTimes(1);
    expect(activeRoute()).toContain("Overview");

    pending.push(passwordRow);
    await act(async () => {
      clickArrived();
      await settle();
    });

    expect(api.takeBackgroundIntakeRoute).toHaveBeenCalledTimes(2);
    expect(activeRoute()).toContain("Tasks");
    expect(activeTaskFilter()).toContain("Needs action");
    expect(container.textContent).toContain("June statement.pdf");
  });

  it("keeps the row when a second click lands during its pull", async () => {
    // The host consumes a route once, so the pull that already has it owns it:
    // a newer pull reads nothing, and dropping the in-flight answer would make
    // the user click again to open a row the app already took.
    let clickArrived = () => undefined;
    const inFlight = deferred<TaskRow | null>();
    const api = createApi({
      listTasks: vi.fn(async (): Promise<Tasks> => ({
        needsActionCount: 1,
        rows: [passwordRow],
      })),
      onBackgroundIntakeRoute: vi.fn(async (handler) => {
        clickArrived = handler;
        return () => undefined;
      }),
      takeBackgroundIntakeRoute: vi
        .fn<() => Promise<TaskRow | null>>()
        .mockResolvedValueOnce(null)
        .mockReturnValueOnce(inFlight.promise)
        .mockResolvedValue(null),
    });

    await mount(api);

    await act(async () => {
      clickArrived();
      await settle();
    });
    // The second click lands while that pull is still in flight.
    await act(async () => {
      clickArrived();
      await settle();
    });
    expect(api.takeBackgroundIntakeRoute).toHaveBeenCalledTimes(3);

    await act(async () => {
      inFlight.resolve(passwordRow);
      await settle();
    });

    expect(activeRoute()).toContain("Tasks");
    expect(activeTaskFilter()).toContain("Needs action");
    expect(container.textContent).toContain("June statement.pdf");
  });
});
