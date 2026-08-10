import {
  ActionBar,
  Badge,
  Button,
  EmptyState,
  Input,
  LedgerHeader,
  Panel,
  SectionHeader,
  Skeleton,
  StatusPoint,
} from "@cancan/ui";

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
  const reviewCount = items?.length ?? 0;
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

      <section className="grid gap-10 pt-6" aria-label="Review queue">
        {props.notice ? <Feedback {...props.notice} /> : null}

        {props.job ? <ReviewJobPanel job={props.job} /> : null}

        <section aria-label="Needs your check">
          <SectionHeader
            title="Needs your check"
            tone={reviewCount > 0 ? "attention" : "healthy"}
            count={reviewCount > 0 ? reviewCount : undefined}
            countUnit="record"
          />

          {items === null ? (
            <div role="status" className="mt-2 grid gap-3 border-t border-ledger-rule py-3">
              <span className="sr-only">Loading review items…</span>
              <Skeleton className="h-7 w-64" />
              <Skeleton className="h-11 w-full" />
              <Skeleton className="h-11 w-full" />
              <Skeleton className="h-11 w-full" />
            </div>
          ) : null}

          {items !== null && items.length === 0 && !committing ? (
            <div className="mt-2 border-t border-ledger-rule">
              <EmptyState
                icon="review"
                title="Nothing needs your check"
                body="New evidence that CanCan can’t place confidently lands here first."
              />
            </div>
          ) : null}

          {items !== null && items.length > 0 ? (
            <>
              <div className="mt-2 flex flex-wrap items-center justify-between gap-2 border-t border-ledger-rule py-3">
                <p className="m-0 text-sm text-ledger-text-muted">
                  {props.selectedIds.size === 0
                    ? "None selected"
                    : `${props.selectedIds.size} selected`}
                </p>
                <ActionBar>
                  <Button
                    variant="quiet"
                    size="sm"
                    disabled={committing || props.selectedIds.size === items.length}
                    onClick={props.onSelectAll}
                  >
                    Select all
                  </Button>
                  <Button
                    variant="quiet"
                    size="sm"
                    disabled={committing || props.selectedIds.size === 0}
                    onClick={props.onClearSelection}
                  >
                    Clear
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    disabled={committing || props.selectedIds.size === 0}
                    onClick={props.onEnqueue}
                  >
                    {committing
                      ? "Adding…"
                      : props.selectedIds.size === 0
                      ? "Add selected"
                      : `Add selected (${props.selectedIds.size})`}
                  </Button>
                </ActionBar>
              </div>

              <ul className="m-0 list-none p-0">
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
  const reasonLabel = reviewReasonLabel(item.reasonCode);
  return (
    <li className="border-b border-ledger-rule">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2 py-2.5">
        {/* The negative-margin label widens the 14px checkbox to a 26px
            pointer target (WCAG 2.5.8) without shifting the row layout;
            self-start keeps the box on the first text line. */}
        <label className="-m-1.5 flex shrink-0 cursor-pointer self-start p-1.5 has-[:disabled]:cursor-default">
          <input
            aria-label={`Select ${formatCurrencyAmount(item.currency, item.amountValue)} for ${item.accountLabel}`}
            checked={selected}
            className="mt-0.5 size-3.5 accent-accent-go-deep"
            disabled={selectionDisabled}
            onChange={() => onToggleSelect(item.reviewItemId)}
            type="checkbox"
          />
        </label>
        <div className="min-w-0 flex-1">
          <p className="m-0 text-md font-medium tabular-nums text-ledger-ink">
            {formatCurrencyAmount(item.currency, item.amountValue)}
          </p>
          <p className="m-0 mt-0.5 text-xs text-ledger-text-muted">
            {item.accountLabel}
            {item.postedOn ? ` · ${formatLedgerDate(item.postedOn)}` : " · No date"}
          </p>
        </div>
        <p className="m-0 flex items-center gap-1.5 text-xs text-ledger-text-muted">
          <StatusPoint tone="attention" />
          {reasonLabel}
        </p>
        <Button
          aria-expanded={expanded}
          variant="quiet"
          size="sm"
          onClick={() => (expanded ? onCloseDetail() : onOpenDetail(item))}
        >
          {expanded ? "Close details" : "Review details"}
        </Button>
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
    <div className="grid gap-4 border-t border-ledger-rule py-4">
      {detail === null ? (
        <div role="status" className="grid gap-2">
          <span className="sr-only">Loading details…</span>
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-2/3" />
        </div>
      ) : (
        <dl className="m-0 grid gap-3 sm:grid-cols-3">
          <div>
            <dt className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
              From
            </dt>
            <dd className="m-0 mt-0.5 text-sm text-ledger-ink">{detail.sourceLabel}</dd>
          </div>
          <div>
            <dt className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
              Document
            </dt>
            <dd className="m-0 mt-0.5 text-sm text-ledger-ink">{detail.documentLabel}</dd>
          </div>
          <div>
            <dt className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
              Type
            </dt>
            <dd className="m-0 mt-0.5 text-sm text-ledger-ink">{eventTypeLabel(detail.eventType)}</dd>
          </div>
        </dl>
      )}

      {state.editing ? (
        <form
          className="grid gap-3"
          onSubmit={(event) => {
            event.preventDefault();
            onSaveEdit();
          }}
        >
          <div className="grid gap-3 md:grid-cols-3">
            <label className="block text-xs font-medium text-ledger-text-muted">
              Amount
              <Input
                aria-label="Amount"
                className="mt-1"
                disabled={state.editing.saving}
                inputMode="decimal"
                numeric
                onChange={(event) => onEditChange({ amountValue: event.target.value })}
                value={state.editing.amountValue}
              />
            </label>
            <label className="block text-xs font-medium text-ledger-text-muted">
              Date
              <Input
                aria-label="Date"
                className="mt-1"
                disabled={state.editing.saving}
                inputMode="numeric"
                onChange={(event) => onEditChange({ postedOn: event.target.value })}
                placeholder="2026-07-19"
                value={state.editing.postedOn}
              />
            </label>
            <label className="block text-xs font-medium text-ledger-text-muted">
              Balance change
              <Input
                aria-label="Balance change"
                className="mt-1"
                disabled={state.editing.saving}
                inputMode="decimal"
                numeric
                onChange={(event) => onEditChange({ accountBalanceDelta: event.target.value })}
                placeholder="Leave blank to keep current"
                value={state.editing.accountBalanceDelta}
              />
            </label>
          </div>
          <p className="m-0 text-xs text-ledger-text-muted">
            Balance change is the signed effect on this account. Leave it blank to keep the current value.
          </p>
          {state.editing.error ? (
            <p className="m-0 text-sm text-signal-danger-text" role="alert">{state.editing.error}</p>
          ) : null}
          <ActionBar>
            <Button variant="primary" size="sm" disabled={state.editing.saving} type="submit">
              {state.editing.saving ? "Saving…" : "Save edit"}
            </Button>
            <Button variant="quiet" size="sm" disabled={state.editing.saving} onClick={onCancelEdit}>
              Cancel
            </Button>
          </ActionBar>
        </form>
      ) : (
        <ActionBar align="start">
          <Button variant="quiet" size="sm" disabled={mutating} onClick={onStartEdit}>
            Edit record
          </Button>
          {state.confirmingRemove ? (
            <span role="group" aria-label="Confirm remove" className="flex items-center gap-2">
              <span className="text-sm text-ledger-text-muted">Remove this record from Review?</span>
              <Button variant="danger" size="sm" disabled={mutating} onClick={onConfirmRemove}>
                {mutating ? "Removing…" : "Confirm remove"}
              </Button>
              <Button variant="quiet" size="sm" disabled={mutating} onClick={onCancelRemove}>
                Keep
              </Button>
            </span>
          ) : (
            <Button variant="danger" size="sm" disabled={mutating} onClick={onRemove}>
              Remove record
            </Button>
          )}
        </ActionBar>
      )}

      <section aria-label="Looks related" className="grid gap-3">
        <h3 className="m-0 font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
          Looks related
        </h3>
        {state.candidates === null ? (
          <div role="status" className="grid gap-2">
            <span className="sr-only">Looking for related records…</span>
            <Skeleton className="h-16 w-full" />
          </div>
        ) : null}
        {state.candidates !== null && state.candidates.length === 0 ? (
          <p className="m-0 text-sm text-ledger-text-muted">No related records found.</p>
        ) : null}
        {state.candidates?.map((candidate) => (
          <Panel className="grid gap-3 p-4" key={candidate.recordId}>
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <p className="m-0 font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
                  This record
                </p>
                <p className="m-0 mt-1 text-md font-medium tabular-nums text-ledger-ink">
                  {formatCurrencyAmount(item.currency, item.amountValue)}
                </p>
                <p className="m-0 mt-0.5 text-xs text-ledger-text-muted">
                  {item.accountLabel}
                  {item.postedOn ? ` · ${formatLedgerDate(item.postedOn)}` : " · No date"}
                </p>
              </div>
              <div>
                <p className="m-0 font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
                  {eventTypeLabel(candidate.eventType)}
                </p>
                <p className="m-0 mt-1 text-md font-medium tabular-nums text-ledger-ink">
                  {formatCurrencyAmount(candidate.currency, candidate.amountValue)}
                </p>
                <p className="m-0 mt-0.5 text-xs text-ledger-text-muted">
                  {candidate.accountLabel} · {formatLedgerDate(candidate.postedOn)}
                </p>
              </div>
            </div>
            <ActionBar align="start">
              <Button
                variant="quiet"
                size="sm"
                disabled={mutating}
                onClick={() => onAcceptCandidate(candidate)}
              >
                {mutating ? "Linking…" : "Accept link"}
              </Button>
            </ActionBar>
          </Panel>
        ))}
      </section>
    </div>
  );
}

function ReviewJobPanel({ job }: { job: ReviewJobPanelState }) {
  if (job.status === "running") {
    return (
      <Panel aria-live="polite" aria-label="Add records status" className="p-4">
        <p className="m-0 flex items-center gap-2.5 text-sm text-ledger-ink">
          <StatusPoint tone="idle" />
          Adding your records…
        </p>
      </Panel>
    );
  }
  if (job.status === "failed") {
    return (
      <Panel aria-live="polite" aria-label="Add records status" className="p-4">
        <p className="m-0 text-sm text-signal-danger-text">
          CanCan couldn’t finish adding records. Refresh and try again.
        </p>
      </Panel>
    );
  }
  const followUps = job.outcomes.filter((outcome) => outcome.status !== "committed");
  return (
    <Panel aria-live="polite" aria-label="Add records status" className="grid gap-3 p-4">
      <p className="m-0 text-sm text-ledger-ink">{batchOutcomeSummary(job.outcomes)}</p>
      {followUps.length > 0 ? (
        <ul className="m-0 grid list-none gap-2 p-0">
          {followUps.map((outcome, index) => (
            <li className="flex flex-wrap items-center gap-2 text-sm" key={`${outcome.status}-${index}`}>
              <Badge tone={outcome.status === "already_committed" ? "neutral" : "attention"}>
                {batchGroupStatusLabel(outcome.status)}
              </Badge>
              <span className="text-ledger-text-muted">
                {`${batchGroupRecordCount(outcome)} ${batchGroupRecordCount(outcome) === 1 ? "record" : "records"} — ${batchGroupReasonLabel(outcome.reason)}.`}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </Panel>
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
