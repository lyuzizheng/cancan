import type { HTMLAttributes, ReactNode } from "react";

import { cx } from "./cx";

export interface AppShellProps {
  children: ReactNode;
}

/**
 * Application chassis — the dark Vault Spine rail beside the light Ledger
 * reading region. Below md the spine stacks as a top bar and its nav becomes
 * a horizontal strip, so navigation is never lost on narrow widths.
 */
export function AppShell({ children }: AppShellProps) {
  return (
    <main className="flex min-h-screen flex-col bg-ledger-mineral font-sans text-base tabular-nums text-ledger-ink antialiased md:flex-row">
      {children}
    </main>
  );
}

/**
 * Ledger region — the flexible light column right of the spine. Breathing
 * room comes from the `ledger-gutter` token, never ad-hoc padding.
 */
export function LedgerRegion({ className, ...rest }: HTMLAttributes<HTMLElement>) {
  return <section className={cx("min-w-0 flex-1 p-ledger-gutter", className)} {...rest} />;
}

/**
 * Ledger measure — caps the reading column at the `ledger-measure` token so
 * content stays left-anchored with deliberate whitespace beyond it.
 */
export function LedgerColumn({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cx("w-full max-w-ledger-measure", className)} {...rest} />;
}
