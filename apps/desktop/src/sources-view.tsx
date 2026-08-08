import type {
  LocalInboxStatus,
  MoneySourceSummary,
  SourceConfirmationPrompt,
  SourceDocumentSummary,
} from "./command-contracts";
import { Feedback, type Notice } from "./feedback";
import { InboxPanel } from "./inbox";
import { SourceConfirmationCardList } from "./source-confirmation";

export interface MoneySourceDocuments {
  documents: SourceDocumentSummary[] | null;
  source: MoneySourceSummary;
}

export interface SourcesViewProps {
  attentionBusyKey: string | null;
  busy: boolean;
  deletingDocumentId: string | null;
  existingSources: MoneySourceSummary[];
  importing: boolean;
  inbox: LocalInboxStatus | null;
  inboxError: string | null;
  inboxBusy: boolean;
  inboxConfirmingDisable: boolean;
  loadingDocuments: boolean;
  normalizingDocumentId: string | null;
  notice: Notice | null;
  onConfirmSourceCandidate: (
    prompt: SourceConfirmationPrompt,
    displayName: string,
    sourceType: string,
  ) => void;
  onDelete: (documentId: string) => void;
  onImport: () => void;
  onInboxCancelDisable: () => void;
  onInboxChoose: () => void;
  onInboxConfirmDisable: () => void;
  onInboxRequestDisable: () => void;
  onInboxRescan: () => void;
  onInboxRetry: () => void;
  onKeepSourceCandidateUnassigned: (prompt: SourceConfirmationPrompt) => void;
  onLock: () => void;
  onNormalize: (documentId: string) => void;
  onOpenUnlock: (document: SourceDocumentSummary) => void;
  onRefresh: () => void;
  onRememberedChange: (remembered: boolean) => void;
  onSelectMoneySource: (moneySourceId: string) => void;
  onSaveRecoveryFile: () => void;
  onSaveSourceCopy: (documentId: string) => void;
  onView: (
    document: SourceDocumentSummary,
    trigger: HTMLButtonElement,
  ) => void;
  recoveryConfigured: boolean;
  rememberedOnThisMac: boolean | null;
  savingCopyDocumentId: string | null;
  savingRecoveryFile: boolean;
  selectedMoneySourceId: string | null;
  sourceDocuments: MoneySourceDocuments[];
  sourcePrompts: SourceConfirmationPrompt[];
  unassignedDocuments: SourceDocumentSummary[];
  updatingRemembered: boolean;
}

