import { useState } from "react";

import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@cancan/ui";

import type {
  MoneySourceSummary,
  SourceConfirmationPrompt,
} from "./command-contracts";
import { providerDisplayName, providerSuggestedSourceType, sourceInitials } from "./format";

export interface SourceConfirmationCardListProps {
  busyKey: string | null;
  existingSources: MoneySourceSummary[];
  onConfirm: (
    prompt: SourceConfirmationPrompt,
    displayName: string,
    sourceType: string,
  ) => void;
  onKeepUnassigned: (prompt: SourceConfirmationPrompt) => void;
  onViewDocument?: (prompt: SourceConfirmationPrompt) => void;
  prompts: SourceConfirmationPrompt[];
}

export function SourceConfirmationCardList(props: SourceConfirmationCardListProps) {
  if (props.prompts.length === 0) {
    return null;
  }
  return (
    <ul className="attention-card-list">
      {props.prompts.map((prompt) => (
        <SourceConfirmationCard
          busy={props.busyKey !== null}
          busyKey={props.busyKey}
          existingSources={props.existingSources}
          key={prompt.candidateId}
          onConfirm={props.onConfirm}
          onKeepUnassigned={props.onKeepUnassigned}
          onViewDocument={props.onViewDocument}
          prompt={prompt}
        />
      ))}
    </ul>
  );
}

function SourceConfirmationCard({
  busy,
  busyKey,
  existingSources,
  onConfirm,
  onKeepUnassigned,
  onViewDocument,
  prompt,
}: {
  busy: boolean;
  busyKey: string | null;
  existingSources: MoneySourceSummary[];
  onConfirm: SourceConfirmationCardListProps["onConfirm"];
  onKeepUnassigned: SourceConfirmationCardListProps["onKeepUnassigned"];
  onViewDocument: SourceConfirmationCardListProps["onViewDocument"];
  prompt: SourceConfirmationPrompt;
}) {
  const [chooserOpen, setChooserOpen] = useState(false);
  const [selectedSourceId, setSelectedSourceId] = useState<string>(
    () => existingSources[0]?.moneySourceId ?? "",
  );
  const displayName = providerDisplayName(prompt.providerKey);
  const parked = prompt.status === "kept_unassigned";
  const confirming = busyKey === `source-confirm:${prompt.candidateId}`;
  const parking = busyKey === `source-park:${prompt.candidateId}`;
  const documentLine = prompt.documentCount === 1
    ? "1 document waiting"
    : `${prompt.documentCount} documents waiting`;
  const selectedSource = existingSources.find(
    (source) => source.moneySourceId === selectedSourceId,
  );

  return (
    <li className="attention-card">
      <span className="attention-tile" aria-hidden="true">
        {sourceInitials(displayName)}
      </span>
      <div className="attention-card-body">
        <p className="attention-card-title">
          {parked ? `Kept unassigned: ${displayName}` : `New source detected: ${displayName}`}
        </p>
        <p className="source-confirm-meta">
          {prompt.latestDocumentTitle !== null
            ? `${prompt.latestDocumentTitle} · ${documentLine}`
            : documentLine}
        </p>
        <p className="attention-card-copy">
          {parked
            ? `This evidence stays parked and unassigned. Create ${displayName} or choose an existing source when you are ready to route it.`
            : `CanCan detected ${displayName} in this evidence, but no ${displayName} Money Source exists yet. Confirm it once and CanCan routes the waiting evidence and keeps future ${displayName} statements together.`}
        </p>
        <div className="attention-card-actions">
          <button
            className="button button-strong"
            disabled={busy}
            onClick={() => onConfirm(
              prompt,
              displayName,
              providerSuggestedSourceType(prompt.providerKey),
            )}
            type="button"
          >
            {confirming ? "Routing…" : "Create source and continue"}
          </button>
          {existingSources.length > 0 ? (
            <button
              aria-expanded={chooserOpen}
              className="button button-quiet"
              disabled={busy}
              onClick={() => setChooserOpen((open) => !open)}
              type="button"
            >
              Choose an existing source
            </button>
          ) : null}
          {prompt.latestDocumentId !== null && onViewDocument !== undefined ? (
            <button
              className="button button-quiet"
              disabled={busy}
              onClick={() => onViewDocument(prompt)}
              type="button"
            >
              View document
            </button>
          ) : null}
          {!parked ? (
            <button
              className="button button-text"
              disabled={busy}
              onClick={() => onKeepUnassigned(prompt)}
              type="button"
            >
              {parking ? "Parking…" : "Keep unassigned"}
            </button>
          ) : null}
        </div>
        {chooserOpen && existingSources.length > 0 ? (
          <div className="source-confirm-chooser">
            <select
              aria-label={`Existing Money Source for ${displayName}`}
              disabled={busy}
              onChange={(event) => setSelectedSourceId(event.target.value)}
              value={selectedSourceId}
            >
              {existingSources.map((source) => (
                <option key={source.moneySourceId} value={source.moneySourceId}>
                  {source.displayName}
                </option>
              ))}
            </select>
            <button
              className="button button-strong"
              disabled={busy || selectedSource === undefined}
              onClick={() => {
                if (selectedSource !== undefined) {
                  onConfirm(prompt, selectedSource.displayName, selectedSource.sourceType);
                }
              }}
              type="button"
            >
              {confirming ? "Routing…" : `Use ${selectedSource?.displayName ?? "this source"}`}
            </button>
          </div>
        ) : null}
      </div>
    </li>
  );
}

export interface FocusedSourceConfirmationDialogProps {
  busyKey: string | null;
  existingSources: MoneySourceSummary[];
  focusedCandidate: SourceConfirmationPrompt | undefined;
  onClose: () => void;
  onConfirm: SourceConfirmationCardListProps["onConfirm"];
  onKeepUnassigned: (prompt: SourceConfirmationPrompt) => void;
  onViewDocument: (prompt: SourceConfirmationPrompt) => void;
}

/**
 * Focused modal for a single detected source — the deep-link target of
 * `source_confirmation` task rows (spec 0006). Confirming or parking the
 * candidate removes it from the host's prompts, which closes the dialog.
 */
export function FocusedSourceConfirmationDialog(props: FocusedSourceConfirmationDialogProps) {
  return (
    <Dialog
      onOpenChange={(open) => {
        if (!open) {
          props.onClose();
        }
      }}
      open={props.focusedCandidate !== undefined}
    >
      <DialogContent>
        <DialogTitle>Confirm detected source</DialogTitle>
        <DialogDescription>
          Route the waiting evidence to a Money Source, or keep it parked until you are ready.
        </DialogDescription>
        <SourceConfirmationCardList
          busyKey={props.busyKey}
          existingSources={props.existingSources}
          onConfirm={props.onConfirm}
          onKeepUnassigned={props.onKeepUnassigned}
          onViewDocument={props.onViewDocument}
          prompts={props.focusedCandidate ? [props.focusedCandidate] : []}
        />
      </DialogContent>
    </Dialog>
  );
}
