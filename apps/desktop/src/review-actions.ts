import type { Dispatch, SetStateAction } from "react";

import type {
  RelationshipCandidateSummary,
  ReviewItemSummary,
  ReviewJobSummary,
  ReviewMutationOutcome,
} from "./command-contracts";
import type { Notice } from "./feedback";
import { isRealIsoDate, reviewConflictMessage } from "./format";
import type {
  ReviewDetailState,
  ReviewEditState,
  ReviewJobPanelState,
} from "./review";
import { commandErrorMessage, type VaultApi } from "./vault-api";

const UNSIGNED_DECIMAL = /^(0|[1-9]\d*)(\.\d+)?$/;
const SIGNED_DECIMAL = /^-?(0|[1-9]\d*)(\.\d+)?$/;

/**
 * Batch commits run on the host's review queue, so the panel polls. The
 * interval backs off to `REVIEW_JOB_POLL_MAX_INTERVAL_MS` and the job is only
 * declared failed once `REVIEW_JOB_POLL_DEADLINE_MS` has passed — a large batch
 * legitimately outlives a fixed number of fast polls.
 */
const REVIEW_JOB_POLL_INTERVAL_MS = 600;
const REVIEW_JOB_POLL_BACKOFF = 1.5;
const REVIEW_JOB_POLL_MAX_INTERVAL_MS = 5_000;
const REVIEW_JOB_POLL_DEADLINE_MS = 10 * 60 * 1000;

export interface ReviewActions {
  acceptCandidate(candidate: RelationshipCandidateSummary): Promise<void>;
  acknowledge(): Promise<void>;
  cancelEdit(): void;
  cancelRemove(): void;
  changeEdit(patch: Partial<Pick<
    ReviewEditState,
    "accountBalanceDelta" | "amountValue" | "postedOn"
  >>): void;
  closeDetail(): void;
  confirmRemove(): Promise<void>;
  enqueueBatch(): void;
  handleConflict(reason: string | null): Promise<void>;
  openDetail(item: ReviewItemSummary): void;
  requestRemove(): void;
  saveEdit(): Promise<void>;
  startEdit(): void;
  toggleSelection(reviewItemId: string): void;
}

export interface ReviewActionDeps {
  api: VaultApi;
  mutatingReviewItemId: string | null;
  /** Reloads the finance read model and returns the fresh review queue. */
  reload(): Promise<ReviewItemSummary[] | null>;
  reviewDetail: ReviewDetailState | null;
  reviewDetailRequestId: { current: number };
  reviewItems: ReviewItemSummary[] | null;
  reviewJob: ReviewJobPanelState | null;
  reviewJobPollTimer: { current: number | null };
  selectedReviewIds: ReadonlySet<string>;
  sessionId: { current: number };
  setError(message: string): void;
  setMutatingReviewItemId: Dispatch<SetStateAction<string | null>>;
  setNotice(notice: Notice | null): void;
  setReviewDetail: Dispatch<SetStateAction<ReviewDetailState | null>>;
  setReviewJob: Dispatch<SetStateAction<ReviewJobPanelState | null>>;
  setSelectedReviewIds: Dispatch<SetStateAction<ReadonlySet<string>>>;
  /** Clears the scheduled batch-job poll; the timer lives with the queue state. */
  stopPolling(): void;
}

/**
 * Review-queue orchestration: opening a record's detail, the edit, remove, and
 * acknowledge mutations with their version conflicts, linking a related record,
 * and the batch-add job that the panel polls.
 */
