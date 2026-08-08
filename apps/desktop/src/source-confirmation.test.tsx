import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type {
  MoneySourceSummary,
  SourceConfirmationPrompt,
} from "./command-contracts";
import {
  SourceConfirmationCardList,
  type SourceConfirmationCardListProps,
} from "./source-confirmation";

const existingSources: MoneySourceSummary[] = [
  {
    displayName: "Synthetic Bank",
    moneySourceId: "source-synthetic",
    sourceType: "bank",
  },
];

const pendingPrompt: SourceConfirmationPrompt = {
  candidateId: "candidate-dbs",
  documentCount: 2,
  latestDocumentTitle: "June statement.pdf",
  providerKey: "dbs",
  scopeKind: "provider_singleton",
  status: "pending",
  version: 1,
};

const baseProps: SourceConfirmationCardListProps = {
  busyKey: null,
  existingSources,
  onConfirm: () => undefined,
  onKeepUnassigned: () => undefined,
  prompts: [],
};

function render(props: Partial<SourceConfirmationCardListProps> = {}): string {
  return renderToStaticMarkup(<SourceConfirmationCardList {...baseProps} {...props} />);
}

describe("SourceConfirmationCardList", () => {
  it("renders nothing without prompts", () => {
    expect(render()).toBe("");
  });

  it("shows only proven facts and the three confirmation actions for a pending prompt", () => {
    const html = render({ prompts: [pendingPrompt] });

    expect(html).toContain("New source detected: DBS");
    expect(html).toContain("June statement.pdf · 2 documents waiting");
    expect(html).toContain("no DBS Money Source exists yet");
    expect(html).toContain("Create source and continue");
    expect(html).toContain("Choose an existing source");
    expect(html).toContain("Keep unassigned");
    expect(html).not.toContain("candidate-dbs");
    expect(html).not.toContain("provider_singleton");
  });

  it("keeps the choose-existing chooser closed until requested", () => {
    const html = render({ prompts: [pendingPrompt] });

    expect(html).toContain('aria-expanded="false"');
    expect(html).not.toContain("Use Synthetic Bank");
  });

  it("omits the choose-existing action when no Money Sources exist", () => {
    const html = render({ existingSources: [], prompts: [pendingPrompt] });

    expect(html).not.toContain("Choose an existing source");
    expect(html).toContain("Create source and continue");
  });

  it("presents a kept-unassigned prompt as parked with correction actions", () => {
    const html = render({
      prompts: [{ ...pendingPrompt, status: "kept_unassigned" }],
    });

    expect(html).toContain("Kept unassigned: DBS");
    expect(html).toContain("stays parked and unassigned");
    expect(html).toContain("Create source and continue");
    expect(html).toContain("Choose an existing source");
    expect(html).not.toContain(">Keep unassigned<");
  });

  it("falls back to a cleaned-up provider label for unknown providers", () => {
    const html = render({
      prompts: [{ ...pendingPrompt, providerKey: "first_tiger_bank" }],
    });

    expect(html).toContain("New source detected: First tiger bank");
  });

  it("disables actions and shows progress while a confirmation is in flight", () => {
    const html = render({
      busyKey: "source-confirm:candidate-dbs",
      prompts: [pendingPrompt],
    });

    expect(html).toContain("Routing…");
    const disabledCount = (html.match(/disabled=""/g) ?? []).length;
    expect(disabledCount).toBeGreaterThanOrEqual(3);
  });
});
