import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { AccountConfirmationPrompt } from "./command-contracts";
import { AttentionSection, type AttentionSectionProps } from "./attention";

const prompt: AccountConfirmationPrompt = {
  candidateAccounts: [
    {
      accountId: "account-dbs",
      accountType: "deposit_account",
      currency: "SGD",
      displayName: "DBS Multiplier Account",
      maskedIdentifier: "•••• 1234",
    },
  ],
  dismissedAccounts: [
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
  proposalVersion: "proposal-version-1",
};

const baseProps: AttentionSectionProps = {
  accountPrompts: [],
  attentionBusyKey: null,
  onDecideAccounts: () => undefined,
  onRestoreAccount: () => undefined,
};

function render(props: Partial<AttentionSectionProps> = {}): string {
  return renderToStaticMarkup(<AttentionSection {...baseProps} {...props} />);
}

describe("AttentionSection", () => {
  it("renders nothing without candidate or dismissed accounts", () => {
    expect(render()).toBe("");
  });

  it("renders per-account choices and an explicit restore action", () => {
    const html = render({ accountPrompts: [prompt] });

    expect(html).toContain("CanCan found new accounts in your DBS statements");
    expect(html).toContain("DBS Multiplier Account");
    expect(html).toContain("Bank account · SGD · •••• 1234");
    expect(html).toContain("Save choices");
    expect(html).toContain("Wise USD Balance · dismissed");
    expect(html).toContain("Restore");
  });

  it("disables actions while an account decision is in flight", () => {
    const html = render({
      accountPrompts: [prompt],
      attentionBusyKey: "account:source-dbs",
    });

    expect(html).toContain("Saving…");
    expect((html.match(/disabled=""/g) ?? []).length).toBeGreaterThan(1);
  });

  it("does not show a zero attention badge for restore-only accounts", () => {
    const html = render({
      accountPrompts: [{
        ...prompt,
        candidateAccounts: [],
      }],
    });

    expect(html).not.toContain('aria-label="0 to check"');
    expect(html).toContain("Wise USD Balance · dismissed");
  });
});
