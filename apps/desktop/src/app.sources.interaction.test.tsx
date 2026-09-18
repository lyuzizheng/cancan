// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type {
  MoneySourceDetail,
  MoneySourceSummary,
  SupportedMoneySourceProviderSummary,
} from "./command-contracts";
import {
  availableDocument,
  bodyDialog,
  button,
  click,
  clickDialogButton,
  container,
  createApi,
  dialogButton,
  dialogInput,
  enterDialogInput,
  installAppHarness,
  moneySource,
  mount,
  noBodyDialog,
  otherMoneySource,
  settle,
  sourceDocument,
} from "./test-support/app-harness";

installAppHarness();

const supportedProviders: SupportedMoneySourceProviderSummary[] = [
  {
    configuredMoneySourceId: moneySource.moneySourceId,
    displayName: "DBS",
    providerKey: "dbs",
    sourceType: "bank",
  },
  {
    configuredMoneySourceId: null,
    displayName: "HSBC",
    providerKey: "hsbc",
    sourceType: "bank",
  },
];

function sourceDetail(
  overrides: Partial<MoneySourceDetail> = {},
): MoneySourceDetail {
  return {
    actions: { canEnterStatementPassword: false, hasSavedStatementPassword: false },
    displayName: moneySource.displayName,
    documents: [],
    moneySourceId: moneySource.moneySourceId,
    providerKey: moneySource.providerKey,
    sourceType: "bank",
    ...overrides,
  };
}

async function openSource(displayName = moneySource.displayName) {
  await act(async () => {
    container.querySelector<HTMLButtonElement>(
      `[aria-label="Open ${displayName}"]`,
    )!.click();
    await settle();
  });
}

async function backToSources() {
  await act(async () => {
    container.querySelector<HTMLButtonElement>(
      `[aria-label="Back to all Money Sources"]`,
    )!.click();
    await settle();
  });
}

async function chooseProvider(providerKey: string) {
  await act(async () => {
    bodyDialog().querySelector<HTMLInputElement>(
      `input[name="money-source-provider"][value="${providerKey}"]`,
    )!.click();
    await settle();
  });
}

