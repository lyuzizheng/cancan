import { Button, StatusPoint } from "@cancan/ui";

export interface Notice {
  body: string;
  tone: "success" | "attention";
  title: string;
}

const pointTones = {
  attention: "attention",
  error: "risk",
  success: "healthy",
} as const;

export function Feedback({
  action,
  body,
  title,
  tone,
}: Omit<Notice, "tone"> & { action?: () => void; tone: Notice["tone"] | "error" }) {
  return (
    <section
      aria-live="polite"
      className="flex items-start justify-between gap-4 rounded-md border border-ledger-rule bg-ledger-porcelain p-4"
    >
      <div className="flex min-w-0 items-start gap-2.5">
        <StatusPoint className="mt-1.5 shrink-0" tone={pointTones[tone]} />
        <div className="min-w-0">
          <p className="text-sm font-medium text-ledger-ink">{title}</p>
          <p className="mt-0.5 text-sm text-ledger-text-muted">{body}</p>
        </div>
      </div>
      {action ? (
        <Button onClick={action} size="sm" variant="quiet">
          Try again
        </Button>
      ) : null}
    </section>
  );
}
