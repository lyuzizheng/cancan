import { useState } from "react";

import { Button, MonogramTile, Panel, Select } from "@cancan/ui";

import type {
  AccountConfirmationPrompt,
  CandidateAccountDecisionInput,
} from "./command-contracts";
import { accountTypeLabel } from "./format";

const decisionOptions = [
  { label: "Accept", value: "accept" },
  { label: "Dismiss", value: "dismiss" },
];

/**
 * Account-confirmation card — the owning surface for candidate-account
 * decisions. Rendered inside the Sources attention panel (account prompts
 * belong to a Money Source, so Sources is their owning view).
 */
export function AccountConfirmationCard({
  busy,
  deciding,
  onDecide,
  onRestore,
  prompt,
  restoringAccountId,
}: {
  busy: boolean;
  deciding: boolean;
  onDecide: (decisions: CandidateAccountDecisionInput[]) => void;
  onRestore: (accountId: string) => void;
  prompt: AccountConfirmationPrompt;
  restoringAccountId: string | null;
}) {
  const [choices, setChoices] = useState<Record<string, "accept" | "dismiss">>({});
  const decisions = prompt.candidateAccounts.map((account) => ({
    accountId: account.accountId,
    action: choices[account.accountId] ?? "accept",
  }));
  return (
    <li>
      <Panel className="flex gap-3 p-4">
        <MonogramTile className="shrink-0" name={prompt.displayName} size={32} />
        <div className="min-w-0 flex-1">
          {prompt.candidateAccounts.length > 0 ? (
            <>
              <p className="text-sm font-medium text-ledger-ink">
                CanCan found new accounts in your {prompt.displayName} statements
              </p>
              <p className="mt-2 text-sm text-ledger-text-muted">
                Confirm each account or dismiss it. Dismissed records remain in history and stay out of review.
              </p>
              <ul className="m-0 mt-3 list-none border-t border-ledger-rule p-0">
                {prompt.candidateAccounts.map((account) => (
                  <li
                    className="flex items-center justify-between gap-3 border-b border-ledger-rule py-2.5"
                    key={account.accountId}
                  >
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium text-ledger-ink">{account.displayName}</p>
                      <p className="mt-0.5 font-mono text-xs text-ledger-text-muted">
                        {accountTypeLabel(account.accountType)}
                        {account.currency ? ` · ${account.currency}` : ""}
                        {account.maskedIdentifier ? ` · ${account.maskedIdentifier}` : ""}
                      </p>
                    </div>
                    <div className="w-28 shrink-0">
                      <Select
                        ariaLabel={`Decision for ${account.displayName}`}
                        disabled={busy}
                        onValueChange={(value) => setChoices((current) => ({
                          ...current,
                          [account.accountId]: value as "accept" | "dismiss",
                        }))}
                        options={decisionOptions}
                        value={choices[account.accountId] ?? "accept"}
                      />
                    </div>
                  </li>
                ))}
              </ul>
              <div className="mt-3 flex items-center gap-2">
                <Button
                  disabled={busy}
                  onClick={() => onDecide(decisions)}
                >
                  {deciding ? "Saving…" : "Save choices"}
                </Button>
              </div>
            </>
          ) : null}
          {prompt.dismissedAccounts.length > 0 ? (
            <ul className="m-0 list-none p-0">
              {prompt.dismissedAccounts.map((account) => (
                <li
                  className="flex items-center justify-between gap-3 py-2.5"
                  key={account.accountId}
                >
                  <p className="truncate text-sm text-ledger-text-muted">
                    {account.displayName} · dismissed
                  </p>
                  <Button
                    disabled={busy}
                    onClick={() => onRestore(account.accountId)}
                    size="sm"
                    variant="quiet"
                  >
                    {restoringAccountId === account.accountId ? "Restoring…" : "Restore"}
                  </Button>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </Panel>
    </li>
  );
}
