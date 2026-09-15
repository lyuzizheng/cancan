import { ActionBar, Button, Input, Panel, Skeleton } from "@cancan/ui";

import type { ReviewItemSummary } from "./command-contracts";
import {
  eventTypeLabel,
  formatCurrencyAmount,
  formatLedgerDate,
} from "./format";
import type { ReviewDetailState, ReviewViewProps } from "./review";

/**
 * One review record's expanded detail: the evidence it came from, the edit,
 * remove, and acknowledge actions, and the related records it can link to.
 */
export function ReviewDetail({
  item,
  mutating,
  onAcceptCandidate,
  onAcknowledge,
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
  onAcknowledge: ReviewViewProps["onAcknowledge"];
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

      {item.recordCommitted ? (
        <div className="grid gap-3">
          <p className="m-0 text-sm text-ledger-text-muted">
            This record is already added. Acknowledging clears the check and leaves the added
            record exactly as it is.
          </p>
          <ActionBar align="start">
            <Button variant="primary" size="sm" disabled={mutating} onClick={onAcknowledge}>
              {mutating ? "Acknowledging…" : "Acknowledge"}
            </Button>
          </ActionBar>
        </div>
      ) : state.editing ? (
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

      {item.recordCommitted ? null : (
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
      )}
    </div>
  );
}
