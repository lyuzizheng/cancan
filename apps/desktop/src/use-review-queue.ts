import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";

import type { ReviewItemSummary } from "./command-contracts";
import type { CommandWiring } from "./command-wiring";
import type { ReviewDetailState, ReviewJobPanelState } from "./review";
import { createReviewActions, type ReviewActions } from "./review-actions";
import type { VaultApi } from "./vault-api";
import type { VaultSession } from "./use-vault-session";

export interface ReviewQueue extends ReviewActions {
  clearSelection(): void;
  readonly mutatingReviewItemId: string | null;
  /** Adopts the review queue the shared finance load just read. */
  receiveFinanceData(items: ReviewItemSummary[]): void;
  /** Drops every session-scoped review state; the gate's reset half. */
  reset(): void;
  readonly reviewDetail: ReviewDetailState | null;
  readonly reviewItems: ReviewItemSummary[] | null;
  readonly reviewJob: ReviewJobPanelState | null;
  readonly selectedReviewIds: ReadonlySet<string>;
  selectAll(): void;
  setSelectedReviewIds: Dispatch<SetStateAction<ReadonlySet<string>>>;
}

/**
 * Owns the review queue: the staged records, the row selection, the open
 * record's detail, and the batch-add job panel. The commands themselves live in
 * `review-actions.ts`.
 */
export function useReviewQueue({
  api,
  session,
  wiring,
}: {
  api: VaultApi;
  session: VaultSession;
  wiring: { current: CommandWiring };
}): ReviewQueue {
  const [reviewItems, setReviewItems] = useState<ReviewItemSummary[] | null>(null);
  const [selectedReviewIds, setSelectedReviewIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [reviewDetail, setReviewDetail] = useState<ReviewDetailState | null>(null);
  const [reviewJob, setReviewJob] = useState<ReviewJobPanelState | null>(null);
  const [mutatingReviewItemId, setMutatingReviewItemId] = useState<string | null>(null);
  const reviewDetailRequestId = useRef(0);
  const reviewJobPollTimer = useRef<number | null>(null);

  const stopPolling = useCallback(() => {
    if (reviewJobPollTimer.current !== null) {
      window.clearTimeout(reviewJobPollTimer.current);
      reviewJobPollTimer.current = null;
    }
  }, []);

  useEffect(() => () => {
    reviewDetailRequestId.current += 1;
    stopPolling();
  }, [stopPolling]);

  const { sessionId, setError, setNotice } = session;

  const receiveFinanceData = useCallback((items: ReviewItemSummary[]) => {
    setReviewItems(items);
    setSelectedReviewIds((current) => new Set(
      [...current].filter((id) =>
        items.some((item) => item.reviewItemId === id)
      ),
    ));
  }, []);

  const reload = useCallback(() => wiring.current.loadFinanceData(), [wiring]);

  // Memoized so the view above keeps a stable prop identity while none of the
  // queue state it acts on has changed.
  const actions = useMemo(() => createReviewActions({
    api,
    mutatingReviewItemId,
    reload,
    reviewDetail,
    reviewDetailRequestId,
    reviewItems,
    reviewJob,
    reviewJobPollTimer,
    selectedReviewIds,
    sessionId,
    setError,
    setMutatingReviewItemId,
    setNotice,
    setReviewDetail,
    setReviewJob,
    setSelectedReviewIds,
    stopPolling,
  }), [
    api,
    mutatingReviewItemId,
    reload,
    reviewDetail,
    reviewItems,
    reviewJob,
    selectedReviewIds,
    sessionId,
    setError,
    setNotice,
    stopPolling,
  ]);

  const clearSelection = useCallback(() => setSelectedReviewIds(new Set()), []);

  const selectAll = useCallback(() => {
    setSelectedReviewIds(new Set(
      reviewItems
        ?.filter((item) => !item.recordCommitted)
        .map((item) => item.reviewItemId) ?? [],
    ));
  }, [reviewItems]);

  const reset = useCallback(() => {
    reviewDetailRequestId.current += 1;
    stopPolling();
    setReviewItems(null);
    setSelectedReviewIds(new Set());
    setReviewDetail(null);
    setReviewJob(null);
    setMutatingReviewItemId(null);
  }, [stopPolling]);

  return {
    ...actions,
    clearSelection,
    mutatingReviewItemId,
    receiveFinanceData,
    reset,
    reviewDetail,
    reviewItems,
    reviewJob,
    selectedReviewIds,
    selectAll,
    setSelectedReviewIds,
  };
}
