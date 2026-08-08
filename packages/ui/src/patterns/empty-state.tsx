import type { ReactNode } from "react";

import { Icon, type IconName } from "../icons/icon";

export interface EmptyStateProps {
  title: string;
  body?: string;
  icon?: IconName;
  action?: ReactNode;
}

/**
 * Empty state — a scoped Fraunces display moment (spec 0008 typography):
 * serif title at 560 weight inside the fixed scale, quiet body, deliberate
 * whitespace. Never used for UI chrome or labels.
 */
export function EmptyState({ title, body, icon = "file", action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center py-16 text-center">
      <span className="inline-flex size-10 items-center justify-center rounded-md border border-ledger-rule bg-ledger-porcelain text-ledger-text-muted">
        <Icon name={icon} size={18} />
      </span>
      <h3 className="mt-4 font-serif text-xl font-display text-ledger-ink">{title}</h3>
      {body !== undefined && (
        <p className="mt-2 max-w-sm text-sm text-ledger-text-muted">{body}</p>
      )}
      {action !== undefined && <div className="mt-5">{action}</div>}
    </div>
  );
}
