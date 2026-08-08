import { useState } from "react";

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
  prompt,
}: {
  busy: boolean;
  busyKey: string | null;
  existingSources: MoneySourceSummary[];
  onConfirm: SourceConfirmationCardListProps["onConfirm"];
  onKeepUnassigned: SourceConfirmationCardListProps["onKeepUnassigned"];
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
          className="button button-primary"
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
        {!parked ? (
          <button
            className="button button-quiet"
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
            className="button button-primary"
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
    </li>
  );
}