export function SourcesView(props: SourcesViewProps) {
  return (
    <>
      <header className="ledger-header">
        <div>
          <p className="ledger-eyebrow">Sources / Evidence</p>
          <h1>Secure file intake</h1>
        </div>
        <div className="ledger-actions">
          <label className="remember-vault-control">
            <input
              checked={props.rememberedOnThisMac === true}
              disabled={props.busy || props.normalizingDocumentId !== null || props.rememberedOnThisMac === null}
              onChange={(event) => props.onRememberedChange(event.target.checked)}
              type="checkbox"
            />
            <span>{props.updatingRemembered ? "Updating…" : props.rememberedOnThisMac === null ? "Touch ID unavailable" : "Unlock with Touch ID"}</span>
          </label>
          <button className="button button-quiet" disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onLock} type="button">
            Lock Vault
          </button>
          <button className="button button-primary" disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onImport} type="button">
            {props.importing ? "Opening picker…" : "Add file"}
          </button>
        </div>
      </header>

      <section className="intake-content" aria-label="Manual import">
        {!props.recoveryConfigured ? (
          <section className="todo-panel" aria-labelledby="todo-heading">
            <div className="todo-panel-heading">
              <h2 id="todo-heading"><span className="panel-dot panel-dot-amber" aria-hidden="true" />To do</h2>
              <span className="attention-count" aria-label="1 task">1</span>
            </div>
            <ul className="todo-list">
              <li className="todo-row">
                <div>
                  <p className="todo-title">Save your recovery file</p>
                  <p className="todo-copy">Use it to recover your Vault if you lose access to this Mac or forget your password. Anyone with the file can recover compatible Vault data, so store it privately.</p>
                </div>
                <button className="button button-primary" disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onSaveRecoveryFile} type="button">
                  {props.savingRecoveryFile ? "Saving…" : "Save recovery file"}
                </button>
              </li>
            </ul>
          </section>
        ) : null}

        <section className="intake-intro">
          <h2>Add a statement or export</h2>
          <p>Choose a PDF, CSV, PNG, or JPEG. CanCan saves it in your Vault before checking its configured source.</p>
        </section>

        {props.notice ? <Feedback {...props.notice} /> : null}

        <InboxPanel
          busy={props.inboxBusy}
          confirmingDisable={props.inboxConfirmingDisable}
          error={props.inboxError}
          onCancelDisable={props.onInboxCancelDisable}
          onChoose={props.onInboxChoose}
          onConfirmDisable={props.onInboxConfirmDisable}
          onRequestDisable={props.onInboxRequestDisable}
          onRescan={props.onInboxRescan}
          onRetry={props.onInboxRetry}
          status={props.inbox}
        />

        <section className="source-panel" aria-labelledby="sources-heading">
          <div className="source-panel-heading">
            <h2 id="sources-heading"><span className="panel-dot panel-dot-emerald" aria-hidden="true" />Money Sources</h2>
            <span className="source-count" aria-label={`${props.sourceDocuments.length} ${props.sourceDocuments.length === 1 ? "source" : "sources"}`}>
              {props.sourceDocuments.length}
            </span>
          </div>
          {props.loadingDocuments ? <p className="panel-status" role="status">Refreshing sources…</p> : null}
          {!props.loadingDocuments && props.sourceDocuments.length === 0 ? (
            <p className="panel-status">No Money Sources are configured yet.</p>
          ) : null}
          {props.sourceDocuments.map(({ documents, source }) => {
            const selected = props.selectedMoneySourceId === source.moneySourceId;
            return (
              <section className="source-documents" key={source.moneySourceId}>
                <div className="source-documents-heading">
                  <div>
                    <h3>{source.displayName}</h3>
                    <p>{sourceTypeLabel(source.sourceType)}</p>
                  </div>
                  <button
                    aria-expanded={selected}
                    aria-label={`View documents for ${source.displayName}`}
                    className="button button-quiet"
                    disabled={props.loadingDocuments}
                    onClick={() => props.onSelectMoneySource(source.moneySourceId)}
                    type="button"
                  >
                    {selected && documents !== null
                      ? "Refresh documents"
                      : "View documents"}
                  </button>
                </div>
              {selected && documents === null && props.loadingDocuments ? (
                <p className="panel-status" role="status">Loading documents…</p>
              ) : null}
              {selected && documents?.length === 0 ? (
                <p className="panel-status">No routed documents yet.</p>
              ) : null}
              {selected && documents && documents.length > 0 ? (
                <EvidenceDocumentGroups documents={documents} props={props} />
              ) : null}
              </section>
            );
          })}
        </section>

        <section className="attention-panel" aria-labelledby="attention-heading">
          <div className="attention-panel-heading">
            <h2 id="attention-heading"><span className="panel-dot panel-dot-amber" aria-hidden="true" />Needs attention</h2>
            <span className="attention-count" aria-label={`${props.unassignedDocuments.length + props.sourcePrompts.filter((prompt) => prompt.status === "pending").length} to check`}>
              {props.unassignedDocuments.length + props.sourcePrompts.filter((prompt) => prompt.status === "pending").length}
            </span>
          </div>

          {!props.loadingDocuments && props.unassignedDocuments.length === 0 && props.sourcePrompts.length === 0 ? (
            <p className="panel-status">No evidence needs your attention.</p>
          ) : null}
          <SourceConfirmationCardList
            busyKey={props.attentionBusyKey}
            existingSources={props.existingSources}
            onConfirm={props.onConfirmSourceCandidate}
            onKeepUnassigned={props.onKeepSourceCandidateUnassigned}
            prompts={props.sourcePrompts}
          />
          {props.unassignedDocuments.length > 0 ? (
            <EvidenceDocumentGroups documents={props.unassignedDocuments} props={props} />
          ) : null}
        </section>
      </section>
    </>
  );
}

