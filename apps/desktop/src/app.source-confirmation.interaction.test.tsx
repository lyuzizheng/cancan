// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import {
  button,
  buttons,
  click,
  container,
  createApi,
  installAppHarness,
  moneySource,
  mount,
  otherMoneySource,
  settle,
  sourceConfirmationPrompt,
} from "./test-support/app-harness";

installAppHarness();

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
  it("creates the detected source from the overview card and reloads", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [sourceConfirmationPrompt]),
    });
    await mount(api);

    expect(container.textContent).toContain("New source detected: DBS");
    expect(container.textContent).toContain("June statement.pdf · 2 documents waiting");
    expect(
      container.querySelector(".attention-count")?.getAttribute("aria-label"),
    ).toBe("1 to check");

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
    expect(container.textContent).toContain("DBS is ready");
    expect(api.listSourceConfirmationPrompts).toHaveBeenCalledTimes(2);
  });

  it("keeps a candidate unassigned and parks its evidence", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSourceConfirmationPrompts: vi.fn(async () => [sourceConfirmationPrompt]),
    });
    await mount(api);

    await click("Keep unassigned");
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
    });
    await mount(api);

    await click("Choose an existing source");
    const select = container.querySelector<HTMLSelectElement>(
      'select[aria-label="Existing Money Source for DBS"]',
    );
    expect(select).not.toBeNull();
    await changeSelect(select!, otherMoneySource.moneySourceId);

    await click("Use Another Bank");
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
    });
    await mount(api);

    await click("Create source and continue");
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
