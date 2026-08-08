import { Button, LedgerHeader } from "@cancan/ui";

import type {
  RelationshipCandidateSummary,
  ReviewBatchGroupOutcome,
  ReviewItemDetail,
  ReviewItemSummary,
} from "./command-contracts";
import { Feedback, type Notice } from "./feedback";
import {
  batchGroupReasonLabel,
  batchGroupStatusLabel,
  eventTypeLabel,
  formatCurrencyAmount,
  formatLedgerDate,
  reviewReasonLabel,
} from "./format";

export interface ReviewEditState {
  accountBalanceDelta: string;
  amountValue: string;
  error: string | null;
  postedOn: string;
  saving: boolean;
}

export interface ReviewDetailState {
  candidates: RelationshipCandidateSummary[] | null;
  confirmingRemove: boolean;
  detail: ReviewItemDetail | null;
  editing: ReviewEditState | null;
  reviewItemId: string;
  summary: ReviewItemSummary;
}

export interface ReviewJobPanelState {
  jobId: string;
  outcomes: ReviewBatchGroupOutcome[];
  status: "done" | "failed" | "running";
}

export interface ReviewViewProps {
  detail: ReviewDetailState | null;
  items: ReviewItemSummary[] | null;
  job: ReviewJobPanelState | null;
  mutatingItemId: string | null;
  notice: Notice | null;
  onAcceptCandidate: (candidate: RelationshipCandidateSummary) => void;
  onCancelEdit: () => void;
  onCancelRemove: () => void;
  onClearSelection: () => void;
  onCloseDetail: () => void;
  onConfirmRemove: () => void;
  onEditChange: (patch: Partial<Pick<ReviewEditState, "accountBalanceDelta" | "amountValue" | "postedOn">>) => void;
  onEnqueue: () => void;
  onLock: () => void;
  onOpenDetail: (item: ReviewItemSummary) => void;
  onRefresh: () => void;
  onRemove: () => void;
  onSaveEdit: () => void;
  onSelectAll: () => void;
  onStartEdit: () => void;
  onToggleSelect: (reviewItemId: string) => void;
  selectedIds: ReadonlySet<string>;
}

export function ReviewView(props: ReviewViewProps) {
  const items = props.items;
  const committing = props.job?.status === "running";
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
        title="Review"
      />

      <section className="review-content" aria-label="Review queue">
        {props.notice ? <Feedback {...props.notice} /> : null}

        {props.job ? <ReviewJobPanel job={props.job} /> : null}

        <section className="review-panel" aria-labelledby="review-queue-heading">
          <div className="review-panel-heading">
            <h2 id="review-queue-heading">
              <span className="panel-dot panel-dot-amber" aria-hidden="true" />
              Needs your check
            </h2>
            {items !== null && items.length > 0 ? (
              <span className="attention-count" aria-label={`${items.length} ${items.length === 1 ? "record" : "records"}`}>
                {items.length}
              </span>
            ) : null}
          </div>

          {items === null ? (
            <p className="panel-status" role="status">Loading review items…</p>
          ) : null}

          {items !== null && items.length === 0 && !committing ? (
            <div className="panel-empty">
              <p>Nothing needs your check. New evidence that CanCan can’t place confidently lands here first.</p>
            </div>
          ) : null}

          {items !== null && items.length > 0 ? (
            <>
              <div className="review-batch-bar">
                <p className="review-batch-count">
                  {props.selectedIds.size === 0
                    ? "None selected"
                    : `${props.selectedIds.size} selected`}
                </p>
                <div className="review-batch-actions">
                  <button
                    className="button button-quiet"
                    disabled={committing || props.selectedIds.size === items.length}
                    onClick={props.onSelectAll}
                    type="button"
                  >
                    Select all
                  </button>
                  <button
                    className="button button-quiet"
                    disabled={committing || props.selectedIds.size === 0}
                    onClick={props.onClearSelection}
                    type="button"
                  >
                    Clear
                  </button>
                  <button
                    className="button button-primary"
                    disabled={committing || props.selectedIds.size === 0}
                    onClick={props.onEnqueue}
                    type="button"
                  >
                    {committing
                      ? "Adding…"
                      : props.selectedIds.size === 0
                      ? "Add selected"
                      : `Add selected (${props.selectedIds.size})`}
                  </button>
                </div>
              </div>

              <ul className="review-list">
                {items.map((item) => (
                  <ReviewRow
                    detail={props.detail?.reviewItemId === item.reviewItemId ? props.detail : null}
                    item={item}
                    key={item.reviewItemId}
                    mutating={props.mutatingItemId === item.reviewItemId}
                    onAcceptCandidate={props.onAcceptCandidate}
                    onCancelEdit={props.onCancelEdit}
                    onCancelRemove={props.onCancelRemove}
                    onCloseDetail={props.onCloseDetail}
                    onConfirmRemove={props.onConfirmRemove}
                    onEditChange={props.onEditChange}
                    onOpenDetail={props.onOpenDetail}
                    onRemove={props.onRemove}
                    onSaveEdit={props.onSaveEdit}
                    onStartEdit={props.onStartEdit}
                    onToggleSelect={props.onToggleSelect}
                    selected={props.selectedIds.has(item.reviewItemId)}
                    selectionDisabled={committing}
                  />
                ))}
              </ul>
            </>
          ) : null}
        </section>
      </section>
    </>
  );
}