function EvidenceDocumentGroups({
  documents,
  props,
}: {
  documents: SourceDocumentSummary[];
  props: SourcesViewProps;
}) {
  return groupEvidenceByMonth(documents).map((group) => (
    <section className="evidence-group" key={group.key}>
      <p className="evidence-group-label">
        {group.label}
        <span className="evidence-group-count">
          {group.documents.length} {group.documents.length === 1 ? "document" : "documents"}
        </span>
      </p>
      <ul className="evidence-list">
        {group.documents.map((document) => {
          const passwordRequired = document.documentStatus === "needs_attention"
            && document.attentionReason === "password_required";
          const fileAvailable = document.fileState === "available";
          const viewingAvailable = fileAvailable && !passwordRequired;
          const routingAvailable = fileAvailable
            && (document.documentStatus === "ready" || document.documentStatus === "needs_attention")
            && !passwordRequired;
          const attentionRequired = document.documentStatus === "needs_attention";
          const deleting = props.deletingDocumentId === document.documentId;
          const normalizing = props.normalizingDocumentId === document.documentId;
          const savingCopy = props.savingCopyDocumentId === document.documentId;
          return (
            <li className="evidence-row" key={document.documentId}>
              <span className="document-kind" aria-hidden="true">{document.mimeType === "application/pdf" ? "PDF" : document.mimeType === "text/csv" ? "CSV" : document.mimeType === "image/png" ? "PNG" : "JPEG"}</span>
              <div className="evidence-details">
                <p>{document.originalFilename}</p>
                <span className="evidence-meta">{evidenceMeta(document)}</span>
              </div>
              <p className={`doc-status doc-status-${document.documentStatus === "processing" ? "processing" : attentionRequired ? "attention" : document.fileState}`}>
                <span className="doc-status-dot" aria-hidden="true" />
                {documentStatusLabel(document)}
              </p>
              <div className="evidence-actions">
                {passwordRequired ? (
                  <button className="button button-primary" disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onOpenUnlock(document)} type="button">
                    Unlock
                  </button>
                ) : (
                  <button className="button button-quiet" disabled={!viewingAvailable || props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={(event) => props.onView(document, event.currentTarget)} type="button">
                    {!viewingAvailable ? "View unavailable" : "View document"}
                  </button>
                )}
                {!passwordRequired ? (
                  <button className="button button-quiet" disabled={!routingAvailable || props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onNormalize(document.documentId)} type="button">
                    {!routingAvailable ? "Parser unavailable" : normalizing ? "Re-running…" : "Re-run parser"}
                  </button>
                ) : null}
                {fileAvailable ? (
                  <button className="button button-quiet" disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onSaveSourceCopy(document.documentId)} type="button">
                    {savingCopy ? "Saving copy…" : "Save a copy"}
                  </button>
                ) : null}
                {fileAvailable ? (
                  <button className="button button-quiet button-danger" disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null} onClick={() => props.onDelete(document.documentId)} type="button">
                    {deleting ? "Deleting…" : "Delete source file"}
                  </button>
                ) : null}
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  ));
}

function sourceTypeLabel(sourceType: string) {
  return sourceType.replaceAll("_", " ").replace(/^./, (letter) => letter.toUpperCase());
}

function documentStatusLabel(document: SourceDocumentSummary) {
  switch (document.documentStatus) {
    case "needs_attention":
      return "Needs attention";
    case "processing":
      return "Processing";
    case "file_deleted":
      return "File deleted";
    case "missing":
      return "Missing";
    case "ready":
      return "Ready";
  }
}

const META_MONTHS = [
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
] as const;

const GROUP_MONTHS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
] as const;

const SQLITE_UTC_TIMESTAMP = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?$/;

export function parseReceivedAt(receivedAt: string) {
  const timestamp = SQLITE_UTC_TIMESTAMP.test(receivedAt)
    ? `${receivedAt.replace(" ", "T")}Z`
    : receivedAt;
  return new Date(timestamp);
}

function evidenceMeta(document: SourceDocumentSummary) {
  return `Added ${formatMetaDate(document.receivedAt)} · ${formatByteSize(document.byteSize)}`;
}

function formatMetaDate(iso: string) {
  const date = parseReceivedAt(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return `${date.getUTCDate()} ${META_MONTHS[date.getUTCMonth()]} ${date.getUTCFullYear()}`;
}

function formatByteSize(bytes: number) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export interface EvidenceMonthGroup {
  documents: SourceDocumentSummary[];
  key: string;
  label: string;
}

export function groupEvidenceByMonth(
  documents: SourceDocumentSummary[],
): EvidenceMonthGroup[] {
  const groups = new Map<string, EvidenceMonthGroup>();
  for (const document of documents) {
    const date = parseReceivedAt(document.receivedAt);
    const valid = !Number.isNaN(date.getTime());
    const key = valid
      ? `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}`
      : "unknown";
    const existing = groups.get(key);
    if (existing) {
      existing.documents.push(document);
    } else {
      groups.set(key, {
        documents: [document],
        key,
        label: valid
          ? `${GROUP_MONTHS[date.getUTCMonth()]} ${date.getUTCFullYear()}`
          : "Unknown date",
      });
    }
  }
  return [...groups.values()].sort((a, b) => {
    if (a.key === "unknown") {
      return 1;
    }
    if (b.key === "unknown") {
      return -1;
    }
    return b.key.localeCompare(a.key);
  });
}
