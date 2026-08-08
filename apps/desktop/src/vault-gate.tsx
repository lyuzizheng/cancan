import { Button, Icon, Input, Panel, StatusPoint } from "@cancan/ui";

/**
 * Vault gate — the locked/create moment. A porcelain panel replaces the old
 * emerald bar: a status point carries the signal, the title is a scoped
 * Fraunces display moment, and Touch ID is the one go-fill confirmation.
 */
export function VaultGate({
  body,
  busy,
  onPasswordChange,
  onSubmit,
  onUnlockWithKeychain,
  password,
  rememberedOnThisMac = false,
  title,
}: {
  body: string;
  busy: boolean;
  onPasswordChange?: (password: string) => void;
  onSubmit?: () => void;
  onUnlockWithKeychain?: () => void;
  password?: string;
  rememberedOnThisMac?: boolean | null;
  title: string;
}) {
  const acceptsPassword = onPasswordChange !== undefined && onSubmit !== undefined;
  return (
    <Panel
      aria-labelledby="vault-gate-title"
      className="mt-10 w-full max-w-lg p-6 sm:p-7"
      role="group"
    >
      <p className="flex items-center gap-2 font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        <StatusPoint tone={acceptsPassword ? "attention" : "idle"} />
        Local Vault
      </p>
      <h2 className="mt-3 font-serif text-xl font-display text-ledger-ink" id="vault-gate-title">
        {title}
      </h2>
      <p className="mt-2.5 text-sm text-ledger-text-muted">{body}</p>
      {rememberedOnThisMac === null ? (
        <p className="mt-4 text-sm text-ledger-text-muted">
          Touch ID unlock is unavailable. Use your Vault password.
        </p>
      ) : rememberedOnThisMac && onUnlockWithKeychain ? (
        <Button
          className="mt-4"
          disabled={busy}
          icon={<Icon name="lock" size={16} />}
          onClick={onUnlockWithKeychain}
          variant="strong"
        >
          Unlock with Touch ID
        </Button>
      ) : null}
      {acceptsPassword ? (
        <form
          className="mt-5 grid gap-2"
          onSubmit={(event) => { event.preventDefault(); onSubmit(); }}
        >
          <label className="text-sm font-medium text-ledger-ink" htmlFor="vault-password">
            Vault password
          </label>
          <div className="flex flex-wrap items-center gap-2.5">
            <div className="min-w-0 flex-1 basis-60">
              <Input
                autoComplete="off"
                disabled={busy}
                id="vault-password"
                onChange={(event) => onPasswordChange(event.target.value)}
                type="password"
                value={password}
              />
            </div>
            <Button disabled={busy} type="submit">
              {busy ? "Working…" : title === "Create your Vault" ? "Create Vault" : "Unlock Vault"}
            </Button>
          </div>
        </form>
      ) : (
        <p className="mt-5 text-sm text-ledger-text-muted" role="status">
          Checking Vault status…
        </p>
      )}
    </Panel>
  );
}
