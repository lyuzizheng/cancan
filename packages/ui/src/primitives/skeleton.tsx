import type { HTMLAttributes } from "react";

import { cx } from "../cx";

/**
 * Loading placeholder block. The bounded low-frequency pulse (two iterations
 * on ease.mech, then a static block) is the one processing-motion exception
 * granted by spec 0011; reduced motion swaps it for a static block.
 */
export function Skeleton({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      aria-hidden="true"
      className={cx(
        "rounded-xs bg-ledger-mineral motion-safe:animate-pulse-bounded",
        className,
      )}
      {...rest}
    />
  );
}
