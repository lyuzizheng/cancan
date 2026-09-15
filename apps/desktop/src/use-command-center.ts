import { useCallback, useEffect, useRef, useState } from "react";

import type {
  MoneyOverview,
  RecentActivitySummary,
  ReviewItemSummary,
  TaskGroup,
  Tasks,
} from "./command-contracts";
import type { CommandWiring } from "./command-wiring";
import { commandErrorMessage, type VaultApi } from "./vault-api";
import type { Attention } from "./use-attention";
import type { Inbox } from "./use-inbox";
import type { ReviewQueue } from "./use-review-queue";
import type { VaultSession } from "./use-vault-session";

export interface CommandCenter {
  loadFinanceData(): Promise<ReviewItemSummary[] | null>;
  readonly moneyOverview: MoneyOverview | null;
  readonly recentActivity: RecentActivitySummary[] | null;
  /** Drops every session-scoped read-model state; the gate's reset half. */
  reset(): void;
  setTasksFilter(filter: TaskGroup): void;
  readonly tasks: Tasks | null;
  readonly tasksFilter: TaskGroup;
  readonly tasksFull: Tasks | null;
  undoCommittedEvent(eventId: string): void;
  readonly undoingEventId: string | null;
}

/**
 * Owns the command center's read model — money overview, recent activity, and
 * tasks — and the one shared finance load that also feeds the review queue, the
 * Inbox status, and the attention prompts.
 */
export function useCommandCenter({
  api,
  attention,
  inbox,
  review,
  session,
  wiring,
}: {
  api: VaultApi;
  attention: Attention;
  inbox: Inbox;
  review: ReviewQueue;
  session: VaultSession;
  wiring: { current: CommandWiring };
}): CommandCenter {
  const [moneyOverview, setMoneyOverview] = useState<MoneyOverview | null>(null);
  const [recentActivity, setRecentActivity] = useState<
    RecentActivitySummary[] | null
  >(null);
  const [undoingEventId, setUndoingEventId] = useState<string | null>(null);
  const [tasks, setTasks] = useState<Tasks | null>(null);
  const [tasksFull, setTasksFull] = useState<Tasks | null>(null);
  const [tasksFilter, setTasksFilter] = useState<TaskGroup>("needs_action");
  const financeLoadRequestId = useRef(0);

  const { receivePrompts } = attention;
  const { receiveFailure, receiveStatus } = inbox;
  const { receiveFinanceData } = review;
  const {
    documentsAllowed,
    runGuarded,
    sessionId: currentSessionId,
    setError,
    setNotice,
  } = session;

  useEffect(() => () => {
    financeLoadRequestId.current += 1;
  }, []);

  const reset = useCallback(() => {
    financeLoadRequestId.current += 1;
    setMoneyOverview(null);
    setRecentActivity(null);
    setUndoingEventId(null);
    setTasks(null);
    setTasksFull(null);
    setTasksFilter("needs_action");
  }, []);

  const loadFinanceData = useCallback(async (): Promise<ReviewItemSummary[] | null> => {
    if (!documentsAllowed.current) {
      return null;
    }
    const sessionId = currentSessionId.current;
    const requestId = financeLoadRequestId.current + 1;
    financeLoadRequestId.current = requestId;
    const isCurrentLoad = () => currentSessionId.current === sessionId
      && financeLoadRequestId.current === requestId;
    try {
      const inboxStatus = api.localInboxStatus();
      void inboxStatus.then(
        (status) => {
          if (isCurrentLoad()) {
            receiveStatus(status);
          }
        },
        (nextError) => {
          if (isCurrentLoad()) {
            receiveFailure(commandErrorMessage(nextError));
          }
        },
      );
      const [items, overview, activity, accounts, sourceConfirmations, fullTasks] = await Promise.all([
        api.listReviewItems(),
        api.getMoneyOverview(),
        api.listRecentActivity(),
        api.listAccountConfirmationPrompts(),
        api.listSourceConfirmationPrompts(),
        api.listTasks("full"),
      ]);
      if (isCurrentLoad()) {
        receiveFinanceData(items);
        receivePrompts(accounts, sourceConfirmations);
        setMoneyOverview(overview);
        setRecentActivity(activity);
        // The command-center list is the full list minus parked rows, capped
        // at five — the same projection the host applies for
        // TaskFilter::CommandCenter (runtime/tasks.rs).
        setTasks({
          needsActionCount: fullTasks.needsActionCount,
          rows: fullTasks.rows
            .filter((row) => row.group !== "parked")
            .slice(0, 5),
        });
        setTasksFull(fullTasks);
        return items;
      }
      return null;
    } catch (nextError) {
      if (isCurrentLoad()) {
        setError(commandErrorMessage(nextError));
      }
    }
    return null;
  }, [
    api,
    currentSessionId,
    documentsAllowed,
    receiveFailure,
    receiveFinanceData,
    receivePrompts,
    receiveStatus,
    setError,
  ]);

  const undoCommittedEvent = useCallback((eventId: string) => {
    if (undoingEventId !== null) {
      return;
    }
    const sessionId = currentSessionId.current;
    setUndoingEventId(eventId);
    void runGuarded(async () => {
      const outcome = await api.undoCommittedEvent(eventId);
      if (currentSessionId.current !== sessionId) {
        return;
      }
      setNotice(
        outcome.status === "undone"
          ? {
              body: "CanCan added the reversal to your recent activity.",
              tone: "success",
              title: "Undone",
            }
          : {
              body: "That event was already reversed.",
              tone: "attention",
              title: "Already undone",
            },
      );
      await wiring.current.loadFinanceData();
    }, { busy: true, sessionId, onSettled: () => setUndoingEventId(null) });
  }, [
    api,
    currentSessionId,
    runGuarded,
    setNotice,
    undoingEventId,
    wiring,
  ]);

  return {
    loadFinanceData,
    moneyOverview,
    recentActivity,
    reset,
    setTasksFilter,
    tasks,
    tasksFilter,
    tasksFull,
    undoCommittedEvent,
    undoingEventId,
  };
}
