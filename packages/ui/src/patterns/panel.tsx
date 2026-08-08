import type { HTMLAttributes } from "react";

import { cx } from "../cx";

/**
 * Ledger panel — porcelain surface bounded by a hairline rule, radius.md.
 * Borders and tonal separation before shadows (spec 0011 elevation).
 */
export function Panel({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cx(
        "rounded-md border border-ledger-rule bg-ledger-porcelain",
        className,
      )}
      {...rest}
    />
  );
}
