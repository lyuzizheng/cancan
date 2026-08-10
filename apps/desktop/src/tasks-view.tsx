import { Button, EmptyState, LedgerHeader, Skeleton, cx } from "@cancan/ui";

import type { TaskGroup, TaskRow, Tasks } from "./command-contracts";
import { TaskRowList, taskGroupLabel } from "./tasks";

const filterOrder: TaskGroup[] = [
  "needs_action",
  "in_progress",
  "recently_completed",
  "parked",
];

const emptyFilterCopy: Record<TaskGroup, string> = {
  in_progress: "No work in progress",
  needs_action: "Nothing needs action",
  parked: "Nothing parked",
  recently_completed: "Nothing completed in the last week",
};

export interface TasksViewProps {
  filter: TaskGroup;
  onFilterChange: (filter: TaskGroup) => void;
  onLock: () => void;
  onOpenTask: (row: TaskRow) => void;
  onRefresh: () => void;
  tasks: Tasks | null;
}

/**
 * Full Tasks route — the four group filters over the host's complete
 * projection (spec 0006 `View all` destination). The filter selection
 * survives deep-link returns via app-level state; scroll restore is a
 * tracked follow-up.
 */
export function TasksView(props: TasksViewProps) {
  const counts = new Map<TaskGroup, number>();
  for (const group of filterOrder) {
    counts.set(group, 0);
  }
  for (const row of props.tasks?.rows ?? []) {
    counts.set(row.group, (counts.get(row.group) ?? 0) + 1);
  }
  const visibleRows = (props.tasks?.rows ?? []).filter(
    (row) => row.group === props.filter,
  );

  return (
    <>
      <LedgerHeader
        actions={(
          <>
            <Button onClick={props.onRefresh} variant="quiet">
              Refresh
            </Button>
            <Button onClick={props.onLock} variant="quiet">
              Lock Vault
            </Button>
          </>
        )}
        eyebrow="Command Center"
        title="Tasks"
      />

      <div className="grid gap-4 pt-6">
        <div className="flex flex-wrap gap-2">
          {filterOrder.map((group) => {
            const active = group === props.filter;
            return (
              <button
                aria-pressed={active}
                className={cx(
                  "inline-flex h-7 items-center gap-1.5 rounded-pill border bg-transparent px-3 text-xs font-medium transition-colors duration-120 ease-mech",
                  "focus-visible:outline-2 focus-visible:outline-accent-go-deep focus-visible:outline-offset-2",
                  active
                    ? "border-accent-go-deep text-accent-go-deep"
                    : "border-ledger-rule text-ledger-text-muted hover:text-ledger-ink",
                )}
                key={group}
                onClick={() => props.onFilterChange(group)}
                type="button"
              >
                {taskGroupLabel(group)}
                <span className="font-mono tabular-nums">{counts.get(group) ?? 0}</span>
              </button>
            );
          })}
        </div>

        <div className="border-t border-ledger-rule">
          {props.tasks === null ? (
            <div className="grid gap-2.5 py-3" role="status">
              <span className="sr-only">Loading tasks…</span>
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
            </div>
          ) : visibleRows.length > 0 ? (
            <TaskRowList captions={false} onOpenTask={props.onOpenTask} rows={visibleRows} />
          ) : (
            <EmptyState icon="check" title={emptyFilterCopy[props.filter]} />
          )}
        </div>
      </div>
    </>
  );
}
