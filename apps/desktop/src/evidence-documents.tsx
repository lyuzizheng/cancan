import { Button, StatusPoint, type StatusPointTone } from "@cancan/ui";

import type { SourceDocumentSummary } from "./command-contracts";
import type { SourcesViewProps } from "./sources-view";

/**
 * The evidence list under a Money Source: documents grouped by the month they
 * arrived, with the per-document status and the viewer, parser, copy, and
 * delete actions.
 */
export function EvidenceDocumentGroups({
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

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
] as const;

const BYTE_UNIT = 1024;

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
  return `${date.getUTCDate()} ${(MONTH_NAMES[date.getUTCMonth()] ?? "???").slice(0, 3)} ${date.getUTCFullYear()}`;
}

function formatByteSize(bytes: number) {
  if (bytes < BYTE_UNIT) {
    return `${bytes} B`;
  }
  if (bytes < BYTE_UNIT * BYTE_UNIT) {
    return `${Math.round(bytes / BYTE_UNIT)} KB`;
  }
  return `${(bytes / (BYTE_UNIT * BYTE_UNIT)).toFixed(1)} MB`;
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
          ? `${MONTH_NAMES[date.getUTCMonth()] ?? "Unknown"} ${date.getUTCFullYear()}`
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
