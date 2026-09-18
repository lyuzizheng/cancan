import {
  Button,
  LedgerHeader,
  MetricRow,
  Panel,
  SectionHeader,
  Skeleton,
} from "@cancan/ui";
import type {
  OperationalDiagnosticsCategory,
  OperationalDiagnosticsPreview,
} from "./command-contracts";
import { Feedback, type Notice } from "./feedback";
import type { IntakeNotifications } from "./use-intake-notifications";
import type { Diagnostics } from "./use-diagnostics";

export interface SettingsViewProps {
  diagnostics: Diagnostics;
  notifications: IntakeNotifications;
  onLock: () => void;
  notice: Notice | null;
  onRefresh: () => void;
}

/**
 * Settings route — today it owns the operational-log export and the opt-in
 * background intake notifications. The preview the host returns is the export's
 * own redacted content, so the panel renders `sampleLines` verbatim and never
 * re-processes them.
 */
export function SettingsView(props: SettingsViewProps) {
  const { diagnostics, notifications } = props;
  const preview = diagnostics.preview;
  const denied = notifications.settings?.enabled === true
    && notifications.settings.permission === "denied";

  return (
    <>
      <LedgerHeader
        actions={(
          <>
            <Button
              onClick={() => {
                props.onRefresh();
                notifications.load();
              }}
              variant="quiet"
            >
              Refresh
            </Button>
            <Button onClick={props.onLock} variant="quiet">
              Lock Vault
            </Button>
          </>
        )}
        eyebrow="Command Center"
        title="Settings"
      />

      <div className="grid gap-10 pt-6">
        {props.notice ? <Feedback {...props.notice} /> : null}

        <section aria-label="Background intake notifications">
          <SectionHeader
            tone={denied ? "attention" : "healthy"}
            title="Background intake notifications"
          />
          <div className="mt-2 border-t border-ledger-rule">
            <p className="pt-3 text-sm text-ledger-text-muted">
              With the window closed, CanCan can post one short notification
              after a batch of new files finishes — how many it processed and
              how many need your attention. The banner holds nothing else: no
              file names, no amounts, no accounts. Off by default, and turning
              it on is the only moment CanCan asks macOS for permission.
            </p>

            <label className="mt-3 flex w-fit cursor-pointer items-center gap-2 text-sm text-ledger-ink">
              <input
                checked={notifications.settings?.enabled === true}
                className="size-3.5 accent-accent-go-deep"
                disabled={notifications.busy || notifications.settings === null}
                onChange={(event) => notifications.setEnabled(event.target.checked)}
                type="checkbox"
              />
              <span>Background intake notifications</span>
            </label>

            {denied ? (
              <div className="mt-2 flex flex-wrap items-center gap-3">
                <p className="text-sm text-ledger-text-muted">
                  Notifications blocked by macOS. Background intake still runs
                  and nothing is posted until you allow notifications for CanCan.
                </p>
                <Button
                  disabled={notifications.busy}
                  onClick={notifications.openSystemSettings}
                  size="sm"
                  variant="quiet"
                >
                  Open System Settings
                </Button>
              </div>
            ) : null}

            {notifications.error !== null ? (
              <div className="pt-3">
                <Feedback
                  action={notifications.busy ? undefined : notifications.retry}
                  body={notifications.error}
                  title="Background intake notifications"
                  tone="attention"
                />
              </div>
            ) : null}
          </div>
        </section>

        <section aria-label="Operational log">
          <SectionHeader
            tone={diagnostics.error !== null ? "attention" : "healthy"}
            title="Operational log"
          />
          <div className="mt-2 border-t border-ledger-rule">
            <p className="pt-3 text-sm text-ledger-text-muted">
              CanCan keeps a short operational log on this Mac — what ran, what
              failed, and why. Entries stay local, are deleted automatically
              after 30 days, and are redacted before export. Nothing is sent
              anywhere.
            </p>

            {diagnostics.error !== null ? (
              <div className="pt-3">
                <Feedback
                  action={diagnostics.busy ? undefined : diagnostics.retry}
                  body={diagnostics.error}
                  title="Export diagnostics"
                  tone="attention"
                />
              </div>
            ) : null}

            {preview === null && diagnostics.pending !== "preview" ? (
              <div className="pt-3">
                <Button
                  disabled={diagnostics.busy}
                  onClick={diagnostics.loadPreview}
                >
                  Preview export
                </Button>
              </div>
            ) : null}

            {diagnostics.pending === "preview" && preview === null ? (
              <div className="grid gap-2.5 py-3" role="status">
                <span className="sr-only">Loading export preview…</span>
                <Skeleton className="h-4 w-1/3" />
                <Skeleton className="h-4 w-2/3" />
                <Skeleton className="h-4 w-1/2" />
              </div>
            ) : null}

            {preview !== null ? (
              <DiagnosticsPreview
                busy={diagnostics.busy}
                exporting={diagnostics.pending === "export"}
                onDismiss={diagnostics.dismissPreview}
                onExport={diagnostics.exportDiagnostics}
                preview={preview}
              />
            ) : null}
          </div>
        </section>
      </div>
    </>
  );
}