function ReviewRow({
  detail,
  item,
  mutating,
  onAcceptCandidate,
  onCancelEdit,
  onCancelRemove,
  onCloseDetail,
  onConfirmRemove,
  onEditChange,
  onOpenDetail,
  onRemove,
  onSaveEdit,
  onStartEdit,
  onToggleSelect,
  selected,
  selectionDisabled,
}: {
  detail: ReviewDetailState | null;
  item: ReviewItemSummary;
  mutating: boolean;
  onAcceptCandidate: ReviewViewProps["onAcceptCandidate"];
  onCancelEdit: () => void;
  onCancelRemove: () => void;
  onCloseDetail: () => void;
  onConfirmRemove: () => void;
  onEditChange: ReviewViewProps["onEditChange"];
  onOpenDetail: ReviewViewProps["onOpenDetail"];
  onRemove: () => void;
  onSaveEdit: () => void;
  onStartEdit: () => void;
  onToggleSelect: (reviewItemId: string) => void;
  selected: boolean;
  selectionDisabled: boolean;
}) {
  const expanded = detail !== null;
  return (
    <li className={`review-row${expanded ? " review-row-expanded" : ""}`}>
      <div className="review-row-main">
        <label className="review-select">
          <input
            aria-label={`Select ${formatCurrencyAmount(item.currency, item.amountValue)} for ${item.accountLabel}`}
            checked={selected}
            disabled={selectionDisabled}
            onChange={() => onToggleSelect(item.reviewItemId)}
            type="checkbox"
          />
        </label>
        <div className="review-details">
          <p className="review-amount">{formatCurrencyAmount(item.currency, item.amountValue)}</p>
          <span className="review-meta">
            {item.accountLabel}
            {item.postedOn ? ` · ${formatLedgerDate(item.postedOn)}` : " · No date"}
          </span>
        </div>
        <p className="review-reason">
          <span className="review-reason-dot" aria-hidden="true" />
          {reviewReasonLabel(item.reasonCode)}
        </p>
        <button
          aria-expanded={expanded}
          className="button button-quiet"
          onClick={() => (expanded ? onCloseDetail() : onOpenDetail(item))}
          type="button"
        >
          {expanded ? "Close details" : "Review details"}
        </button>
      </div>
      {expanded ? (
        <ReviewDetail
          item={item}
          mutating={mutating}
          onAcceptCandidate={onAcceptCandidate}
          onCancelEdit={onCancelEdit}
          onCancelRemove={onCancelRemove}
          onConfirmRemove={onConfirmRemove}
          onEditChange={onEditChange}
          onRemove={onRemove}
          onSaveEdit={onSaveEdit}
          onStartEdit={onStartEdit}
          state={detail}
        />
      ) : null}
    </li>
  );
}

