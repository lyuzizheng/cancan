import type { HTMLAttributes, ReactNode } from "react";

import { cx } from "../cx";

export interface MetricRowProps extends HTMLAttributes<HTMLDivElement> {
  label: ReactNode;
  value: ReactNode;
  /** Secondary line under the label (freshness, period, identifier). */
  meta?: ReactNode;
  /** Renders the bottom hairline; omit on the last row of a group. */
  ruled?: boolean;
}

/**
 * Ledger metric row — open ledger rhythm: label/meta on the left, tabular
 * value right-aligned, separated by hairline rules instead of cards.
 */
export function MetricRow({
  label,
  value,
  meta,
  ruled = true,
  className,
  ...rest
}: MetricRowProps) {
  return (
    <div
      className={cx(
        "flex items-baseline justify-between gap-4 py-3",
        ruled && "border-b border-ledger-rule",
        className,
      )}
      {...rest}
    >
      <div className="min-w-0">
        <div className="truncate text-md font-medium text-ledger-ink">{label}</div>
        {meta !== undefined && (
          <div className="mt-0.5 truncate font-mono text-xs text-ledger-text-muted">
            {meta}
          </div>
        )}
      </div>
      <div className="shrink-0 text-right text-md font-medium tabular-nums text-ledger-ink">
        {value}
      </div>
    </div>
  );
}
