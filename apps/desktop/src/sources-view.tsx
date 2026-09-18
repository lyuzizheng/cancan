import {
  Button,
  EmptyState,
  LedgerHeader,
  MonogramTile,
  Panel,
  SectionHeader,
  Skeleton,
  StatusPoint,
} from "@cancan/ui";

import { memo } from "react";

import type {
  AccountConfirmationPrompt,
  CandidateAccountDecisionInput,
  LocalInboxStatus,
  MoneySourceDetail,
  MoneySourceSummary,
  SourceConfirmationPrompt,
  SourceDocumentSummary,
} from "./command-contracts";
import { AccountConfirmationCard } from "./attention";
import { EvidenceDocumentGroups } from "./evidence-documents";
import { Feedback, type Notice } from "./feedback";
import { InboxPanel } from "./inbox";
import { sourceTypeLabel } from "./format";
import { SourceDetailView } from "./source-detail-view";
import { SourceConfirmationCardList } from "./source-confirmation";

export interface MoneySourceDocuments {
  /** The source-detail projection; `null` until the source is opened. */
  detail: MoneySourceDetail | null;
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
  onClearMoneySourceSelection: () => void;
  onCreateMoneySource: () => void;
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
  onOpenEditMoneySource: (source: MoneySourceSummary) => void;
  onOpenRemoveSourcePassword: (source: MoneySourceSummary) => void;
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

export const SourcesView = memo(function SourcesView(props: SourcesViewProps) {
  const attentionCount = props.unassignedDocuments.length
    + props.sourcePrompts.filter((prompt) => prompt.status === "pending").length
    + props.accountPrompts.reduce(
      (count, prompt) => count + prompt.candidateAccounts.length,
      0,
    );
  const selectedEntry = props.selectedMoneySourceId === null
    ? undefined
    : props.sourceDocuments.find(
      (entry) => entry.source.moneySourceId === props.selectedMoneySourceId,
    );
  // A selected source owns the whole surface: the detail view replaces the
  // intake list until the user goes back (0017 — evidence lives under each
  // Money Source detail view).
  if (selectedEntry !== undefined) {
    return (
      <SourceDetailView
        detail={selectedEntry.detail}
        loading={props.loadingDocuments}
        notice={props.notice}
        props={props}
        source={selectedEntry.source}
      />
    );
  }
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
            action={
              <Button
                disabled={props.busy || props.normalizingDocumentId !== null}
                onClick={props.onCreateMoneySource}
                size="sm"
                variant="quiet"
              >
                Add source
              </Button>
            }
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
                body="Add a source for a supported provider, or confirm evidence to a bank, card, or wallet."
                icon="sources"
                title="No Money Sources yet"
              />
            </div>
          ) : null}
          <div className="mt-2">
            {props.sourceDocuments.map(({ source }) => (
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
                    aria-label={`Open ${source.displayName}`}
                    disabled={props.loadingDocuments}
                    onClick={() => props.onSelectMoneySource(source.moneySourceId)}
                    size="sm"
                    variant="quiet"
                  >
                    Open
                  </Button>
                </div>
              </section>
            ))}
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
});
