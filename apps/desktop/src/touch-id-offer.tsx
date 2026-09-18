import { Button, Panel, StatusPoint } from "@cancan/ui";

/**
 * Post-password Touch ID offer — one dismissible panel shown after a password
 * unlock while remembered unlock is off (spec 0009). Accepting runs the same
 * host enable action as the Sources opt-in; declining only dismisses the
 * offer for this session.
 */
export function TouchIdOffer({
  busy,
  onAccept,
  onDismiss,
}: {
  busy: boolean;
  onAccept: () => void;
  onDismiss: () => void;
}) {
  return (
    <Panel
      aria-label="Touch ID offer"
      className="flex items-center gap-4 p-4"
      role="group"
    >
      <StatusPoint className="shrink-0" tone="idle" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-ledger-ink">
          Use Touch ID next time
        </p>
        <p className="mt-0.5 text-sm text-ledger-text-muted">
          CanCan can unlock your Vault with Touch ID without running the password check.
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <Button disabled={busy} onClick={onDismiss} variant="text">
          Not now
        </Button>
        <Button disabled={busy} onClick={onAccept}>
          {busy ? "Turning on…" : "Turn on Touch ID"}
        </Button>
      </div>
    </Panel>
  );
}
