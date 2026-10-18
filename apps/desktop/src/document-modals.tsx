import { useEffect, useRef } from "react";

import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogDescription,
  DialogTitle,
  Input,
} from "@cancan/ui";

import type {
  RenderedDocumentPage,
  SourceDocumentPreview,
  SourceDocumentSummary,
  StatementPasswordUnlockContext,
} from "./command-contracts";
import { Feedback } from "./feedback";

export interface DocumentViewerState {
  documentId: string;
  documentTitle: string;
  page: RenderedDocumentPage;
}

export interface DocumentPreviewState {
  documentId: string;
  documentTitle: string;
  preview: SourceDocumentPreview;
}

export interface DocumentUnlockState {
  busy: boolean;
  /** The host-confirmed owner of this document; `null` while unconfirmed or loading. */
  context: StatementPasswordUnlockContext | null;
  documentId: string;
  documentTitle: string;
  error: string | null;
  password: string;
  savedPasswordStatus: "invalid" | "unavailable" | null;
}

const fieldLabelClass = "mb-1 block text-xs font-medium text-ledger-text-muted";

export function DocumentUnlock({
  onClose,
  onPasswordChange,
  onRetry,
  onSubmit,
  state,
}: {
  onClose: () => void;
  onPasswordChange: (password: string) => void;
  onRetry: () => void;
  onSubmit: (updateSavedPassword: boolean) => void;
  state: DocumentUnlockState;
}) {
  const { context } = state;
  const confirmedSourceName = context?.moneySourceDisplayName ?? null;
  // Saving or replacing a password is offered only for the Money Source the
  // host confirmed for this document; the renderer never picks an owner.
  const canSaveForSource = context?.moneySourceId != null;
  const sourceLabel = confirmedSourceName ?? "this statement’s Money Source";
  const passwordInputRef = useRef<HTMLInputElement | null>(null);
  // The host context — and the bounded saved-password pass behind it — arrives
  // asynchronously, so this dialog first paints without a usable password
  // field. Once the field exists and is enabled, the prompt claims focus for
  // it exactly once: immediate typing reaches the password and Enter runs the
  // form's session-only submission instead of dismissing the prompt from a
  // control the person never chose. A context reset (retry) re-arms the claim;
  // a dismissed prompt unmounts this component, so a late resolution can
  // neither steal focus nor reopen anything.
  const passwordFocusClaimed = useRef(false);
  useEffect(() => {
    if (context === null) {
      passwordFocusClaimed.current = false;
      return;
    }
    if (state.busy || passwordFocusClaimed.current) {
      return;
    }
    const input = passwordInputRef.current;
    if (input === null) {
      return;
    }
    passwordFocusClaimed.current = true;
    input.focus();
  }, [context, state.busy]);

  return (
    <Dialog
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      open
    >
      <DialogContent
        onOpenAutoFocus={(event) => {
          // Radix would focus the only mounted control (Close) while the
          // context is still loading; hold focus on the panel itself so a
          // stray Enter can never cancel the prompt before the field exists.
          event.preventDefault();
          (event.currentTarget as HTMLElement | null)?.focus();
        }}
        onPointerDownOutside={(event) => event.preventDefault()}
      >
        <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
          Protected statement
        </p>
        <DialogTitle className="mt-1">Unlock {state.documentTitle}</DialogTitle>
        <DialogDescription>
          The password decrypts this statement locally in your Vault. It never leaves this Mac.
        </DialogDescription>
        <div className="mt-4">
          {context === null && !state.error ? (
            <p className="text-sm text-ledger-text-muted" role="status">
              Checking this statement’s Money Source…
            </p>
          ) : null}
          {context === null && state.error ? (
            <>
              <Feedback
                body={state.error}
                title="Statement password details couldn’t be loaded"
                tone="attention"
              />
              <div className="mt-3">
                <Button onClick={onRetry} variant="quiet">Try again</Button>
              </div>
            </>
          ) : null}
          {context ? (
            <form onSubmit={(event) => { event.preventDefault(); onSubmit(false); }}>
              <p className="text-sm text-ledger-text-muted">
                {canSaveForSource
                  ? confirmedSourceName
                    ? `This statement belongs to ${confirmedSourceName}.`
                    : "This statement belongs to a confirmed Money Source."
                  : "This statement’s Money Source isn’t confirmed yet, so CanCan can’t save its password. Use once still unlocks it for this Vault session."}
              </p>
              <label className={`${fieldLabelClass} mt-3`} htmlFor="statement-password">
                Statement password
              </label>
              <Input
                autoComplete="off"
                disabled={state.busy}
                id="statement-password"
                onChange={(event) => onPasswordChange(event.target.value)}
                ref={passwordInputRef}
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
                {canSaveForSource ? (
                  <Button
                    disabled={state.busy || !state.password}
                    onClick={() => onSubmit(true)}
                    type="button"
                  >
                    {context.hasSavedPassword ? "Update saved password" : "Save for this source"}
                  </Button>
                ) : null}
              </DialogActions>
              {canSaveForSource ? (
                <p className="mt-3 text-xs text-ledger-text-muted">
                  Use once is forgotten when the Vault locks.{" "}
                  {context.hasSavedPassword ? "Updating saves" : "Saving stores"} one verified
                  password for {sourceLabel} in this Mac’s Keychain.
                </p>
              ) : null}
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