function ReviewDetail({
  item,
  mutating,
  onAcceptCandidate,
  onCancelEdit,
  onCancelRemove,
  onConfirmRemove,
  onEditChange,
  onRemove,
  onSaveEdit,
  onStartEdit,
  state,
}: {
  item: ReviewItemSummary;
  mutating: boolean;
  onAcceptCandidate: ReviewViewProps["onAcceptCandidate"];
  onCancelEdit: () => void;
  onCancelRemove: () => void;
  onConfirmRemove: () => void;
  onEditChange: ReviewViewProps["onEditChange"];
  onRemove: () => void;
  onSaveEdit: () => void;
  onStartEdit: () => void;
  state: ReviewDetailState;
}) {
  const { detail } = state;
  return (
    <div className="review-detail">
      {detail === null ? (
        <p className="panel-status" role="status">Loading details…</p>
      ) : (
        <dl className="review-detail-meta">
          <div>
            <dt>From</dt>
            <dd>{detail.sourceLabel}</dd>
          </div>
          <div>
            <dt>Document</dt>
            <dd>{detail.documentLabel}</dd>
          </div>
          <div>
            <dt>Type</dt>
            <dd>{eventTypeLabel(detail.eventType)}</dd>
          </div>
        </dl>
      )}

      {state.editing ? (
        <form
          className="review-edit"
          onSubmit={(event) => {
            event.preventDefault();
            onSaveEdit();
          }}
        >
          <div className="review-edit-grid">
            <label>
              Amount
              <input
                aria-label="Amount"
                disabled={state.editing.saving}
                inputMode="decimal"
                onChange={(event) => onEditChange({ amountValue: event.target.value })}
                value={state.editing.amountValue}
              />
            </label>
            <label>
              Date
              <input
                aria-label="Date"
                disabled={state.editing.saving}
                inputMode="numeric"
                onChange={(event) => onEditChange({ postedOn: event.target.value })}
                placeholder="2026-07-19"
                value={state.editing.postedOn}
              />
            </label>
            <label>
              Balance change
              <input
                aria-label="Balance change"
                disabled={state.editing.saving}
                inputMode="decimal"
                onChange={(event) => onEditChange({ accountBalanceDelta: event.target.value })}
                placeholder="Leave blank to keep current"
                value={state.editing.accountBalanceDelta}
              />
            </label>
          </div>
          <p className="review-edit-hint">
            Balance change is the signed effect on this account. Leave it blank to keep the current value.
          </p>
          {state.editing.error ? <p className="review-edit-error" role="alert">{state.editing.error}</p> : null}
          <div className="review-detail-actions">
            <button className="button button-primary" disabled={state.editing.saving} type="submit">
              {state.editing.saving ? "Saving…" : "Save edit"}
            </button>
            <button className="button button-quiet" disabled={state.editing.saving} onClick={onCancelEdit} type="button">
              Cancel
            </button>
          </div>
        </form>
      ) : (
        <div className="review-detail-actions">
          <button className="button button-quiet" disabled={mutating} onClick={onStartEdit} type="button">
            Edit record
          </button>
          {state.confirmingRemove ? (
            <span className="review-remove-confirm" role="group" aria-label="Confirm remove">
              <span>Remove this record from Review?</span>
              <button className="button button-quiet button-danger" disabled={mutating} onClick={onConfirmRemove} type="button">
                {mutating ? "Removing…" : "Confirm remove"}
              </button>
              <button className="button button-quiet" disabled={mutating} onClick={onCancelRemove} type="button">
                Keep
              </button>
            </span>
          ) : (
            <button className="button button-quiet button-danger" disabled={mutating} onClick={onRemove} type="button">
              Remove record
            </button>
          )}
        </div>
      )}

      <section className="review-candidates" aria-label="Looks related">
        <h3>Looks related</h3>
        {state.candidates === null ? (
          <p className="panel-status" role="status">Looking for related records…</p>
        ) : null}
        {state.candidates !== null && state.candidates.length === 0 ? (
          <p className="panel-status">No related records found.</p>
        ) : null}
        {state.candidates?.map((candidate) => (
          <div className="review-candidate" key={candidate.recordId}>
            <div className="review-candidate-compare">
              <div>
                <p className="review-candidate-label">This record</p>
                <p className="review-amount">{formatCurrencyAmount(item.currency, item.amountValue)}</p>
                <span className="review-meta">
                  {item.accountLabel}
                  {item.postedOn ? ` · ${formatLedgerDate(item.postedOn)}` : " · No date"}
                </span>
              </div>
              <div>
                <p className="review-candidate-label">{eventTypeLabel(candidate.eventType)}</p>
                <p className="review-amount">{formatCurrencyAmount(candidate.currency, candidate.amountValue)}</p>
                <span className="review-meta">
                  {candidate.accountLabel} · {formatLedgerDate(candidate.postedOn)}
                </span>
              </div>
            </div>
            <div className="review-candidate-actions">
              <button
                className="button button-quiet"
                disabled={mutating}
                onClick={() => onAcceptCandidate(candidate)}
                type="button"
              >
                {mutating ? "Linking…" : "Accept link"}
              </button>
            </div>
          </div>
        ))}
      </section>
    </div>
  );
}

