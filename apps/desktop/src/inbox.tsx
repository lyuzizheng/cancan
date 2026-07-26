import type { LocalInboxStatus } from "./command-contracts";
import { localInboxScanSummaryText } from "./format";

export interface InboxPanelProps {
  busy: boolean;
  confirmingDisable: boolean;
  onCancelDisable: () => void;
  onChoose: () => void;
  onConfirmDisable: () => void;
  onRequestDisable: () => void;
  onRescan: () => void;
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
    || status?.accessState === "needs_reauthorization";
  return (
    <section className="inbox-panel" aria-labelledby="inbox-heading">
      <div className="inbox-panel-heading">
        <h2 id="inbox-heading">
          <span
            className={`panel-dot ${attention ? "panel-dot-amber" : "panel-dot-emerald"}`}
            aria-hidden="true"
          />
          CanCan Inbox
        </h2>
      </div>

      {status === null ? (
        <p className="panel-status" role="status">Checking CanCan Inbox…</p>
      ) : null}

      {status?.accessState === "disabled" ? (
        <div className="inbox-body">
          <p className="inbox-title">Add statements without opening CanCan</p>
          <p className="inbox-copy">
            Save bank statements to the Cancan folder in iCloud Drive, and CanCan adds new ones
            for you. CanCan only ever looks at the folder’s Inbox child; the folder stays outside
            your encrypted Vault and follows iCloud Drive’s privacy and security.
          </p>
          <p className="inbox-note">
            CanCan also prepares a Backups folder next to Inbox. Backups aren’t set up yet, and
            Backups is never read for statements.
          </p>
          <div className="inbox-actions">
            <button
              className="button button-primary"
              disabled={props.busy}
              onClick={props.onChoose}
              type="button"
            >
              {props.busy ? "Waiting for folder…" : "Choose Cancan folder"}
            </button>
          </div>
        </div>
      ) : null}

      {status?.accessState === "enabled" || status?.accessState === "paused" ? (
        <div className="inbox-body">
          <p className="inbox-title">
            {status.accessState === "enabled"
              ? "CanCan Inbox is on"
              : "CanCan Inbox is paused"}
          </p>
          <p className="inbox-copy">
            {status.accessState === "enabled"
              ? "New statements you save to Inbox are added for you. Your folder stays outside the encrypted Vault."
              : "CanCan Inbox resumes when your Vault is unlocked."}
          </p>
          <p className="inbox-scan">
            {status.lastScan === null
              ? "No check yet. CanCan checks on unlock, when files change, and when you ask."
              : `Last check: ${localInboxScanSummaryText(status.lastScan)}`}
          </p>
          {status.backupsPrepared ? (
            <p className="inbox-note">
              A Backups folder is prepared next to Inbox. Backups aren’t set up yet.
            </p>
          ) : null}
          <InboxControls {...props} />
        </div>
      ) : null}

      {status?.accessState === "needs_reauthorization" ? (
        <div className="inbox-body">
          <p className="inbox-title">CanCan can’t reach your Cancan folder</p>
          <p className="inbox-copy">
            Choose it again to keep new statements flowing. Nothing was moved or deleted.
          </p>
          <InboxControls {...props} reauthorize />
        </div>
      ) : null}

      {status?.accessState === "needs_attention" ? (
        <div className="inbox-body">
          <p className="inbox-title">CanCan Inbox needs attention</p>
          <p className="inbox-copy">
            CanCan couldn’t prepare Inbox and Backups in your Cancan folder. A name may already
            exist there that isn’t a folder — CanCan never changes existing files. Check the
            folder, then choose it again.
          </p>
          <InboxControls {...props} reauthorize />
        </div>
      ) : null}
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
      <div className="inbox-actions">
        <span className="inbox-disable-confirm" role="group" aria-label="Confirm turn off">
          <span>Turn off CanCan Inbox? Your folder and files stay untouched.</span>
          <button
            className="button button-quiet button-danger"
            disabled={busy}
            onClick={onConfirmDisable}
            type="button"
          >
            {busy ? "Turning off…" : "Turn off"}
          </button>
          <button
            className="button button-quiet"
            disabled={busy}
            onClick={onCancelDisable}
            type="button"
          >
            Keep
          </button>
        </span>
      </div>
    );
  }
  return (
    <div className="inbox-actions">
      <button
        className="button button-quiet"
        disabled={busy}
        onClick={reauthorize ? onChoose : onRescan}
        type="button"
      >
        {busy
          ? reauthorize ? "Waiting for folder…" : "Checking…"
          : reauthorize ? "Choose again" : "Check now"}
      </button>
      <button
        className="button button-quiet button-danger"
        disabled={busy}
        onClick={onRequestDisable}
        type="button"
      >
        Turn off
      </button>
    </div>
  );
}
