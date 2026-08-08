import type { HTMLAttributes } from "react";

import { cx } from "../cx";

/**
 * Loading placeholder block. The bounded low-frequency pulse is the one
 * perpetual-motion exception granted by spec 0011 (processing status);
 * reduced motion swaps it for a static block.
 */
export function Skeleton({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      aria-hidden="true"
      className={cx(
        "rounded-xs bg-ledger-mineral motion-safe:animate-pulse",
        className,
      )}
      {...rest}
    />
  );
}
