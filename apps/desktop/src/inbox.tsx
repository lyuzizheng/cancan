import { Button, EmptyState, SectionHeader, Skeleton } from "@cancan/ui";

import type { LocalInboxStatus } from "./command-contracts";
import { Feedback } from "./feedback";
import { localInboxScanSummaryText } from "./format";

export interface InboxPanelProps {
  busy: boolean;
  confirmingDisable: boolean;
  error: string | null;
  onCancelDisable: () => void;
  onChoose: () => void;
  onConfirmDisable: () => void;
  onRequestDisable: () => void;
  onRescan: () => void;
  onRetry: () => void;
  status: LocalInboxStatus | null;
}

/**
 * CanCan Inbox setup and status inside Sources. The folder picker, bookmark,
 * scan, and suppression rules stay host-owned; this panel only reflects the
 * sanitized status and asks for explicit user actions.
 */
export function InboxPanel(props: InboxPanelProps) {
  const status = props.status;
  const attention = status?.accessState === "needs_attention"
    || status?.accessState === "needs_reauthorization" || props.error !== null;
  return (
    <section aria-label="CanCan Inbox">
      <SectionHeader
        title="CanCan Inbox"
        tone={attention ? "attention" : "healthy"}
      />
      <div className="mt-2 border-t border-ledger-rule">
        {props.error !== null ? (
          <div className="pt-3">
            <Feedback
              action={props.busy ? undefined : props.onRetry}
              body={props.error}
              title="CanCan Inbox couldn’t be checked"
              tone="attention"
            />
          </div>
        ) : null}

        {status === null && props.error === null ? (
          <div className="grid gap-2.5 py-3" role="status">
            <span className="sr-only">Checking CanCan Inbox…</span>
            <Skeleton className="h-4 w-1/2" />
            <Skeleton className="h-4 w-1/3" />
          </div>
        ) : null}

        {status?.accessState === "disabled" ? (
          <EmptyState
            action={(
              <Button disabled={props.busy} onClick={props.onChoose}>
                {props.busy ? "Waiting for folder…" : "Choose Cancan folder"}
              </Button>
            )}
            body="Save bank statements to the Cancan folder in iCloud Drive, and CanCan adds new ones for you. CanCan only ever looks at the folder’s Inbox child; the folder stays outside your encrypted Vault and follows iCloud Drive’s privacy and security. CanCan also prepares a Backups folder next to Inbox — Backups aren’t set up yet, and Backups is never read for statements."
            icon="sources"
            title="Add statements without opening CanCan"
          />
        ) : null}

        {status?.accessState === "enabled" || status?.accessState === "paused" ? (
          <div className="py-3">
            <p className="text-sm font-medium text-ledger-ink">
              {status.accessState === "enabled"
                ? "CanCan Inbox is on"
                : "CanCan Inbox is paused"}
            </p>
            <p className="mt-0.5 text-sm text-ledger-text-muted">
              {status.accessState === "enabled"
                ? "New statements you save to Inbox are added for you. Your folder stays outside the encrypted Vault."
                : "CanCan Inbox resumes when your Vault is unlocked."}
            </p>
            <p className="mt-2 font-mono text-xs text-ledger-text-muted">
              {status.lastScan === null
                ? "No check yet. CanCan checks on unlock, when files change, and when you ask."
                : `Last check: ${localInboxScanSummaryText(status.lastScan)}`}
            </p>
            {status.backupsPrepared ? (
              <p className="mt-2 text-xs text-ledger-text-muted">
                A Backups folder is prepared next to Inbox. Backups aren’t set up yet.
              </p>
            ) : null}
            <InboxControls {...props} />
          </div>
        ) : null}

        {status?.accessState === "needs_reauthorization" ? (
          <div className="py-3">
            <p className="text-sm font-medium text-ledger-ink">
              CanCan can’t reach your Cancan folder
            </p>
            <p className="mt-0.5 text-sm text-ledger-text-muted">
              Choose it again to keep new statements flowing. Nothing was moved or deleted.
            </p>
            <InboxControls {...props} reauthorize />
          </div>
        ) : null}

        {status?.accessState === "needs_attention" ? (
          <div className="py-3">
            <p className="text-sm font-medium text-ledger-ink">
              CanCan Inbox needs attention
            </p>
            <p className="mt-0.5 text-sm text-ledger-text-muted">
              CanCan couldn’t prepare Inbox and Backups in your Cancan folder. A name may already
              exist there that isn’t a folder — CanCan never changes existing files. Check the
              folder, then choose it again.
            </p>
            <InboxControls {...props} reauthorize />
          </div>
        ) : null}
      </div>
    </section>
  );
}

function InboxControls({
  busy,
  confirmingDisable,
  onCancelDisable,
  onChoose,
  onConfirmDisable,
  onRequestDisable,
  onRescan,
  reauthorize = false,
}: InboxPanelProps & { reauthorize?: boolean }) {
  if (confirmingDisable) {
    return (
      <div
        aria-label="Confirm turn off"
        className="mt-3 flex flex-wrap items-center gap-2"
        role="group"
      >
        <span className="text-sm text-ledger-text-muted">
          Turn off CanCan Inbox? Your folder and files stay untouched.
        </span>
        <Button
          disabled={busy}
          onClick={onConfirmDisable}
          size="sm"
          variant="danger"
        >
          {busy ? "Turning off…" : "Turn off"}
        </Button>
        <Button
          disabled={busy}
          onClick={onCancelDisable}
          size="sm"
          variant="quiet"
        >
          Keep
        </Button>
      </div>
    );
  }
  return (
    <div className="mt-3 flex items-center gap-2">
      <Button
        disabled={busy}
        onClick={reauthorize ? onChoose : onRescan}
        size="sm"
        variant="quiet"
      >
        {busy
          ? reauthorize ? "Waiting for folder…" : "Checking…"
          : reauthorize ? "Choose again" : "Check now"}
      </Button>
      <Button
        disabled={busy}
        onClick={onRequestDisable}
        size="sm"
        variant="danger"
      >
        Turn off
      </Button>
    </div>
  );
}
