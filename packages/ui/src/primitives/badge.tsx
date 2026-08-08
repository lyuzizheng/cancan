import type { HTMLAttributes, ReactNode } from "react";

import { cx } from "../cx";

/**
 * Count/label chip — radius.pill is reserved for this true pill control.
 * Counts render in Martian Mono (machine metadata role).
 */
export type BadgeTone = "neutral" | "attention" | "healthy";

const tones: Record<BadgeTone, string> = {
  neutral: "border-ledger-rule text-ledger-text-muted",
  attention: "border-signal-amber text-signal-amber-text",
  healthy: "border-signal-emerald text-signal-emerald",
};

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: BadgeTone;
  children: ReactNode;
}

export function Badge({ tone = "neutral", className, children, ...rest }: BadgeProps) {
  return (
    <span
      className={cx(
        "inline-flex h-5 min-w-5 items-center justify-center rounded-pill border bg-ledger-porcelain px-1.5 font-mono text-xs",
        tones[tone],
        className,
      )}
      {...rest}
    >
      {children}
    </span>
  );
}
