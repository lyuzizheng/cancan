import {
  Badge,
  Button,
  EmptyState,
  LedgerHeader,
  MetricRow,
  SectionHeader,
  Skeleton,
} from "@cancan/ui";

import type {
  MoneyOverview,
  MoneyOverviewAmount,
  RecentActivitySummary,
  TaskRow,
  Tasks,
} from "./command-contracts";
import { Feedback, type Notice } from "./feedback";
import {
  eventTypeLabel,
  formatCurrencyAmount,
  formatLedgerDate,
} from "./format";
import { TasksSection } from "./tasks";

export interface OverviewViewProps {
  loading: boolean;
  moneyOverview: MoneyOverview | null;
  notice: Notice | null;
  onLock: () => void;
  onOpenSources: () => void;
  onOpenTask: (row: TaskRow) => void;
  onRefresh: () => void;
  onUndo: (eventId: string) => void;
  onViewAllTasks: () => void;
  recentActivity: RecentActivitySummary[] | null;
  tasks: Tasks | null;
  undoingEventId: string | null;
}

/**
 * Command Center — the 0006 zones on the token foundation: one unified Tasks
 * section, then the money overview and recent activity streams on the open
 * ledger rhythm (hairline rules, no card grid).
 */
export function OverviewView(props: OverviewViewProps) {
  const overviewEmpty = props.moneyOverview !== null
    && props.moneyOverview.assets.length === 0
    && props.moneyOverview.liabilities.length === 0;

  return (
    <>
      <LedgerHeader
        actions={
          <>
            <Button onClick={props.onRefresh} variant="quiet">
              Refresh
            </Button>
            <Button onClick={props.onLock} variant="quiet">
              Lock Vault
            </Button>
          </>
        }
        eyebrow="Command Center"
        title="Your money, organized"
      />

      <div className="grid gap-10 pt-6">
        {props.notice ? <Feedback {...props.notice} /> : null}

        <TasksSection
          onOpenTask={props.onOpenTask}
          onViewAll={props.onViewAllTasks}
          tasks={props.tasks}
        />

        <section aria-label="Money overview">
          <SectionHeader title="Money Overview" tone="healthy" />
          <div className="mt-2 border-t border-ledger-rule">
            {props.loading && props.moneyOverview === null ? (
              <div className="grid gap-2.5 py-3" role="status">
                <span className="sr-only">Loading your balances…</span>
                <Skeleton className="h-4 w-2/3" />
                <Skeleton className="h-4 w-1/2" />
              </div>
            ) : null}
            {!props.loading && overviewEmpty ? (
              <EmptyState
                action={(
                  <Button onClick={props.onOpenSources} variant="quiet">
                    Add a statement from Sources
                  </Button>
                )}
                body="Balances appear here after your first records are added."
                icon="money-flow"
                title="No balances yet"
              />
            ) : null}
            {!overviewEmpty && props.moneyOverview !== null ? (
              <div className="grid gap-5 pt-3">
                <MoneyGroup label="Assets" amounts={props.moneyOverview.assets} />
                <MoneyGroup label="Liabilities" amounts={props.moneyOverview.liabilities} />
              </div>
            ) : null}
          </div>
        </section>

        <section aria-label="Recent activity">
          <SectionHeader title="Recent activity" tone="healthy" />
          <div className="mt-2 border-t border-ledger-rule">
            {props.loading && props.recentActivity === null ? (
              <div className="grid gap-2.5 py-3" role="status">
                <span className="sr-only">Loading recent activity…</span>
                <Skeleton className="h-4 w-2/3" />
                <Skeleton className="h-4 w-1/2" />
              </div>
            ) : null}
            {!props.loading && props.recentActivity !== null && props.recentActivity.length === 0 ? (
              <p className="py-3 text-sm text-ledger-text-muted">
                Nothing added yet. Records you add appear here quietly, with Undo if you need it.
              </p>
            ) : null}
            {props.recentActivity !== null && props.recentActivity.length > 0 ? (
              <ul className="m-0 list-none p-0">
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
          </div>
        </section>
      </div>
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
    <section aria-label={label}>
      <h3 className="px-0 font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        {label}
      </h3>
      <div className="mt-1 border-t border-ledger-rule">
        {amounts.map((amount, index) => (
          <MetricRow
            key={amount.accountId}
            label={amount.accountLabel}
            meta={`As of ${formatLedgerDate(amount.asOf)}`}
            ruled={index < amounts.length - 1}
            value={formatCurrencyAmount(amount.currency, amount.value)}
          />
        ))}
      </div>
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
    <li className="flex items-center justify-between gap-4 border-b border-ledger-rule py-2.5">
      <div className="min-w-0">
        <p className="flex items-center gap-2 text-md font-medium text-ledger-ink">
          {eventTypeLabel(event.eventType)}
          {event.spending ? <Badge tone="attention">Spending</Badge> : null}
        </p>
        <p className="mt-0.5 truncate font-mono text-xs text-ledger-text-muted">
          {formatLedgerDate(event.eventDate)}
          {event.sourceLabels.length > 0 ? ` · ${event.sourceLabels.join(" · ")}` : ""}
        </p>
      </div>
      {event.canUndo ? (
        <Button
          disabled={undoing}
          onClick={() => onUndo(event.eventId)}
          size="sm"
          variant="quiet"
        >
          {undoing ? "Undoing…" : "Undo"}
        </Button>
      ) : null}
    </li>
  );
}
