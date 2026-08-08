import type { ReactNode } from "react";

export interface LedgerHeaderProps {
  /** Mono metadata kicker above the title (e.g. "Command Center"). */
  eyebrow: string;
  /** Page moment — the one Fraunces display role per view (spec 0008). */
  title: ReactNode;
  /** Right-aligned view actions (quiet/primary Buttons). */
  actions?: ReactNode;
}

/**
 * Ledger page header — eyebrow + serif display title + actions over a
 * hairline rule. The page title is the spec-authorized Fraunces moment
 * (≤text-2xl, display weight); every other label stays in Geologica.
 */
export function LedgerHeader({ eyebrow, title, actions }: LedgerHeaderProps) {
  return (
    <header className="flex flex-wrap items-start justify-between gap-x-6 gap-y-3 border-b border-ledger-rule pb-5">
      <div>
        <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
          {eyebrow}
        </p>
        <h1 className="mt-1.5 font-serif text-2xl font-display text-ledger-ink">
          {title}
        </h1>
      </div>
      {actions !== undefined ? (
        <div className="flex flex-wrap items-center gap-2.5">{actions}</div>
      ) : null}
    </header>
  );
}
