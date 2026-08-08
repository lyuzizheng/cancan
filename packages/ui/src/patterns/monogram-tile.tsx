import type { HTMLAttributes } from "react";

import { cx } from "../cx";

/**
 * Two-letter monogram for a source/prompt display name: first letters of the
 * first two words, or of a single word. (Promoted from the desktop format
 * helpers so every surface shares one monogram discipline.)
 */
export function monogramInitials(displayName: string): string {
  const words = displayName.trim().split(/\s+/).filter((word) => word.length > 0);
  if (words.length === 0) {
    return "··";
  }
  const first = words[0]!;
  if (words.length === 1) {
    return first.slice(0, 2).toUpperCase();
  }
  return `${first[0]!}${words[1]![0]!}`.toUpperCase();
}

export interface MonogramTileProps extends HTMLAttributes<HTMLSpanElement> {
  name: string;
  size?: 28 | 32;
}

/** Source monogram tile — Martian Mono initials on a ruled mineral tile. */
export function MonogramTile({
  name,
  size = 28,
  className,
  ...rest
}: MonogramTileProps) {
  return (
    <span
      aria-hidden="true"
      className={cx(
        "inline-flex items-center justify-center rounded-sm border border-ledger-rule bg-ledger-mineral font-mono text-xs text-ledger-text-muted",
        size === 28 ? "size-7" : "size-8",
        className,
      )}
      {...rest}
    >
      {monogramInitials(name)}
    </span>
  );
}
