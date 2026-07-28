import type {
  AccountConfirmationPrompt,
  CandidateAccountDecisionInput,
  MoneyOverview,
  MoneyOverviewAmount,
  RecentActivitySummary,
} from "./command-contracts";
import { AttentionSection } from "./attention";
import { Feedback, type Notice } from "./feedback";
import {
  eventTypeLabel,
  formatCurrencyAmount,
  formatLedgerDate,
} from "./format";

export interface OverviewViewProps {
  accountPrompts: AccountConfirmationPrompt[];
  attentionBusyKey: string | null;
  loading: boolean;
  moneyOverview: MoneyOverview | null;
  notice: Notice | null;
  onDecideAccounts: (
    prompt: AccountConfirmationPrompt,
    decisions: CandidateAccountDecisionInput[],
  ) => void;
  onLock: () => void;
  onOpenReview: () => void;
  onOpenSources: () => void;
  onRefresh: () => void;
  onRestoreAccount: (accountId: string) => void;
  onUndo: (eventId: string) => void;
  recentActivity: RecentActivitySummary[] | null;
  reviewCount: number | null;
  undoingEventId: string | null;
}

export function OverviewView(props: OverviewViewProps) {
  const overviewEmpty = props.moneyOverview !== null
    && props.moneyOverview.assets.length === 0
    && props.moneyOverview.liabilities.length === 0;

  return (
    <>
      <header className="ledger-header">
        <div>
          <p className="ledger-eyebrow">Command Center</p>
          <h1>Your money, organized</h1>
        </div>
        <div className="ledger-actions">
          <button className="button button-quiet" onClick={props.onRefresh} type="button">
            Refresh
          </button>
          <button className="button button-quiet" onClick={props.onLock} type="button">
            Lock Vault
          </button>
        </div>
      </header>

      <section className="overview-content" aria-label="Money overview">
        {props.notice ? <Feedback {...props.notice} /> : null}

        <AttentionSection
          accountPrompts={props.accountPrompts}
          attentionBusyKey={props.attentionBusyKey}
          onDecideAccounts={props.onDecideAccounts}
          onRestoreAccount={props.onRestoreAccount}
        />

        <section className="money-panel" aria-labelledby="money-overview-heading">
          <div className="money-panel-heading">
            <h2 id="money-overview-heading">
              <span className="panel-dot panel-dot-emerald" aria-hidden="true" />
              Money Overview
            </h2>
          </div>
          {props.loading && props.moneyOverview === null ? (
            <p className="panel-status" role="status">Loading your balances…</p>
          ) : null}
          {!props.loading && overviewEmpty ? (
            <div className="panel-empty">
              <p>Balances appear here after your first records are added.</p>
              <button className="button button-quiet" onClick={props.onOpenSources} type="button">
                Add a statement from Sources
              </button>
            </div>
          ) : null}
          {!overviewEmpty && props.moneyOverview !== null ? (
            <div className="money-groups">
              <MoneyGroup label="Assets" amounts={props.moneyOverview.assets} />
              <MoneyGroup label="Liabilities" amounts={props.moneyOverview.liabilities} />
            </div>
          ) : null}
        </section>

        <section className="review-status-panel" aria-labelledby="review-status-heading">
          <div className="money-panel-heading">
            <h2 id="review-status-heading">
              <span className="panel-dot panel-dot-amber" aria-hidden="true" />
              Review
            </h2>
            {props.reviewCount !== null && props.reviewCount > 0 ? (
              <span className="attention-count" aria-label={`${props.reviewCount} to review`}>
                {props.reviewCount}
              </span>
            ) : null}
          </div>
          {props.reviewCount === null ? (
            <p className="panel-status" role="status">Checking review…</p>
          ) : props.reviewCount === 0 ? (
            <p className="panel-status">Nothing needs your check.</p>
          ) : (
            <div className="review-status-body">
              <p>
                {props.reviewCount === 1
                  ? "1 record needs your check."
                  : `${props.reviewCount} records need your check.`}
              </p>
              <button className="button button-primary" onClick={props.onOpenReview} type="button">
                Open Review
              </button>
            </div>
          )}
        </section>

        <section className="activity-panel" aria-labelledby="activity-heading">
          <div className="money-panel-heading">
            <h2 id="activity-heading">
              <span className="panel-dot panel-dot-emerald" aria-hidden="true" />
              Recent activity
            </h2>
          </div>
          {props.loading && props.recentActivity === null ? (
            <p className="panel-status" role="status">Loading recent activity…</p>
          ) : null}
          {!props.loading && props.recentActivity !== null && props.recentActivity.length === 0 ? (
            <p className="panel-status">
              Nothing added yet. Records you add appear here quietly, with Undo if you need it.
            </p>
          ) : null}
          {props.recentActivity !== null && props.recentActivity.length > 0 ? (
            <ul className="activity-list">
              {props.recentActivity.map((event) => (
                <ActivityRow
                  event={event}
                  key={event.eventId}
                  onUndo={props.onUndo}
                  undoing={props.undoingEventId === event.eventId}
                />
              ))}
            </ul>
          ) : null}
        </section>
      </section>
    </>
  );
}

function MoneyGroup({
  amounts,
  label,
}: {
  amounts: MoneyOverviewAmount[];
  label: string;
}) {
  if (amounts.length === 0) {
    return null;
  }
  return (
    <section className="money-group" aria-label={label}>
      <h3>{label}</h3>
      <ul className="money-list">
        {amounts.map((amount) => (
          <li className="money-row" key={amount.accountId}>
            <div className="money-account">
              <p>{amount.accountLabel}</p>
              <span className="money-asof">As of {formatLedgerDate(amount.asOf)}</span>
            </div>
            <p className="money-value">
              {formatCurrencyAmount(amount.currency, amount.value)}
            </p>
          </li>
        ))}
      </ul>
    </section>
  );
}

function ActivityRow({
  event,
  onUndo,
  undoing,
}: {
  event: RecentActivitySummary;
  onUndo: (eventId: string) => void;
  undoing: boolean;
}) {
  return (
    <li className="activity-row">
      <div className="activity-details">
        <p>
          {eventTypeLabel(event.eventType)}
          {event.spending ? <span className="activity-tag">Spending</span> : null}
        </p>
        <span className="activity-meta">
          {formatLedgerDate(event.eventDate)}
          {event.sourceLabels.length > 0 ? ` · ${event.sourceLabels.join(" · ")}` : ""}
        </span>
      </div>
      {event.canUndo ? (
        <button
          className="button button-quiet"
          disabled={undoing}
          onClick={() => onUndo(event.eventId)}
          type="button"
        >
          {undoing ? "Undoing…" : "Undo"}
        </button>
      ) : null}
    </li>
  );
}
