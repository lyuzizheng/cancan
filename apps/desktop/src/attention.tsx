import { useState } from "react";

import type {
  AccountConfirmationPrompt,
  CandidateAccountDecisionInput,
  MoneySourceSummary,
  SourceConfirmationPrompt,
} from "./command-contracts";
import { accountTypeLabel, sourceInitials } from "./format";
import { SourceConfirmationCardList } from "./source-confirmation";

export interface AttentionSectionProps {
  accountPrompts: AccountConfirmationPrompt[];
  attentionBusyKey: string | null;
  existingSources: MoneySourceSummary[];
  onConfirmSourceCandidate: (
    prompt: SourceConfirmationPrompt,
    displayName: string,
    sourceType: string,
  ) => void;
  onDecideAccounts: (
    prompt: AccountConfirmationPrompt,
    decisions: CandidateAccountDecisionInput[],
  ) => void;
  onKeepSourceCandidateUnassigned: (prompt: SourceConfirmationPrompt) => void;
  onRestoreAccount: (accountId: string) => void;
  sourcePrompts: SourceConfirmationPrompt[];
}

export function AttentionSection(props: AttentionSectionProps) {
  const total = props.accountPrompts.reduce(
    (count, prompt) => count + prompt.candidateAccounts.length,
    0,
  ) + props.sourcePrompts.length;
  if (props.accountPrompts.length === 0 && props.sourcePrompts.length === 0) {
    return null;
  }
  return (
    <section className="attention-panel" aria-labelledby="overview-attention-heading">
      <div className="money-panel-heading">
        <h2 id="overview-attention-heading">
          <span className="panel-dot panel-dot-amber" aria-hidden="true" />
          Needs attention
        </h2>
        {total > 0 ? (
          <span className="attention-count" aria-label={`${total} to check`}>
            {total}
          </span>
        ) : null}
      </div>
      <SourceConfirmationCardList
        busyKey={props.attentionBusyKey}
        existingSources={props.existingSources}
        onConfirm={props.onConfirmSourceCandidate}
        onKeepUnassigned={props.onKeepSourceCandidateUnassigned}
        prompts={props.sourcePrompts}
      />
      <ul className="attention-card-list">
        {props.accountPrompts.map((prompt) => (
          <AccountConfirmationCard
            busy={props.attentionBusyKey !== null}
            deciding={props.attentionBusyKey === `account:${prompt.moneySourceId}`}
            key={prompt.moneySourceId}
            onDecide={(decisions) => props.onDecideAccounts(prompt, decisions)}
            onRestore={props.onRestoreAccount}
            prompt={prompt}
          />
        ))}
      </ul>
    </section>
  );
}

function AccountConfirmationCard({
  busy,
  deciding,
  onDecide,
  onRestore,
  prompt,
}: {
  busy: boolean;
  deciding: boolean;
  onDecide: (decisions: CandidateAccountDecisionInput[]) => void;
  onRestore: (accountId: string) => void;
  prompt: AccountConfirmationPrompt;
}) {
  const [choices, setChoices] = useState<Record<string, "accept" | "dismiss">>({});
  const decisions = prompt.candidateAccounts.map((account) => ({
    accountId: account.accountId,
    action: choices[account.accountId] ?? "accept",
  }));
  return (
    <li className="attention-card">
      <span className="attention-tile" aria-hidden="true">
        {sourceInitials(prompt.displayName)}
      </span>
      <div className="attention-card-body">
        {prompt.candidateAccounts.length > 0 ? (
          <>
            <p className="attention-card-title">
              CanCan found new accounts in your {prompt.displayName} statements
            </p>
            <p className="attention-card-copy">
              Confirm each account or dismiss it. Dismissed records remain in history and stay out of review.
            </p>
            <ul className="attention-candidate-list">
              {prompt.candidateAccounts.map((account) => (
                <li className="attention-candidate" key={account.accountId}>
                  <div>
                    <p>{account.displayName}</p>
                    <span>
                      {accountTypeLabel(account.accountType)}
                      {account.currency ? ` · ${account.currency}` : ""}
                      {account.maskedIdentifier ? ` · ${account.maskedIdentifier}` : ""}
                    </span>
                  </div>
                  <select
                    aria-label={`Decision for ${account.displayName}`}
                    disabled={busy}
                    onChange={(event) => setChoices((current) => ({
                      ...current,
                      [account.accountId]: event.target.value as "accept" | "dismiss",
                    }))}
                    value={choices[account.accountId] ?? "accept"}
                  >
                    <option value="accept">Accept</option>
                    <option value="dismiss">Dismiss</option>
                  </select>
                </li>
              ))}
            </ul>
            <div className="attention-card-actions">
              <button
                className="button button-strong"
                disabled={busy}
                onClick={() => onDecide(decisions)}
                type="button"
              >
                {deciding ? "Saving…" : "Save choices"}
              </button>
            </div>
          </>
        ) : null}
        {prompt.dismissedAccounts.length > 0 ? (
          <ul className="attention-candidate-list">
            {prompt.dismissedAccounts.map((account) => (
              <li className="attention-candidate" key={account.accountId}>
                <p>{account.displayName} · dismissed</p>
                <button
                  className="button button-quiet"
                  disabled={busy}
                  onClick={() => onRestore(account.accountId)}
                  type="button"
                >
                  Restore
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </div>
    </li>
  );
}