function DiagnosticsPreview({
  busy,
  exporting,
  onDismiss,
  onExport,
  preview,
}: {
  busy: boolean;
  exporting: boolean;
  onDismiss: () => void;
  onExport: () => void;
  preview: OperationalDiagnosticsPreview;
}) {
  const truncated = preview.entryCount > preview.exportLimit;
  return (
    <Panel className="mt-3 p-4">
      <p className="font-mono text-xs text-ledger-text-muted">
        CanCan {preview.appVersion} · {preview.retentionDays}-day retention
      </p>
      <div className="mt-1">
        <MetricRow
          label="Retained entries"
          meta={preview.oldestEntryAt !== null && preview.newestEntryAt !== null
            ? `${preview.oldestEntryAt} — ${preview.newestEntryAt}`
            : undefined}
          value={preview.entryCount}
        />
        {truncated ? (
          <MetricRow
            label="Entries per export"
            meta="The file holds the newest entries only"
            value={preview.exportLimit}
          />
        ) : null}
      </div>

      {preview.components.length > 0 || preview.errorCodes.length > 0 ? (
        <div className="grid gap-4 border-b border-ledger-rule py-3 sm:grid-cols-2">
          <CategoryList categories={preview.components} title="Components" />
          <CategoryList categories={preview.errorCodes} title="Error codes" />
        </div>
      ) : null}

      <p className="pt-3 font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        Export content
      </p>
      {preview.sampleLines.length > 0 ? (
        <pre className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap break-words rounded-sm border border-ledger-rule bg-ledger-mineral p-3 font-mono text-xs text-ledger-ink">
          {preview.sampleLines.join("\n")}
        </pre>
      ) : (
        <p className="mt-2 text-sm text-ledger-text-muted">
          No retained entries — the export contains only its header.
        </p>
      )}

      <div className="mt-3 flex items-center gap-2 border-t border-ledger-rule pt-3">
        <Button disabled={busy} onClick={onExport}>
          {exporting ? "Saving…" : "Export diagnostics"}
        </Button>
        <Button disabled={busy} onClick={onDismiss} variant="quiet">
          Dismiss
        </Button>
      </div>
    </Panel>
  );
}

function CategoryList({
  categories,
  title,
}: {
  categories: OperationalDiagnosticsCategory[];
  title: string;
}) {
  if (categories.length === 0) {
    return null;
  }
  return (
    <div>
      <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        {title}
      </p>
      <ul className="m-0 mt-1.5 grid list-none gap-1 p-0">
        {categories.map((category) => (
          <li
            className="flex items-baseline justify-between gap-3 text-sm text-ledger-ink"
            key={category.label}
          >
            <span className="min-w-0 truncate">{category.label}</span>
            <span className="shrink-0 tabular-nums">{category.count}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
