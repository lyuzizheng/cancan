import type {
  AccountConfirmationPrompt,
  MoneySourceSummary,
  StatementCoveragePrompt,
} from "./command-contracts";
import {
  accountTypeLabel,
  coveragePromptTitle,
} from "./format";

export interface RemindState {
  date: string;
  error: string | null;
  key: string;
  saving: boolean;
}

export function coverageKey(prompt: StatementCoveragePrompt): string {
  return [
    prompt.moneySourceId,
    prompt.accountId,
    prompt.documentType,
    prompt.statementPeriodFrom,
    prompt.statementPeriodTo,
  ].join("|");
}

export interface AttentionSectionProps {
  accountPrompts: AccountConfirmationPrompt[];
  attentionBusyKey: string | null;
  coveragePrompts: StatementCoveragePrompt[];
  moneySources: MoneySourceSummary[];
  onAddFile: () => void;
  onCancelRemind: () => void;
  onChangeRemindDate: (value: string) => void;
  onConfirmAccounts: (prompt: AccountConfirmationPrompt) => void;
  onNotExpected: (prompt: StatementCoveragePrompt) => void;
  onSaveRemind: () => void;
  onStartRemind: (prompt: StatementCoveragePrompt) => void;
  remind: RemindState | null;
}

/** Calm Command Center attention cards: missing statements and new accounts. */
export function AttentionSection(props: AttentionSectionProps) {
  const sourceNames = new Map(
    props.moneySources.map((source) => [source.moneySourceId, source.displayName]),
  );
  const total = props.coveragePrompts.length + props.accountPrompts.length;
  if (total === 0) {
    return null;
  }
  return (
    <section className="attention-panel" aria-labelledby="overview-attention-heading">
      <div className="money-panel-heading">
        <h2 id="overview-attention-heading">
          <span className="panel-dot panel-dot-amber" aria-hidden="true" />
          Needs attention
        </h2>
        <span className="attention-count" aria-label={`${total} to check`}>
          {total}
        </span>
      </div>
      <ul className="attention-card-list">
        {props.accountPrompts.map((prompt) => (
          <AccountConfirmationCard
            busy={props.attentionBusyKey !== null}
            confirming={props.attentionBusyKey === `account:${prompt.moneySourceId}`}
            key={prompt.moneySourceId}
            onConfirm={() => props.onConfirmAccounts(prompt)}
            prompt={prompt}
          />
        ))}
        {props.coveragePrompts.map((prompt) => (
          <CoverageCard
            busy={props.attentionBusyKey !== null}
            deciding={props.attentionBusyKey === coverageKey(prompt)}
            key={coverageKey(prompt)}
            onAddFile={props.onAddFile}
            onCancelRemind={props.onCancelRemind}
            onChangeRemindDate={props.onChangeRemindDate}
            onNotExpected={() => props.onNotExpected(prompt)}
            onSaveRemind={props.onSaveRemind}
            onStartRemind={() => props.onStartRemind(prompt)}
            prompt={prompt}
            remind={props.remind?.key === coverageKey(prompt) ? props.remind : null}
            sourceName={sourceNames.get(prompt.moneySourceId) ?? "A money source"}
          />
        ))}
      </ul>
    </section>
  );
}

function AccountConfirmationCard({
  busy,
  confirming,
  onConfirm,
  prompt,
}: {
  busy: boolean;
  confirming: boolean;
  onConfirm: () => void;
  prompt: AccountConfirmationPrompt;
}) {
  return (
    <li className="attention-card">
      <p className="attention-card-title">
        CanCan found new accounts in your {prompt.displayName} statements
      </p>
      <p className="attention-card-copy">
        Confirm these accounts are yours. Their records can then be added to your ledger.
      </p>
      <ul className="attention-candidate-list">
        {prompt.candidateAccounts.map((account) => (
          <li className="attention-candidate" key={account.accountId}>
            <p>{account.displayName}</p>
            <span>
              {accountTypeLabel(account.accountType)}
              {account.currency ? ` · ${account.currency}` : ""}
              {account.maskedIdentifier ? ` · ${account.maskedIdentifier}` : ""}
            </span>
          </li>
        ))}
      </ul>
      <div className="attention-card-actions">
        <button
          className="button button-primary"
          disabled={busy}
          onClick={onConfirm}
          type="button"
        >
          {confirming ? "Confirming…" : "These are mine"}
        </button>
      </div>
    </li>
  );
}

function CoverageCard({
  busy,
  deciding,
  onAddFile,
  onCancelRemind,
  onChangeRemindDate,
  onNotExpected,
  onSaveRemind,
  onStartRemind,
  prompt,
  remind,
  sourceName,
}: {
  busy: boolean;
  deciding: boolean;
  onAddFile: () => void;
  onCancelRemind: () => void;
  onChangeRemindDate: (value: string) => void;
  onNotExpected: () => void;
  onSaveRemind: () => void;
  onStartRemind: () => void;
  prompt: StatementCoveragePrompt;
  remind: RemindState | null;
  sourceName: string;
}) {
  return (
    <li className="attention-card">
      <p className="attention-card-title">
        {coveragePromptTitle(prompt, sourceName)}
      </p>
      <p className="attention-card-copy">
        Add the statement if you have it, or tell CanCan what to expect for this period.
      </p>
      {remind ? (
        <form
          className="attention-remind"
          onSubmit={(event) => {
            event.preventDefault();
            onSaveRemind();
          }}
        >
          <label>
            Remind after
            <input
              aria-label="Remind after"
              disabled={remind.saving}
              inputMode="numeric"
              onChange={(event) => onChangeRemindDate(event.target.value)}
              placeholder="2026-09-01"
              value={remind.date}
            />
          </label>
          <p className="attention-remind-hint">
            Pick a future date. The prompt comes back after it.
          </p>
          {remind.error ? (
            <p className="attention-remind-error" role="alert">{remind.error}</p>
          ) : null}
          <div className="attention-card-actions">
            <button className="button button-primary" disabled={remind.saving} type="submit">
              {remind.saving ? "Saving…" : "Save reminder"}
            </button>
            <button
              className="button button-quiet"
              disabled={remind.saving}
              onClick={onCancelRemind}
              type="button"
            >
              Cancel
            </button>
          </div>
        </form>
      ) : (
        <div className="attention-card-actions">
          <button
            className="button button-primary"
            disabled={busy}
            onClick={onAddFile}
            type="button"
          >
            Add file
          </button>
          <button
            className="button button-quiet"
            disabled={busy}
            onClick={onNotExpected}
            type="button"
          >
            {deciding ? "Saving…" : "Not expected"}
          </button>
          <button
            className="button button-quiet"
            disabled={busy}
            onClick={onStartRemind}
            type="button"
          >
            Remind later
          </button>
        </div>
      )}
    </li>
  );
}
