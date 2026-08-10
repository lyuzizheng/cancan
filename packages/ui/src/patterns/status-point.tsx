import type { HTMLAttributes } from "react";

import { cx } from "../cx";

export type StatusPointTone = "healthy" | "attention" | "risk" | "idle";

const tones: Record<StatusPointTone, string> = {
  healthy: "bg-signal-emerald",
  attention: "bg-signal-amber",
  risk: "bg-signal-cinnabar",
  idle: "bg-ledger-rule",
};

export interface StatusPointProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: StatusPointTone;
}

/**
 * Optical status point — always paired with a text label; status never
 * relies on color alone (spec 0011).
 */
export function StatusPoint({ tone = "idle", className, ...rest }: StatusPointProps) {
  return (
    <span
      aria-hidden="true"
      className={cx("inline-block size-1.5 rounded-pill transition-colors duration-120 ease-mech", tones[tone], className)}
      {...rest}
    />
  );
}
