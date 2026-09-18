import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogDescription,
  DialogTitle,
  Input,
} from "@cancan/ui";

import { Feedback } from "./feedback";
import { sourceTypeLabel } from "./format";
import type { SourceDialogState } from "./use-vault-documents";

const fieldLabelClass = "mb-1 block text-xs font-medium text-ledger-text-muted";

export interface SourceDialogProps {
  onClose: () => void;
  onCreate: () => void;
  onDisplayNameChange: (displayName: string) => void;
  onProviderChange: (providerKey: string) => void;
  onRemovePassword: () => void;
  onRename: () => void;
  onRetryProviders: () => void;
  state: SourceDialogState;
}

/**
 * The Money Source management dialogs (GH #88): add a source for a supported
 * provider, rename one, or remove its saved statement password. Each dialog
 * keeps its own busy/error line so a failed command never strands the user on
 * a closed modal.
 */
export function SourceDialog({
  onClose,
  onCreate,
  onDisplayNameChange,
  onProviderChange,
  onRemovePassword,
  onRename,
  onRetryProviders,
  state,
}: SourceDialogProps) {
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
        {state.kind === "create" ? (
          <CreateMoneySourceForm
            onCreate={onCreate}
            onDisplayNameChange={onDisplayNameChange}
            onProviderChange={onProviderChange}
            onRetryProviders={onRetryProviders}
            state={state}
          />
        ) : null}
        {state.kind === "rename" ? (
          <RenameMoneySourceForm
            onDisplayNameChange={onDisplayNameChange}
            onRename={onRename}
            state={state}
          />
        ) : null}
        {state.kind === "remove_password" ? (
          <>
            <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
              Statement password
            </p>
            <DialogTitle className="mt-1">
              Remove the saved password for {state.source.displayName}?
            </DialogTitle>
            <DialogDescription>
              CanCan forgets the statement password stored in this Mac’s Keychain.
              Protected statements for this source ask for their password again.
            </DialogDescription>
            {state.error ? (
              <p className="mt-3 text-sm text-signal-danger-text" role="alert">{state.error}</p>
            ) : null}
            <DialogActions>
              <Button disabled={state.busy} onClick={onRemovePassword} variant="danger">
                {state.busy ? "Removing…" : "Remove saved password"}
              </Button>
            </DialogActions>
          </>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

function CreateMoneySourceForm({
  onCreate,
  onDisplayNameChange,
  onProviderChange,
  onRetryProviders,
  state,
}: {
  onCreate: () => void;
  onDisplayNameChange: (displayName: string) => void;
  onProviderChange: (providerKey: string) => void;
  onRetryProviders: () => void;
  state: Extract<SourceDialogState, { kind: "create" }>;
}) {
  const providers = state.providers;
  const selected = providers?.find(
    (provider) => provider.providerKey === state.providerKey,
  );
  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        onCreate();
      }}
    >
      <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        Money Source
      </p>
      <DialogTitle className="mt-1">Add a Money Source</DialogTitle>
      <DialogDescription>
        CanCan routes new evidence for the provider to this source automatically.
        Each provider supports one source.
      </DialogDescription>
      <div className="mt-4">
        {providers === null ? (
          <p className="text-sm text-ledger-text-muted" role="status">Loading providers…</p>
        ) : null}
        {providers !== null && providers.length === 0 ? (
          <>
            <Feedback
              body={state.error ?? "The supported providers couldn’t be loaded."}
              title="Providers unavailable"
              tone="attention"
            />
            <div className="mt-3">
              <Button onClick={onRetryProviders} type="button" variant="quiet">Try again</Button>
            </div>
          </>
        ) : null}
        {providers !== null && providers.length > 0 ? (
          <>
            <span className={fieldLabelClass} id="money-source-provider-label">Provider</span>
            <div aria-labelledby="money-source-provider-label" className="grid gap-1.5" role="radiogroup">
              {providers.map((provider) => {
                const configured = provider.configuredMoneySourceId !== null;
                return (
                  <label
                    className="flex cursor-pointer items-center gap-3 rounded-sm border border-ledger-rule px-3 py-2 transition duration-120 ease-mech has-checked:border-accent-go-deep has-checked:bg-ledger-mineral has-disabled:cursor-not-allowed has-disabled:opacity-60"
                    key={provider.providerKey}
                  >
                    <input
                      checked={state.providerKey === provider.providerKey}
                      className="size-3.5 accent-accent-go-deep"
                      disabled={state.busy || configured}
                      name="money-source-provider"
                      onChange={() => onProviderChange(provider.providerKey)}
                      type="radio"
                      value={provider.providerKey}
                    />
                    <span className="min-w-0 flex-1">
                      <span className="block text-sm font-medium text-ledger-ink">{provider.displayName}</span>
                      <span className="block text-xs text-ledger-text-muted">
                        {configured ? "Already configured" : sourceTypeLabel(provider.sourceType)}
                      </span>
                    </span>
                  </label>
                );
              })}
            </div>
            <label className={`${fieldLabelClass} mt-3`} htmlFor="money-source-name">
              Name <span className="text-ledger-text-muted">(optional)</span>
            </label>
            <Input
              autoComplete="off"
              disabled={state.busy}
              id="money-source-name"
              onChange={(event) => onDisplayNameChange(event.target.value)}
              placeholder={selected?.displayName ?? "Provider name"}
              value={state.displayName}
            />
            {state.error ? (
              <p className="mt-3 text-sm text-signal-danger-text" role="alert">{state.error}</p>
            ) : null}
            <DialogActions>
              <Button
                disabled={state.busy || state.providerKey === ""}
                type="submit"
              >
                {state.busy ? "Adding…" : "Add source"}
              </Button>
            </DialogActions>
          </>
        ) : null}
      </div>
    </form>
  );
}

function RenameMoneySourceForm({
  onDisplayNameChange,
  onRename,
  state,
}: {
  onDisplayNameChange: (displayName: string) => void;
  onRename: () => void;
  state: Extract<SourceDialogState, { kind: "rename" }>;
}) {
  const trimmed = state.displayName.trim();
  const unchanged = trimmed === "" || trimmed === state.source.displayName;
  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        onRename();
      }}
    >
      <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        Money Source
      </p>
      <DialogTitle className="mt-1">Rename {state.source.displayName}</DialogTitle>
      <DialogDescription>
        Only the label changes — the provider, documents, and account links stay put.
      </DialogDescription>
      <div className="mt-4">
        <label className={fieldLabelClass} htmlFor="money-source-rename">
          Name
        </label>
        <Input
          autoComplete="off"
          disabled={state.busy}
          id="money-source-rename"
          onChange={(event) => onDisplayNameChange(event.target.value)}
          value={state.displayName}
        />
        {state.error ? (
          <p className="mt-3 text-sm text-signal-danger-text" role="alert">{state.error}</p>
        ) : null}
        <DialogActions>
          <Button disabled={state.busy || unchanged} type="submit">
            {state.busy ? "Renaming…" : "Rename source"}
          </Button>
        </DialogActions>
      </div>
    </form>
  );
}
