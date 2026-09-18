import {
  Button,
  EmptyState,
  Icon,
  LedgerHeader,
  MonogramTile,
  SectionHeader,
  Skeleton,
  StatusPoint,
} from "@cancan/ui";
import type {
  MoneySourceDetail,
  MoneySourceSummary,
} from "./command-contracts";
import { EvidenceDocumentGroups } from "./evidence-documents";
import { Feedback, type Notice } from "./feedback";
import { providerDisplayName, sourceTypeLabel } from "./format";
import type { SourcesViewProps } from "./sources-view";

export interface SourceDetailViewProps {
  /** `null` while the detail command is in flight. */
  detail: MoneySourceDetail | null;
  /** True while any document/detail load is in flight. */
  loading: boolean;
  notice: Notice | null;
  /** The full Sources props, forwarded to the evidence document groups. */
  props: SourcesViewProps;
  source: MoneySourceSummary;
}

/**
 * The Money Source detail surface (0017): the source's own header, the
 * statement-password surface its actions offer, and its evidence documents
 * with their per-document actions. The renderer only ever sees the bounded
 * `MoneySourceDetail` projection — no bytes, locators, hashes, or secrets.
 */
export function SourceDetailView({
  detail,
  loading,
  notice,
  props,
  source,
}: SourceDetailViewProps) {
  const documents = detail?.documents ?? null;
  const passwordDocument = detail?.documents.find(
    (document) => document.documentStatus === "needs_attention"
      && document.attentionReason === "password_required",
  ) ?? null;
  const actionsDisabled = props.busy
    || props.normalizingDocumentId !== null
    || props.savingCopyDocumentId !== null;
  return (
    <>
      <LedgerHeader
        actions={
          <>
            <Button onClick={props.onRefresh} variant="quiet">
              Refresh
            </Button>
            <Button
              disabled={props.busy || props.normalizingDocumentId !== null}
              onClick={() => props.onOpenEditMoneySource(source)}
              variant="quiet"
            >
              Rename
            </Button>
            <Button disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onLock} variant="quiet">
              Lock Vault
            </Button>
            <Button disabled={props.busy || props.normalizingDocumentId !== null} onClick={props.onImport}>
              {props.importing ? "Opening picker…" : "Add file"}
            </Button>
          </>
        }
        eyebrow="Money Source"
        title={source.displayName}
      />

      <div className="grid gap-10 pt-6">
        <div className="flex items-center gap-3">
          <Button
            aria-label="Back to all Money Sources"
            icon={<Icon name="chevron-left" size={16} />}
            onClick={props.onClearMoneySourceSelection}
            size="sm"
            variant="text"
          >
            All sources
          </Button>
          <MonogramTile className="shrink-0" name={source.displayName} />
          <div className="min-w-0 flex-1">
            <p className="text-sm text-ledger-text-muted">
              {providerDisplayName(source.providerKey)} · {sourceTypeLabel(source.sourceType)}
            </p>
          </div>
        </div>

        {notice ? <Feedback {...notice} /> : null}

        {detail === null && loading ? (
          <div className="grid gap-2.5" role="status">
            <span className="sr-only">Loading {source.displayName}…</span>
            <Skeleton className="h-9 w-full" />
            <Skeleton className="h-9 w-full" />
            <Skeleton className="h-9 w-full" />
          </div>
        ) : null}
        {detail === null && !loading ? (
          <EmptyState
            body="Refresh to try again, or go back to all Money Sources."
            icon="sources"
            title={`Couldn’t load ${source.displayName}`}
          />
        ) : null}
        {detail !== null ? (
          <>
            <section aria-label="Statement password">
              <SectionHeader
                title="Statement password"
                tone={detail.actions.canEnterStatementPassword ? "attention" : "healthy"}
              />
              <div className="mt-2 flex items-center gap-4 border-t border-ledger-rule py-3">
                <StatusPoint
                  className="shrink-0"
                  tone={detail.actions.canEnterStatementPassword ? "attention" : "healthy"}
                />
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium text-ledger-ink">
                    {detail.actions.canEnterStatementPassword
                      ? "A statement is waiting for its password"
                      : detail.actions.hasSavedStatementPassword
                        ? "A statement password is saved on this Mac"
                        : "No statement password is saved"}
                  </p>
                  <p className="mt-0.5 text-sm text-ledger-text-muted">
                    {detail.actions.canEnterStatementPassword
                      ? "Enter it once to decrypt the waiting statement locally, or save it for this source."
                      : detail.actions.hasSavedStatementPassword
                        ? "CanCan tries it on new protected statements for this source before asking you."
                        : "Protected statements for this source ask for their password when they arrive."}
                  </p>
                </div>
                <div className="flex shrink-0 flex-wrap items-center gap-1.5">
                  {detail.actions.canEnterStatementPassword && passwordDocument !== null ? (
                    <Button
                      disabled={actionsDisabled}
                      onClick={() => props.onOpenUnlock(passwordDocument)}
                      size="sm"
                    >
                      Enter password
                    </Button>
                  ) : null}
                  {detail.actions.hasSavedStatementPassword ? (
                    <Button
                      disabled={actionsDisabled}
                      onClick={() => props.onOpenRemoveSourcePassword(source)}
                      size="sm"
                      variant="quiet"
                    >
                      Remove saved password
                    </Button>
                  ) : null}
                </div>
              </div>
            </section>

            <section aria-label="Documents">
              <SectionHeader
                count={documents !== null && documents.length > 0 ? documents.length : undefined}
                countUnit="document"
                title="Documents"
                tone="healthy"
              />
              {documents !== null && documents.length === 0 ? (
                <div className="mt-2 border-t border-ledger-rule">
                  <EmptyState
                    body={`No ${source.displayName} documents yet. Add a file or set up CanCan Inbox.`}
                    icon="sources"
                    title="No documents yet"
                  />
                </div>
              ) : null}
              {documents !== null && documents.length > 0 ? (
                <EvidenceDocumentGroups documents={documents} props={props} />
              ) : null}
            </section>
          </>
        ) : null}
      </div>
    </>
  );
}

