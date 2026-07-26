import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type {
  AccountConfirmationPrompt,
  StatementCoveragePrompt,
} from "./command-contracts";
import {
  AttentionSection,
  coverageKey,
  type AttentionSectionProps,
} from "./attention";

const coverageConfirmed: StatementCoveragePrompt = {
  accountId: "account-dbs",
  documentType: "bank_statement",
  moneySourceId: "source-dbs",
  statementPeriodFrom: "2026-06-01",
  statementPeriodTo: "2026-06-30",
  status: "confirmed_missing",
};

const coverageLikely: StatementCoveragePrompt = {
  accountId: "account-card",
  documentType: "credit_card_statement",
  moneySourceId: "source-dbs",
  statementPeriodFrom: "2026-05-01",
  statementPeriodTo: "2026-05-31",
  status: "likely_missing",
};

const accountPrompt: AccountConfirmationPrompt = {
  candidateAccounts: [
    {
      accountId: "account-dbs",
      accountType: "deposit_account",
      currency: "SGD",
      displayName: "DBS Multiplier Account",
      maskedIdentifier: "•••• 1234",
    },
    {
      accountId: "account-wise",
      accountType: "deposit_account",
      currency: null,
      displayName: "Wise USD Balance",
      maskedIdentifier: null,
    },
  ],
  displayName: "DBS",
  moneySourceId: "source-dbs",
};

const baseProps: AttentionSectionProps = {
  accountPrompts: [],
  attentionBusyKey: null,
  coveragePrompts: [],
  moneySources: [{ displayName: "DBS", moneySourceId: "source-dbs", sourceType: "bank" }],
  onAddFile: () => undefined,
  onCancelRemind: () => undefined,
  onChangeRemindDate: () => undefined,
  onConfirmAccounts: () => undefined,
  onNotExpected: () => undefined,
  onSaveRemind: () => undefined,
  onStartRemind: () => undefined,
  remind: null,
};

function render(props: Partial<AttentionSectionProps> = {}): string {
  return renderToStaticMarkup(<AttentionSection {...baseProps} {...props} />);
}

describe("coverageKey", () => {
  it("identifies a prompt by source, account, type, and period", () => {
    expect(coverageKey(coverageConfirmed)).toBe(
      "source-dbs|account-dbs|bank_statement|2026-06-01|2026-06-30",
    );
  });
});

describe("AttentionSection", () => {
  it("renders nothing when there are no prompts", () => {
    expect(render()).toBe("");
  });

  it("distinguishes confirmed-missing and likely-missing titles", () => {
    const html = render({ coveragePrompts: [coverageConfirmed, coverageLikely] });
    expect(html).toContain("DBS June 2026 bank statement is missing");
    expect(html).toContain("DBS May 2026 card statement may be missing");
    expect(html).toContain("Needs attention");
    expect(html).toContain("2");
  });

  it("offers add, not-expected, and remind-later actions per coverage card", () => {
    const html = render({ coveragePrompts: [coverageConfirmed] });
    expect(html).toContain("Add file");
    expect(html).toContain("Not expected");
    expect(html).toContain("Remind later");
  });

  it("falls back to a generic source name when the source is unknown", () => {
    const html = render({ coveragePrompts: [coverageConfirmed], moneySources: [] });
    expect(html).toContain("A money source June 2026 bank statement is missing");
  });

  it("lists candidate accounts with their types for confirmation", () => {
    const html = render({ accountPrompts: [accountPrompt] });
    expect(html).toContain("CanCan found new accounts in your DBS statements");
    expect(html).toContain("DBS Multiplier Account");
    expect(html).toContain("Bank account · SGD · •••• 1234");
    expect(html).toContain("Wise USD Balance");
    expect(html).toContain("These are mine");
  });

  it("shows the remind form only on the matching card", () => {
    const html = render({
      coveragePrompts: [coverageConfirmed, coverageLikely],
      remind: {
        date: "2026-08-01",
        error: null,
        key: coverageKey(coverageLikely),
        saving: false,
      },
    });
    expect(html).toContain("aria-label=\"Remind after\"");
    expect(html).toContain("value=\"2026-08-01\"");
    expect(html).toContain("Save reminder");
    // The other card keeps its default actions.
    expect(html).toContain("Not expected");
  });

  it("surfaces remind validation errors accessibly", () => {
    const html = render({
      coveragePrompts: [coverageConfirmed],
      remind: {
        date: "2020-01-01",
        error: "Pick a future date.",
        key: coverageKey(coverageConfirmed),
        saving: false,
      },
    });
    expect(html).toContain("role=\"alert\"");
    expect(html).toContain("Pick a future date.");
  });

  it("disables actions while a decision is in flight", () => {
    const html = render({
      attentionBusyKey: coverageKey(coverageConfirmed),
      coveragePrompts: [coverageConfirmed],
    });
    expect(html).toContain("Saving…");
    expect((html.match(/disabled=""/g) ?? []).length).toBe(3);
  });
});
