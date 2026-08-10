import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogDescription,
  DialogTitle,
  Input,
  Select,
} from "@cancan/ui";

import type {
  RenderedDocumentPage,
  SourceDocumentPreview,
  SourceDocumentSummary,
  StatementPasswordSourceSummary,
} from "./command-contracts";
import { Feedback } from "./feedback";

export interface DocumentViewerState {
  documentId: string;
  documentTitle: string;
  page: RenderedDocumentPage;
}

export interface DocumentPreviewState {
  documentTitle: string;
  preview: SourceDocumentPreview;
}

export interface DocumentUnlockState {
  busy: boolean;
  documentId: string;
  documentTitle: string;
  error: string | null;
  password: string;
  savedPasswordStatus: "invalid" | "unavailable" | null;
  selectedMoneySourceId: string;
  sources: StatementPasswordSourceSummary[] | null;
}

const fieldLabelClass = "mb-1 block text-xs font-medium text-ledger-text-muted";

export function DocumentUnlock({
  onClose,
  onPasswordChange,
  onRetrySources,
  onSourceChange,
  onSubmit,
  state,
}: {
  onClose: () => void;
  onPasswordChange: (password: string) => void;
  onRetrySources: () => void;
  onSourceChange: (moneySourceId: string) => void;
  onSubmit: (updateSavedPassword: boolean) => void;
  state: DocumentUnlockState;
}) {
  const hasSources = state.sources !== null && state.sources.length > 0;

  return (
    <Dialog
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      open
    >
      <DialogContent onPointerDownOutside={(event) => event.preventDefault()}>
        <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
          Protected statement
        </p>
        <DialogTitle className="mt-1">Unlock {state.documentTitle}</DialogTitle>
        <DialogDescription>
          The password decrypts this statement locally in your Vault. It never leaves this Mac.
        </DialogDescription>
        <div className="mt-4">
          {state.sources === null ? (
            <p className="text-sm text-ledger-text-muted" role="status">Loading Money Sources…</p>
          ) : null}
          {state.sources?.length === 0 && state.error ? (
            <>
              <Feedback
                body={state.error}
                title="Money Sources couldn’t be loaded"
                tone="attention"
              />
              <div className="mt-3">
                <Button onClick={onRetrySources} variant="quiet">Try again</Button>
              </div>
            </>
          ) : null}
          {state.sources?.length === 0 && !state.error ? (
            <Feedback
              body="Set up a Money Source before unlocking this protected statement. CanCan needs the source to scope saved passwords safely."
              title="No Money Source is configured"
              tone="attention"
            />
          ) : null}
          {hasSources ? (
            <form onSubmit={(event) => { event.preventDefault(); onSubmit(false); }}>
              <span className={fieldLabelClass}>Money Source</span>
              <Select
                ariaLabel="Money Source for this statement"
                disabled={state.busy}
                onValueChange={onSourceChange}
                options={(state.sources ?? []).map((source) => ({
                  label: `${source.displayName}${source.hasSavedPassword ? " — saved password" : ""}`,
                  value: source.moneySourceId,
                }))}
                placeholder="Choose a Money Source"
                value={state.selectedMoneySourceId}
              />
              <label className={`${fieldLabelClass} mt-3`} htmlFor="statement-password">
                Statement password
              </label>
              <Input
                autoComplete="off"
                disabled={state.busy || !state.selectedMoneySourceId}
                id="statement-password"
                onChange={(event) => onPasswordChange(event.target.value)}
                type="password"
                value={state.password}
              />
              {state.busy ? (
                <p className="mt-3 text-sm text-ledger-text-muted" role="status">
                  Trying the statement password locally…
                </p>
              ) : null}
              {state.savedPasswordStatus === "invalid" ? (
                <p className="mt-3 text-sm text-ledger-text-muted">
                  The saved password did not work. Enter the current password below.
                </p>
              ) : null}
              {state.savedPasswordStatus === "unavailable" ? (
                <p className="mt-3 text-sm text-ledger-text-muted">
                  The saved password is not available on this Mac. Enter it again below.
                </p>
              ) : null}
              {state.error ? (
                <p className="mt-3 text-sm text-signal-danger-text" role="alert">{state.error}</p>
              ) : null}
              <DialogActions>
                <Button disabled={state.busy || !state.password} type="submit" variant="quiet">
                  Use once
                </Button>
                <Button
                  disabled={state.busy || !state.password}
                  onClick={() => onSubmit(true)}
                  type="button"
                >
                  Update saved password
                </Button>
              </DialogActions>
              <p className="mt-3 text-xs text-ledger-text-muted">
                Use once is forgotten when the Vault locks. Updating saves one verified password
                for this Money Source in this Mac’s Keychain.
              </p>
            </form>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function DocumentViewer({
  onClose,
  onPage,
  viewer,
  viewingPage,
}: {
  onClose: () => void;
  onPage: (pageNumber: number) => void;
  viewer: DocumentViewerState;
  viewingPage: boolean;
}) {
  return (
    <Dialog
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      open
    >
      <DialogContent className="flex flex-col" width="3xl">
        <DialogTitle>{viewer.documentTitle}</DialogTitle>
        <DialogDescription>
          Rendered locally from encrypted evidence. Page buffers are released when this viewer closes.
        </DialogDescription>
        <div
          aria-busy={viewingPage}
          className="mt-4 min-h-0 flex-1 overflow-auto rounded-sm border border-ledger-rule bg-ledger-mineral"
        >
          <img
            alt={`Page ${viewer.page.pageNumber} of ${viewer.page.pageCount}`}
            className="mx-auto max-w-full"
            src={`data:image/png;base64,${viewer.page.pngBase64}`}
          />
          {viewingPage ? (
            <p className="p-3 text-sm text-ledger-text-muted" role="status">Rendering page…</p>
          ) : null}
        </div>
        <div className="mt-4 flex items-center justify-between gap-2">
          <Button
            disabled={viewingPage || viewer.page.pageNumber === 1}
            onClick={() => onPage(viewer.page.pageNumber - 1)}
            variant="quiet"
          >
            Previous
          </Button>
          <p className="text-sm text-ledger-text-muted">
            Page {viewer.page.pageNumber} of {viewer.page.pageCount}
          </p>
          <Button
            disabled={viewingPage || viewer.page.pageNumber === viewer.page.pageCount}
            onClick={() => onPage(viewer.page.pageNumber + 1)}
            variant="quiet"
          >
            Next
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function DocumentPreview({
  onClose,
  state,
}: {
  onClose: () => void;
  state: DocumentPreviewState;
}) {
  const { lineCount, previewLines, previewText, truncated } = state.preview;
  const lineUnit = lineCount === 1 ? "line" : "lines";
  return (
    <Dialog
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      open
    >
      <DialogContent className="flex flex-col" width="3xl">
        <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
          Encrypted evidence
        </p>
        <DialogTitle className="mt-1">{state.documentTitle}</DialogTitle>
        <DialogDescription>
          Text extracted locally from the encrypted Vault file. Nothing is uploaded for preview.
        </DialogDescription>
        <div className="mt-4 min-h-0 flex-1 overflow-auto rounded-sm border border-ledger-rule bg-ledger-mineral p-3">
          {lineCount === 0 ? (
            <p className="text-sm text-ledger-text-muted">This file is empty.</p>
          ) : (
            <pre className="whitespace-pre-wrap font-mono text-xs text-ledger-ink">{previewText}</pre>
          )}
        </div>
        <p className="mt-4 text-sm text-ledger-text-muted">
          {truncated
            ? `Preview truncated. Displaying content from ${previewLines} of ${lineCount} ${lineUnit}; the final displayed line may be partial. Save a copy to view the full file.`
            : `${lineCount} ${lineUnit}`}
        </p>
      </DialogContent>
    </Dialog>
  );
}

/**
 * Destructive confirmation for `Delete source file` (spec 0017): the current
 * encrypted Vault file is removed while the tombstone, record history, audit
 * trail, and ledger links remain.
 */
export function DeleteSourceDocumentDialog({
  deleting,
  document,
  onCancel,
  onConfirm,
}: {
  deleting: boolean;
  document: SourceDocumentSummary | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <Dialog
      onOpenChange={(open) => {
        if (!open && !deleting) {
          onCancel();
        }
      }}
      open={document !== null}
    >
      <DialogContent>
        <DialogTitle>Delete source file?</DialogTitle>
        <DialogDescription>
          {document ? `Delete the current Vault copy of ${document.originalFilename}?` : null}
        </DialogDescription>
        <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-ledger-text-muted">
          <li>The current Vault file will be deleted.</li>
          <li>The document registry entry, record history, audit trail, and ledger links remain.</li>
          <li>Linked views will show Source file deleted.</li>
          <li>Future backups will not include the deleted file.</li>
          <li>Older immutable backups or copies previously saved outside CanCan may still contain it.</li>
        </ul>
        <DialogActions>
          <Button disabled={deleting} onClick={onCancel} variant="quiet">
            Cancel
          </Button>
          <Button disabled={deleting} onClick={onConfirm} variant="danger">
            {deleting ? "Deleting…" : "Delete source file"}
          </Button>
        </DialogActions>
      </DialogContent>
    </Dialog>
  );
}