function ReviewJobPanel({ job }: { job: ReviewJobPanelState }) {
  if (job.status === "running") {
    return (
      <section className="review-job" aria-live="polite" aria-label="Add records status">
        <p className="review-job-running">
          <span className="vault-status-light vault-status-loading" aria-hidden="true" />
          Adding your records…
        </p>
      </section>
    );
  }
  if (job.status === "failed") {
    return (
      <section className="review-job review-job-failed" aria-live="polite" aria-label="Add records status">
        <p>CanCan couldn’t finish adding records. Refresh and try again.</p>
      </section>
    );
  }
  const followUps = job.outcomes.filter((outcome) => outcome.status !== "committed");
  return (
    <section className="review-job" aria-live="polite" aria-label="Add records status">
      <p className="review-job-summary">{batchOutcomeSummary(job.outcomes)}</p>
      {followUps.length > 0 ? (
        <ul className="review-job-outcomes">
          {followUps.map((outcome, index) => (
            <li key={`${outcome.status}-${index}`}>
              <span className="review-job-status">{batchGroupStatusLabel(outcome.status)}</span>
              {` ${batchGroupRecordCount(outcome)} ${batchGroupRecordCount(outcome) === 1 ? "record" : "records"} — ${batchGroupReasonLabel(outcome.reason)}.`}
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

export interface BatchOutcomeSummary {
  added: number;
  alreadyAdded: number;
  changed: number;
  pending: number;
}

export function batchGroupRecordCount(outcome: ReviewBatchGroupOutcome): number {
  return Math.max(outcome.recordIds.length, 1);
}

export function summarizeBatchOutcomes(
  outcomes: ReviewBatchGroupOutcome[],
): BatchOutcomeSummary {
  const summary: BatchOutcomeSummary = { added: 0, alreadyAdded: 0, changed: 0, pending: 0 };
  for (const outcome of outcomes) {
    const count = batchGroupRecordCount(outcome);
    if (outcome.status === "committed") {
      summary.added += count;
    } else if (outcome.status === "already_committed") {
      summary.alreadyAdded += count;
    } else if (outcome.status === "stale") {
      summary.changed += count;
    } else {
      summary.pending += count;
    }
  }
  return summary;
}

export function batchOutcomeSummary(outcomes: ReviewBatchGroupOutcome[]): string {
  const summary = summarizeBatchOutcomes(outcomes);
  const parts: string[] = [];
  if (summary.added > 0) {
    parts.push(`Added ${summary.added} ${summary.added === 1 ? "record" : "records"}`);
  }
  if (summary.alreadyAdded > 0) {
    parts.push(`${summary.alreadyAdded === 1 ? "1 was" : `${summary.alreadyAdded} were`} already added`);
  }
  if (summary.pending > 0) {
    parts.push(`${summary.pending === 1 ? "1 still needs" : `${summary.pending} still need`} your check`);
  }
  if (summary.changed > 0) {
    parts.push(`${summary.changed === 1 ? "1 changed" : `${summary.changed} changed`} while adding`);
  }
  return parts.length === 0 ? "Nothing was added." : `${parts.join("; ")}.`;
}
