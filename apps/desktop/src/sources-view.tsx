import {
  Button,
  EmptyState,
  LedgerHeader,
  MonogramTile,
  Panel,
  SectionHeader,
  Skeleton,
  StatusPoint,
  type StatusPointTone,
} from "@cancan/ui";

import type {
  AccountConfirmationPrompt,
  CandidateAccountDecisionInput,
  LocalInboxStatus,
  MoneySourceSummary,
  SourceConfirmationPrompt,
  SourceDocumentSummary,
} from "./command-contracts";
import { AccountConfirmationCard } from "./attention";
import { Feedback, type Notice } from "./feedback";
import { InboxPanel } from "./inbox";
import { SourceConfirmationCardList } from "./source-confirmation";

export interface MoneySourceDocuments {
  documents: SourceDocumentSummary[] | null;
  source: MoneySourceSummary;
}

export interface SourcesViewProps {
  accountPrompts: AccountConfirmationPrompt[];
  attentionBusyKey: string | null;
  busy: boolean;
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
  onDecideAccounts: (
    prompt: AccountConfirmationPrompt,
    decisions: CandidateAccountDecisionInput[],
  ) => void;
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
  onRequestDelete: (document: SourceDocumentSummary) => void;
  onRestoreAccount: (accountId: string) => void;
  onSelectMoneySource: (moneySourceId: string) => void;
  onSaveRecoveryFile: () => void;
  onSaveSourceCopy: (documentId: string) => void;
  onView: (
    document: SourceDocumentSummary,
    trigger: HTMLButtonElement,
  ) => void;
  onViewPromptDocument: (prompt: SourceConfirmationPrompt) => void;
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
  const attentionCount = props.unassignedDocuments.length
    + props.sourcePrompts.filter((prompt) => prompt.status === "pending").length
    + props.accountPrompts.reduce(
      (count, prompt) => count + prompt.candidateAccounts.length,
      0,
    );
  return (
    <>
      <LedgerHeader
        actions={
          <>
            <label className="flex h-8 cursor-pointer items-center gap-2 px-1 text-sm text-ledger-text-muted">
              <input
                checked={props.rememberedOnThisMac === true}
                className="size-3.5 accent-accent-go-deep"
                disabled={props.busy || props.normalizingDocumentId !== null || props.rememberedOnThisMac === null}
                onChange={(event) => props.onRememberedChange(event.target.checked)}
                type="checkbox"
              />
              <span>{props.updatingRemembered ? "Updating…" : props.rememberedOnThisMac === null ? "Touch ID unavailable" : "Unlock with Touch ID"}</span>
            </label>
            <Button onClick={props.onRefresh} variant="quiet">
              Refresh
            </Button>
            <Button disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onLock} variant="quiet">
              Lock Vault
            </Button>
            <Button disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onImport}>
              {props.importing ? "Opening picker…" : "Add file"}
            </Button>
          </>
        }
        eyebrow="Sources / Evidence"
        title="Secure file intake"
      />

      <div className="grid gap-10 pt-6">
        <p className="text-sm text-ledger-text-muted">
          Add a PDF, CSV, PNG, or JPEG — CanCan saves it in your Vault before checking its configured source.
        </p>

        {props.notice ? <Feedback {...props.notice} /> : null}

        {!props.recoveryConfigured ? (
          <section aria-label="To do">
            <Panel className="flex items-center gap-4 p-4">
              <StatusPoint className="shrink-0" tone="attention" />
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium text-ledger-ink">Save your recovery file</p>
                <p className="mt-0.5 text-sm text-ledger-text-muted">
                  Use it to recover your Vault if you lose access to this Mac or forget your password. Anyone with the file can recover compatible Vault data, so store it privately.
                </p>
              </div>
              <Button
                disabled={props.busy || props.normalizingDocumentId !== null}
                onClick={props.onSaveRecoveryFile}
              >
                {props.savingRecoveryFile ? "Saving…" : "Save recovery file"}
              </Button>
            </Panel>
          </section>
        ) : null}

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

        <section aria-label="Money Sources">
          <SectionHeader
            count={props.sourceDocuments.length > 0 ? props.sourceDocuments.length : undefined}
            countUnit="source"
            title="Money Sources"
            tone="healthy"
          />
          {props.loadingDocuments ? (
            <div className="mt-2 grid gap-2.5 border-t border-ledger-rule py-3" role="status">
              <span className="sr-only">Refreshing sources…</span>
              <Skeleton className="h-9 w-full" />
              <Skeleton className="h-9 w-full" />
            </div>
          ) : null}
          {!props.loadingDocuments && props.sourceDocuments.length === 0 ? (
            <div className="mt-2 border-t border-ledger-rule">
              <EmptyState
                body="Money Sources appear here once evidence is confirmed to a bank, card, or wallet."
                icon="sources"
                title="No Money Sources yet"
              />
            </div>
          ) : null}
          <div className="mt-2">
            {props.sourceDocuments.map(({ documents, source }) => {
              const selected = props.selectedMoneySourceId === source.moneySourceId;
              return (
                <section
                  aria-label={source.displayName}
                  className="border-t border-ledger-rule py-3"
                  key={source.moneySourceId}
                >
                  <div className="flex items-center gap-3">
                    <MonogramTile className="shrink-0" name={source.displayName} />
                    <div className="min-w-0 flex-1">
                      <h3 className="text-md font-medium text-ledger-ink">{source.displayName}</h3>
                      <p className="mt-0.5 text-xs text-ledger-text-muted">{sourceTypeLabel(source.sourceType)}</p>
                    </div>
                    <Button
                      aria-expanded={selected}
                      aria-label={`View documents for ${source.displayName}`}
                      disabled={props.loadingDocuments}
                      onClick={() => props.onSelectMoneySource(source.moneySourceId)}
                      size="sm"
                      variant="quiet"
                    >
                      {selected && documents !== null
                        ? "Refresh documents"
                        : "View documents"}
                    </Button>
                  </div>
                  {selected && documents === null && props.loadingDocuments ? (
                    <p className="py-3 text-sm text-ledger-text-muted" role="status">Loading documents…</p>
                  ) : null}
                  {selected && documents?.length === 0 ? (
                    <p className="py-3 text-sm text-ledger-text-muted">
                      No {source.displayName} documents yet. Add a file or set up CanCan Inbox.
                    </p>
                  ) : null}
                  {selected && documents && documents.length > 0 ? (
                    <EvidenceDocumentGroups documents={documents} props={props} />
                  ) : null}
                </section>
              );
            })}
          </div>
        </section>

        <section aria-label="Needs attention">
          <SectionHeader
            count={attentionCount > 0 ? attentionCount : undefined}
            countUnit="item"
            title="Needs attention"
            tone={attentionCount > 0 ? "attention" : "healthy"}
          />
          <div className="mt-2 grid gap-4 border-t border-ledger-rule">
            {!props.loadingDocuments && attentionCount === 0 && props.sourcePrompts.length === 0 && props.accountPrompts.length === 0 ? (
              <p className="py-3 text-sm text-ledger-text-muted">
                No evidence needs your attention.
              </p>
            ) : null}
            <SourceConfirmationCardList
              busyKey={props.attentionBusyKey}
              existingSources={props.existingSources}
              onConfirm={props.onConfirmSourceCandidate}
              onKeepUnassigned={props.onKeepSourceCandidateUnassigned}
              onViewDocument={props.onViewPromptDocument}
              prompts={props.sourcePrompts}
            />
            {props.accountPrompts.length > 0 ? (
              <ul className="m-0 grid list-none gap-3 p-0">
                {props.accountPrompts.map((prompt) => (
                  <AccountConfirmationCard
                    busy={props.attentionBusyKey !== null}
                    deciding={props.attentionBusyKey === `account:${prompt.moneySourceId}`}
                    key={prompt.moneySourceId}
                    onDecide={(decisions) => props.onDecideAccounts(prompt, decisions)}
                    onRestore={props.onRestoreAccount}
                    prompt={prompt}
                    restoringAccountId={props.attentionBusyKey?.startsWith("restore:")
                      ? props.attentionBusyKey.slice("restore:".length)
                      : null}
                  />
                ))}
              </ul>
            ) : null}
            {props.unassignedDocuments.length > 0 ? (
              <div>
                <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
                  Unassigned evidence
                </p>
                <EvidenceDocumentGroups documents={props.unassignedDocuments} props={props} />
              </div>
            ) : null}
          </div>
        </section>
      </div>
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
    <section aria-label={group.label} className="mt-3" key={group.key}>
      <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        {group.label}
        <span className="normal-case"> · {group.documents.length} {group.documents.length === 1 ? "document" : "documents"}</span>
      </p>
      <ul className="m-0 mt-1 list-none border-t border-ledger-rule p-0">
        {group.documents.map((document) => {
          const passwordRequired = document.documentStatus === "needs_attention"
            && document.attentionReason === "password_required";
          const fileAvailable = document.fileState === "available";
          const viewingAvailable = fileAvailable && !passwordRequired;
          const routingAvailable = fileAvailable
            && (document.documentStatus === "ready" || document.documentStatus === "needs_attention")
            && !passwordRequired;
          const normalizing = props.normalizingDocumentId === document.documentId;
          const savingCopy = props.savingCopyDocumentId === document.documentId;
          return (
            <li
              className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-ledger-rule py-2.5"
              key={document.documentId}
            >
              <span className="shrink-0 rounded-sm border border-ledger-rule px-1.5 py-0.5 font-mono text-xs text-ledger-text-muted">
                {document.mimeType === "application/pdf" ? "PDF" : document.mimeType === "text/csv" ? "CSV" : document.mimeType === "image/png" ? "PNG" : "JPEG"}
              </span>
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-ledger-ink">{document.originalFilename}</p>
                <p className="mt-0.5 font-mono text-xs text-ledger-text-muted">{evidenceMeta(document)}</p>
              </div>
              <span className="flex shrink-0 items-center gap-1.5 text-xs text-ledger-text-muted">
                <StatusPoint tone={documentStatusTone(document)} />
                {documentStatusLabel(document)}
              </span>
              <div className="flex flex-wrap items-center gap-1.5">
                {passwordRequired ? (
                  <Button
                    disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null}
                    onClick={() => props.onOpenUnlock(document)}
                    size="sm"
                  >
                    Unlock
                  </Button>
                ) : (
                  <Button
                    disabled={!viewingAvailable || props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null}
                    onClick={(event) => props.onView(document, event.currentTarget)}
                    size="sm"
                    variant="quiet"
                  >
                    {!viewingAvailable ? "View unavailable" : "View document"}
                  </Button>
                )}
                {!passwordRequired ? (
                  <Button
                    disabled={!routingAvailable || props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null}
                    onClick={() => props.onNormalize(document.documentId)}
                    size="sm"
                    variant="quiet"
                  >
                    {!routingAvailable ? "Parser unavailable" : normalizing ? "Re-running…" : "Re-run parser"}
                  </Button>
                ) : null}
                {fileAvailable ? (
                  <Button
                    disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null}
                    onClick={() => props.onSaveSourceCopy(document.documentId)}
                    size="sm"
                    variant="quiet"
                  >
                    {savingCopy ? "Saving copy…" : "Save a copy"}
                  </Button>
                ) : null}
                {fileAvailable ? (
                  <Button
                    disabled={props.busy || props.normalizingDocumentId !== null || props.savingCopyDocumentId !== null}
                    onClick={() => props.onRequestDelete(document)}
                    size="sm"
                    variant="danger"
                  >
                    Delete source file
                  </Button>
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

function documentStatusTone(document: SourceDocumentSummary): StatusPointTone {
  switch (document.documentStatus) {
    case "needs_attention":
      return "attention";
    case "processing":
      return "idle";
    case "file_deleted":
      return "idle";
    case "missing":
      return "risk";
    case "ready":
      return "healthy";
  }
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

interface EvidenceMonthGroup {
  documents: SourceDocumentSummary[];
  key: string;
  label: string;
}

function groupEvidenceByMonth(
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
