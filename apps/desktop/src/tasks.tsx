import {
  Button,
  EmptyState,
  Icon,
  SectionHeader,
  Skeleton,
  StatusPoint,
  type StatusPointTone,
} from "@cancan/ui";

import type {
  TaskConsequence,
  TaskGroup,
  TaskRow,
  Tasks,
} from "./command-contracts";

/**
 * Tasks — the one user-facing work projection (spec 0006). The host owns
 * ordering, grouping, badges, and expiry; the renderer only labels rows and
 * routes their destinations. Command Center shows at most five rows with
 * `View all`; the full route adds the four group filters.
 */

const groupLabels: Record<TaskGroup, string> = {
  in_progress: "In progress",
  needs_action: "Needs action",
  parked: "Parked",
  recently_completed: "Recently completed",
};

export function taskGroupLabel(group: TaskGroup): string {
  return groupLabels[group];
}

function taskGroupTone(group: TaskGroup): StatusPointTone {
  return group === "needs_action"
    ? "attention"
    : group === "recently_completed"
    ? "healthy"
    : "idle";
}

const consequenceLabels: Record<TaskConsequence, string> = {
  already_in_cancan: "Already in CanCan",
  file_not_added: "File not added",
  import_interrupted: "Import interrupted",
  inbox_file_could_not_be_added: "Inbox file couldn’t be added",
  inbox_file_parked: "Inbox file parked",
  needs_attention: "Needs attention",
  needs_review: "Needs review",
  new_source_detected: "New source detected",
  password_needed: "Password needed",
  password_parked: "Password parked",
  processing: "Processing",
  ready: "Ready",
  restore_source_file: "Restore source file?",
  same_statement_content: "Same statement content",
  save_recovery_file: "Recovery setup",
  setup_reminder_postponed: "Reminder postponed",
  source_file_left_deleted: "Source file left deleted",
  source_file_restored: "Source file restored",
  source_unassigned: "Kept unassigned",
};

export function taskConsequenceLabel(consequence: TaskConsequence): string {
  return consequenceLabels[consequence];
}

export interface TaskRowButtonProps {
  onOpenTask: (row: TaskRow) => void;
  row: TaskRow;
}

/**
 * One task row — a single click target with state in accessible text
 * (consequence label + group), never color alone (spec 0006 narrow/a11y).
 */
export function TaskRowButton({ onOpenTask, row }: TaskRowButtonProps) {
  return (
    <button
      className="group flex w-full items-center gap-2.5 border-b border-ledger-rule bg-transparent py-2.5 text-left transition-colors duration-120 ease-mech hover:bg-ledger-mineral focus-visible:outline-2 focus-visible:outline-accent-go-deep focus-visible:outline-offset-2"
      onClick={() => onOpenTask(row)}
      type="button"
    >
      <StatusPoint tone={taskGroupTone(row.group)} />
      <span className="shrink-0 text-xs text-ledger-text-muted">
        {taskConsequenceLabel(row.consequence)}
      </span>
      <span className="min-w-0 flex-1 truncate text-sm font-medium text-ledger-ink">
        {row.title}
      </span>
      <span className="shrink-0 text-xs tabular-nums text-ledger-text-muted">
        {row.timestamp}
      </span>
      <Icon
        className="shrink-0 text-ledger-text-muted transition-transform duration-120 ease-mech group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0"
        name="chevron-right"
        size={16}
      />
    </button>
  );
}

/**
 * Group-ordered row list with mono captions rendered when the group changes.
 * Rows arrive pre-ordered from the host; the renderer never re-sorts.
 * Single-group surfaces (the full route's filters) pass `captions={false}`
 * because the active filter chip already names the group.
 */
export function TaskRowList({
  captions = true,
  onOpenTask,
  rows,
}: {
  captions?: boolean;
  onOpenTask: (row: TaskRow) => void;
  rows: TaskRow[];
}) {
  let lastGroup: TaskGroup | null = null;
  return (
    <ul className="m-0 list-none p-0">
      {rows.map((row) => {
        const caption = captions && row.group !== lastGroup ? row.group : null;
        lastGroup = row.group;
        return (
          <li key={row.rowKey}>
            {caption !== null ? (
              <p className="mt-3 pt-1 font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted first:mt-0">
                {taskGroupLabel(caption)}
              </p>
            ) : null}
            <TaskRowButton onOpenTask={onOpenTask} row={row} />
          </li>
        );
      })}
    </ul>
  );
}

export interface TasksSectionProps {
  onOpenTask: (row: TaskRow) => void;
  onViewAll: () => void;
  tasks: Tasks | null;
}

/**
 * The unified Command Center Tasks section — replaces the interim
 * Needs-attention, Review-status, and Jobs cards (spec 0006).
 */
export function TasksSection(props: TasksSectionProps) {
  const needsAction = props.tasks?.needsActionCount ?? 0;
  return (
    <section aria-label="Tasks">
      <SectionHeader
        action={(
          <Button onClick={props.onViewAll} variant="text">
            View all
          </Button>
        )}
        count={needsAction > 0 ? needsAction : undefined}
        countUnit="task"
        title="Tasks"
        tone={needsAction > 0 ? "attention" : "healthy"}
      />
      <div className="mt-2 border-t border-ledger-rule">
        {props.tasks === null ? (
          <div className="grid gap-2.5 py-3" role="status">
            <span className="sr-only">Loading tasks…</span>
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-10 w-full" />
          </div>
        ) : props.tasks.rows.length === 0 ? (
          <EmptyState
            body="New work from Add, CanCan Inbox, and Review lands here first."
            icon="check"
            title="You’re all caught up"
          />
        ) : (
          <TaskRowList onOpenTask={props.onOpenTask} rows={props.tasks.rows} />
        )}
      </div>
    </section>
  );
}