describe("Money Source management", () => {
  it("creates a source for an unconfigured provider and opens its detail", async () => {
    const created: MoneySourceSummary = {
      displayName: "HSBC",
      moneySourceId: "source-hsbc",
      providerKey: "hsbc",
      sourceType: "bank",
    };
    const api = createApi({
      createMoneySource: vi.fn(async () => created),
      getMoneySourceDetail: vi.fn(async (moneySourceId: string) => sourceDetail({
        displayName: created.displayName,
        moneySourceId,
        providerKey: created.providerKey,
      })),
      listMoneySources: vi.fn(async () => [moneySource, created]),
      listSupportedMoneySourceProviders: vi.fn(async () => supportedProviders),
    });

    await mount(api, "sources");
    await click("Add source");

    expect(api.listSupportedMoneySourceProviders).toHaveBeenCalledTimes(1);
    expect(dialogButton("Add source").disabled).toBe(true);
    const configuredRadio = dialogInput(
      `input[name="money-source-provider"][value="dbs"]`,
    );
    expect(configuredRadio.disabled).toBe(true);
    expect(bodyDialog().textContent).toContain("Already configured");

    await chooseProvider("hsbc");
    await enterDialogInput("#money-source-name", "  HSBC Everyday  ");
    await clickDialogButton("Add source");

    expect(api.createMoneySource).toHaveBeenCalledWith("hsbc", "HSBC Everyday");
    noBodyDialog();
    expect(container.textContent).toContain("Money Source added");
    expect(container.textContent).toContain("HSBC");
    expect(api.getMoneySourceDetail).toHaveBeenCalledWith("source-hsbc");
  });

  it("sends a null display name when the create form is left blank", async () => {
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSupportedMoneySourceProviders: vi.fn(async () => supportedProviders),
    });

    await mount(api, "sources");
    await click("Add source");
    await chooseProvider("hsbc");
    await clickDialogButton("Add source");

    expect(api.createMoneySource).toHaveBeenCalledWith("hsbc", null);
  });

  it("keeps the create dialog open with its own error line when the command fails", async () => {
    const api = createApi({
      createMoneySource: vi.fn(async () => {
        throw { code: "source_provider_already_configured" };
      }),
      listMoneySources: vi.fn(async () => [moneySource]),
      listSupportedMoneySourceProviders: vi.fn(async () => supportedProviders),
    });

    await mount(api, "sources");
    await click("Add source");
    await chooseProvider("hsbc");
    await clickDialogButton("Add source");

    expect(bodyDialog().textContent).toContain(
      "That provider already has a Money Source. Open it from Sources instead.",
    );
    expect(dialogButton("Add source").disabled).toBe(false);
  });

  it("retries the provider catalog inside the create dialog after a failed load", async () => {
    const listSupportedMoneySourceProviders = vi
      .fn<() => Promise<SupportedMoneySourceProviderSummary[]>>()
      .mockRejectedValueOnce(new Error("catalog unavailable"))
      .mockResolvedValueOnce(supportedProviders);
    const api = createApi({
      listMoneySources: vi.fn(async () => [moneySource]),
      listSupportedMoneySourceProviders,
    });

    await mount(api, "sources");
    await click("Add source");

    expect(bodyDialog().textContent).toContain("Providers unavailable");
    await clickDialogButton("Try again");

    expect(listSupportedMoneySourceProviders).toHaveBeenCalledTimes(2);
    expect(bodyDialog().textContent).toContain("HSBC");
    expect(dialogButton("Add source")).toBeDefined();
  });

  it("renames a source from its detail view and shows the new name", async () => {
    const renamed: MoneySourceSummary = {
      ...moneySource,
      displayName: "Everyday Bank",
    };
    const listMoneySources = vi
      .fn<() => Promise<MoneySourceSummary[]>>()
      .mockResolvedValueOnce([moneySource])
      .mockResolvedValue([renamed]);
    const api = createApi({
      editMoneySource: vi.fn(async () => renamed),
      getMoneySourceDetail: vi.fn(async () => sourceDetail({
        displayName: renamed.displayName,
      })),
      listMoneySources,
    });

    await mount(api, "sources");
    await openSource();
    await click("Rename");

    expect(dialogInput("#money-source-rename").value).toBe(moneySource.displayName);
    await enterDialogInput("#money-source-rename", "Everyday Bank");
    await clickDialogButton("Rename source");

    expect(api.editMoneySource).toHaveBeenCalledWith(
      moneySource.moneySourceId,
      "Everyday Bank",
    );
    noBodyDialog();
    expect(container.textContent).toContain("Money Source renamed");
    expect(container.textContent).toContain("Everyday Bank");
  });

  it("navigates list to detail and back, and shows the source-specific empty state", async () => {
    const api = createApi({
      getMoneySourceDetail: vi.fn(async () => sourceDetail()),
      listMoneySources: vi.fn(async () => [moneySource, otherMoneySource]),
    });

    await mount(api, "sources");
    expect(container.textContent).toContain("Add source");

    await openSource();
    expect(container.textContent).toContain(
      `No ${moneySource.displayName} documents yet. Add a file or set up CanCan Inbox.`,
    );
    expect(container.textContent).toContain("No statement password is saved");
    expect(container.textContent).not.toContain("Needs attention");

    await backToSources();
    expect(container.textContent).toContain(otherMoneySource.displayName);
    expect(container.textContent).toContain("Needs attention");
  });

  it("offers the statement-password actions the detail reports", async () => {
    const waitingDocument = sourceDocument({
      attentionReason: "password_required",
      documentStatus: "needs_attention",
    });
    const api = createApi({
      getMoneySourceDetail: vi.fn(async () => sourceDetail({
        actions: {
          canEnterStatementPassword: true,
          hasSavedStatementPassword: true,
        },
        documents: [waitingDocument],
      })),
      listMoneySources: vi.fn(async () => [moneySource]),
      listStatementPasswordSources: vi.fn(async () => [
        {
          displayName: moneySource.displayName,
          hasSavedPassword: true,
          moneySourceId: moneySource.moneySourceId,
        },
      ]),
    });

    await mount(api, "sources");
    await openSource();

    expect(container.textContent).toContain(
      "A statement is waiting for its password",
    );
    await click("Enter password");
    expect(bodyDialog().textContent).toContain(
      `Unlock ${waitingDocument.originalFilename}`,
    );
    await clickDialogButton("Close");
    noBodyDialog();

    await click("Remove saved password");
    expect(bodyDialog().textContent).toContain(
      `Remove the saved password for ${moneySource.displayName}?`,
    );
    await clickDialogButton("Remove saved password");

    expect(api.removeStatementPassword).toHaveBeenCalledWith(
      moneySource.moneySourceId,
    );
    noBodyDialog();
    expect(container.textContent).toContain("Saved statement password removed");
  });

  it("renders the evidence list with document states inside the detail view", async () => {
    const api = createApi({
      getMoneySourceDetail: vi.fn(async () => sourceDetail({
        documents: [
          availableDocument,
          sourceDocument({
            documentId: "document-deleted",
            documentStatus: "file_deleted",
            fileState: "deleted",
            originalFilename: "old-export.csv",
          }),
        ],
      })),
      listMoneySources: vi.fn(async () => [moneySource]),
    });

    await mount(api, "sources");
    await openSource();

    expect(container.textContent).toContain(availableDocument.originalFilename);
    expect(container.textContent).toContain("File deleted");
    expect(container.textContent).toContain("View unavailable");
    expect(container.textContent).toContain("old-export.csv");
    expect(button("View document")).toBeDefined();
  });
});
