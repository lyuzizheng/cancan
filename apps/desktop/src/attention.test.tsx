import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AccountConfirmationCard } from "./attention";
import type { AccountConfirmationPrompt } from "./command-contracts";

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

function render(
  overrides: Partial<Parameters<typeof AccountConfirmationCard>[0]> = {},
): string {
  return renderToStaticMarkup(
    <ul>
      <AccountConfirmationCard
        busy={false}
        deciding={false}
        onDecide={() => undefined}
        onRestore={() => undefined}
        prompt={prompt}
        restoringAccountId={null}
        {...overrides}
      />
    </ul>,
  );
}

describe("AccountConfirmationCard", () => {
  it("renders per-account choices and an explicit restore action", () => {
    const html = render();

    expect(html).toContain("CanCan found new accounts in your DBS statements");
    expect(html).toContain("DBS Multiplier Account");
    expect(html).toContain("Bank account · SGD · •••• 1234");
    expect(html).toContain("Save choices");
    expect(html).toContain("Wise USD Balance · dismissed");
    expect(html).toContain("Restore");
  });

  it("disables actions while an account decision is in flight", () => {
    const html = render({ busy: true, deciding: true });

    expect(html).toContain("Saving…");
    expect((html.match(/disabled=""/g) ?? []).length).toBeGreaterThan(1);
  });

  it("shows restore-only accounts without candidate choices", () => {
    const html = render({
      prompt: {
        ...prompt,
        candidateAccounts: [],
      },
    });

    expect(html).not.toContain("Save choices");
    expect(html).toContain("Wise USD Balance · dismissed");
    expect(html).toContain("Restore");
  });

  it("marks the in-flight restore with progress text", () => {
    const html = render({ busy: true, restoringAccountId: "account-wise" });

    expect(html).toContain("Restoring…");
    expect(html).not.toContain(">Restore</button>");
  });
});
