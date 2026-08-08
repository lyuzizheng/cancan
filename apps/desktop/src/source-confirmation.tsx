import { useState } from "react";

import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
  MonogramTile,
  Panel,
  Select,
} from "@cancan/ui";

import type {
  MoneySourceSummary,
  SourceConfirmationPrompt,
} from "./command-contracts";
import { providerDisplayName, providerSuggestedSourceType } from "./format";

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
    <ul className="m-0 grid list-none gap-3 p-0">
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
    <li>
      <Panel className="flex gap-3 p-4">
        <MonogramTile className="shrink-0" name={displayName} size={32} />
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium text-ledger-ink">
            {parked ? `Kept unassigned: ${displayName}` : `New source detected: ${displayName}`}
          </p>
          <p className="mt-0.5 font-mono text-xs text-ledger-text-muted">
            {prompt.latestDocumentTitle !== null
              ? `${prompt.latestDocumentTitle} · ${documentLine}`
              : documentLine}
          </p>
          <p className="mt-2 text-sm text-ledger-text-muted">
            {parked
              ? `This evidence stays parked and unassigned. Create ${displayName} or choose an existing source when you are ready to route it.`
              : `CanCan detected ${displayName} in this evidence, but no ${displayName} Money Source exists yet. Confirm it once and CanCan routes the waiting evidence and keeps future ${displayName} statements together.`}
          </p>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Button
              disabled={busy}
              onClick={() => onConfirm(
                prompt,
                displayName,
                providerSuggestedSourceType(prompt.providerKey),
              )}
            >
              {confirming ? "Routing…" : "Create source and continue"}
            </Button>
            {existingSources.length > 0 ? (
              <Button
                aria-expanded={chooserOpen}
                disabled={busy}
                onClick={() => setChooserOpen((open) => !open)}
                variant="quiet"
              >
                Choose an existing source
              </Button>
            ) : null}
            {prompt.latestDocumentId !== null && onViewDocument !== undefined ? (
              <Button
                disabled={busy}
                onClick={() => onViewDocument(prompt)}
                variant="quiet"
              >
                View document
              </Button>
            ) : null}
            {!parked ? (
              <Button
                disabled={busy}
                onClick={() => onKeepUnassigned(prompt)}
                variant="text"
              >
                {parking ? "Parking…" : "Keep unassigned"}
              </Button>
            ) : null}
          </div>
          {chooserOpen && existingSources.length > 0 ? (
            <div className="mt-3 flex items-center gap-2">
              <div className="w-56">
                <Select
                  ariaLabel={`Existing Money Source for ${displayName}`}
                  disabled={busy}
                  onValueChange={setSelectedSourceId}
                  options={existingSources.map((source) => ({
                    label: source.displayName,
                    value: source.moneySourceId,
                  }))}
                  value={selectedSourceId}
                />
              </div>
              <Button
                disabled={busy || selectedSource === undefined}
                onClick={() => {
                  if (selectedSource !== undefined) {
                    onConfirm(prompt, selectedSource.displayName, selectedSource.sourceType);
                  }
                }}
                variant="strong"
              >
                {confirming ? "Routing…" : `Use ${selectedSource?.displayName ?? "this source"}`}
              </Button>
            </div>
          ) : null}
        </div>
      </Panel>
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
      <DialogContent onPointerDownOutside={(event) => event.preventDefault()}>
        <DialogTitle>Confirm detected source</DialogTitle>
        <DialogDescription>
          Route the waiting evidence to a Money Source, or keep it parked until you are ready.
        </DialogDescription>
        <div className="mt-4">
          <SourceConfirmationCardList
            busyKey={props.busyKey}
            existingSources={props.existingSources}
            onConfirm={props.onConfirm}
            onKeepUnassigned={props.onKeepUnassigned}
            onViewDocument={props.onViewDocument}
            prompts={props.focusedCandidate ? [props.focusedCandidate] : []}
          />
        </div>
      </DialogContent>
    </Dialog>
  );
}