export function createReviewActions(deps: ReviewActionDeps): ReviewActions {
  const {
    api,
    mutatingReviewItemId,
    reload,
    reviewDetail,
    reviewDetailRequestId,
    reviewItems,
    reviewJob,
    reviewJobPollTimer,
    selectedReviewIds,
    sessionId: vaultSessionId,
    setError,
    setMutatingReviewItemId,
    setNotice,
    setReviewDetail,
    setReviewJob,
    setSelectedReviewIds,
    stopPolling,
  } = deps;

  const closeDetail: ReviewActions["closeDetail"] = () => {
    reviewDetailRequestId.current += 1;
    setReviewDetail(null);
  };

  const handleConflict: ReviewActions["handleConflict"] = async (reason) => {
    setNotice({
      body: reviewConflictMessage(reason),
      tone: "attention",
      title: "Couldn’t apply that change",
    });
    reviewDetailRequestId.current += 1;
    setReviewDetail(null);
    await reload();
  };

  const openDetail: ReviewActions["openDetail"] = (item) => {
    const sessionId = vaultSessionId.current;
    const requestId = reviewDetailRequestId.current + 1;
    reviewDetailRequestId.current = requestId;
    setReviewDetail({
      candidates: null,
      confirmingRemove: false,
      detail: null,
      editing: null,
      reviewItemId: item.reviewItemId,
      summary: item,
    });
    void Promise.all([
      api.getReviewDetail(item.reviewItemId),
      item.recordCommitted
        ? Promise.resolve<RelationshipCandidateSummary[]>([])
        : api.listRelationshipCandidates(item.reviewItemId, item.recordVersion),
    ]).then(([detail, candidates]) => {
      if (
        vaultSessionId.current !== sessionId
        || reviewDetailRequestId.current !== requestId
      ) {
        return;
      }
      if (detail === null) {
        setReviewDetail(null);
        setNotice({
          body: "That item is no longer waiting for review.",
          tone: "attention",
          title: "Already resolved",
        });
        void reload();
        return;
      }
      setReviewDetail({
        candidates,
        confirmingRemove: false,
        detail,
        editing: null,
        reviewItemId: item.reviewItemId,
        summary: item,
      });
    }).catch((nextError) => {
      if (
        vaultSessionId.current === sessionId
        && reviewDetailRequestId.current === requestId
      ) {
        reviewDetailRequestId.current += 1;
        setReviewDetail(null);
        setError(commandErrorMessage(nextError));
      }
    });
  };

  const toggleSelection: ReviewActions["toggleSelection"] = (reviewItemId) => {
    setSelectedReviewIds((current) => {
      const next = new Set(current);
      if (next.has(reviewItemId)) {
        next.delete(reviewItemId);
      } else {
        next.add(reviewItemId);
      }
      return next;
    });
  };

  const startEdit: ReviewActions["startEdit"] = () => {
    setReviewDetail((current) => current === null
      ? current
      : {
          ...current,
          confirmingRemove: false,
          editing: {
            accountBalanceDelta: "",
            amountValue: current.summary.amountValue ?? "",
            error: null,
            postedOn: current.summary.postedOn ?? "",
            saving: false,
          },
        });
  };

  const cancelEdit: ReviewActions["cancelEdit"] = () => {
    setReviewDetail((current) => current === null
      ? current
      : { ...current, editing: null });
  };

  const changeEdit: ReviewActions["changeEdit"] = (patch) => {
    setReviewDetail((current) => current?.editing
      ? { ...current, editing: { ...current.editing, ...patch, error: null } }
      : current);
  };

  const failEdit = (message: string) => {
    setReviewDetail((current) => current?.editing
      ? { ...current, editing: { ...current.editing, error: message, saving: false } }
      : current);
  };

  const saveEdit: ReviewActions["saveEdit"] = async () => {
    const state = reviewDetail;
    if (!state?.editing || mutatingReviewItemId !== null) {
      return;
    }
    const { editing } = state;
    const patch: Partial<Pick<ReviewEditState, "accountBalanceDelta" | "amountValue" | "postedOn">> = {};
    const amountValue = editing.amountValue.trim();
    const postedOn = editing.postedOn.trim();
    const accountBalanceDelta = editing.accountBalanceDelta.trim();
    if (amountValue !== (state.summary.amountValue ?? "")) {
      if (!UNSIGNED_DECIMAL.test(amountValue)) {
        failEdit("Amount must be a positive decimal, such as 128.50.");
        return;
      }
      patch.amountValue = amountValue;
    }
    if (postedOn !== (state.summary.postedOn ?? "")) {
      if (!isRealIsoDate(postedOn)) {
        failEdit("Date must use the YYYY-MM-DD format, such as 2026-07-19.");
        return;
      }
      patch.postedOn = postedOn;
    }
    if (accountBalanceDelta !== "") {
      if (!SIGNED_DECIMAL.test(accountBalanceDelta)) {
        failEdit("Balance change must be a signed decimal, such as -128.50.");
        return;
      }
      patch.accountBalanceDelta = accountBalanceDelta;
    }
    if (
      patch.amountValue === undefined
      && patch.postedOn === undefined
      && patch.accountBalanceDelta === undefined
    ) {
      failEdit("Change something before saving.");
      return;
    }

    const sessionId = vaultSessionId.current;
    setMutatingReviewItemId(state.reviewItemId);
    setReviewDetail((current) => current?.editing
      ? { ...current, editing: { ...current.editing, saving: true } }
      : current);
    try {
      const outcome = await api.editReviewRecord(
        state.reviewItemId,
        state.summary.recordVersion,
        patch,
      );
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      if (outcome.status === "conflict") {
        setMutatingReviewItemId(null);
        await handleConflict(outcome.reason);
        return;
      }
      setNotice({
        body: "CanCan will use the corrected details from now on.",
        tone: "success",
        title: "Edit saved",
      });
      setMutatingReviewItemId(null);
      const refreshed = await reload();
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      if (outcome.reviewItemId !== null) {
        const nextItem = refreshed?.find(
          (item) => item.reviewItemId === outcome.reviewItemId,
        );
        if (nextItem) {
          openDetail(nextItem);
        } else {
          closeDetail();
        }
      } else {
        closeDetail();
      }
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setMutatingReviewItemId(null);
        setReviewDetail((current) => current?.editing
          ? { ...current, editing: { ...current.editing, saving: false } }
          : current);
        setError(commandErrorMessage(nextError));
      }
    }
  };

  const requestRemove: ReviewActions["requestRemove"] = () => {
    setReviewDetail((current) => current === null
      ? current
      : { ...current, confirmingRemove: true, editing: null });
  };

  const cancelRemove: ReviewActions["cancelRemove"] = () => {
    setReviewDetail((current) => current === null
      ? current
      : { ...current, confirmingRemove: false });
  };

  /**
   * Every review mutation lands on the same outcome: a version conflict
   * reloads the queue, success clears the detail and drops the record from the
   * selection, and a failure restores the mutating mark for a retry.
   */
  const runMutation = async (
    mutate: () => Promise<ReviewMutationOutcome>,
    onSuccess: () => void | Promise<void>,
  ) => {
    const state = reviewDetail;
    if (!state || mutatingReviewItemId !== null) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setMutatingReviewItemId(state.reviewItemId);
    try {
      const outcome = await mutate();
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setMutatingReviewItemId(null);
      if (outcome.status === "conflict") {
        await handleConflict(outcome.reason);
        return;
      }
      await onSuccess();
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setMutatingReviewItemId(null);
        setError(commandErrorMessage(nextError));
      }
    }
  };

  const clearResolvedItem = (reviewItemId: string) => {
    reviewDetailRequestId.current += 1;
    setReviewDetail(null);
    setSelectedReviewIds((current) => {
      const next = new Set(current);
      next.delete(reviewItemId);
      return next;
    });
  };

  const confirmRemove: ReviewActions["confirmRemove"] = async () => {
    const state = reviewDetail;
    if (!state) {
      return;
    }
    await runMutation(
      () => api.removeReviewRecord(state.reviewItemId, state.summary.recordVersion),
      async () => {
        clearResolvedItem(state.reviewItemId);
        setNotice({
          body: "The staged record is out of the queue. Its history stays in your audit trail.",
          tone: "success",
          title: "Record removed",
        });
        await reload();
      },
    );
  };

  const acknowledge: ReviewActions["acknowledge"] = async () => {
    const state = reviewDetail;
    if (!state) {
      return;
    }
    await runMutation(
      () => api.acknowledgeReviewItem(state.reviewItemId, state.summary.recordVersion),
      async () => {
        clearResolvedItem(state.reviewItemId);
        setNotice({
          body: "The added record stays as it is. This check is out of the queue.",
          tone: "success",
          title: "Check cleared",
        });
        await reload();
      },
    );
  };

  const acceptCandidate: ReviewActions["acceptCandidate"] = async (candidate) => {
    const state = reviewDetail;
    if (!state) {
      return;
    }
    const sessionId = vaultSessionId.current;
    await runMutation(
      () => api.acceptReviewRelationship(
        state.reviewItemId,
        state.summary.recordVersion,
        candidate.recordId,
        candidate.recordVersion,
      ),
      async () => {
        setSelectedReviewIds((current) => {
          const next = new Set(current);
          next.add(state.reviewItemId);
          const linked = reviewItems?.find(
            (item) => item.recordId === candidate.recordId,
          );
          if (linked) {
            next.add(linked.reviewItemId);
          }
          return next;
        });
        setNotice({
          body: "CanCan will treat them as one event. Add both together when you’re ready.",
          tone: "success",
          title: "Linked",
        });
        await reload();
        if (vaultSessionId.current === sessionId) {
          openDetail(state.summary);
        }
      },
    );
  };

  const finishReviewJob = (summary: ReviewJobSummary) => {
    if (summary.status === "succeeded") {
      setReviewJob({ jobId: summary.jobId, outcomes: summary.outcomes, status: "done" });
    } else {
      setReviewJob({ jobId: summary.jobId, outcomes: summary.outcomes, status: "failed" });
    }
    setSelectedReviewIds(new Set());
    void reload();
  };

  const failReviewJob = (jobId: string) => {
    setReviewJob({ jobId, outcomes: [], status: "failed" });
    setSelectedReviewIds(new Set());
    void reload();
  };

  const pollReviewJob = (jobId: string, startedAt: number, intervalMs: number) => {
    stopPolling();
    if (Date.now() - startedAt >= REVIEW_JOB_POLL_DEADLINE_MS) {
      failReviewJob(jobId);
      return;
    }
    const sessionId = vaultSessionId.current;
    reviewJobPollTimer.current = window.setTimeout(() => {
      reviewJobPollTimer.current = null;
      void api.getReviewJob(jobId).then((summary) => {
        if (vaultSessionId.current !== sessionId) {
          return;
        }
        if (summary === null) {
          failReviewJob(jobId);
          return;
        }
        if (
          summary.status === "queued"
          || summary.status === "running"
        ) {
          pollReviewJob(
            jobId,
            startedAt,
            Math.min(intervalMs * REVIEW_JOB_POLL_BACKOFF, REVIEW_JOB_POLL_MAX_INTERVAL_MS),
          );
          return;
        }
        finishReviewJob(summary);
      }).catch(() => {
        if (vaultSessionId.current === sessionId) {
          pollReviewJob(
            jobId,
            startedAt,
            Math.min(intervalMs * REVIEW_JOB_POLL_BACKOFF, REVIEW_JOB_POLL_MAX_INTERVAL_MS),
          );
        }
      });
    }, intervalMs);
  };

  const enqueueBatch: ReviewActions["enqueueBatch"] = () => {
    if (reviewItems === null || reviewJob?.status === "running") {
      return;
    }
    const ids = reviewItems
      .filter((item) => selectedReviewIds.has(item.reviewItemId) && !item.recordCommitted)
      .map((item) => item.reviewItemId);
    if (ids.length === 0) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setNotice(null);
    setReviewJob({ jobId: null, outcomes: [], status: "running" });
    void api.enqueueCommitReviewBatch(ids).then((summary) => {
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setReviewJob({ jobId: summary.jobId, outcomes: [], status: "running" });
      if (summary.status === "queued" || summary.status === "running") {
        pollReviewJob(summary.jobId, Date.now(), REVIEW_JOB_POLL_INTERVAL_MS);
        return;
      }
      finishReviewJob(summary);
    }).catch((nextError) => {
      if (vaultSessionId.current === sessionId) {
        setReviewJob(null);
        setError(commandErrorMessage(nextError));
      }
    });
  };

  return {
    acceptCandidate,
    acknowledge,
    cancelEdit,
    cancelRemove,
    changeEdit,
    closeDetail,
    confirmRemove,
    enqueueBatch,
    handleConflict,
    openDetail,
    requestRemove,
    saveEdit,
    startEdit,
    toggleSelection,
  };
}
