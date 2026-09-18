import type { ReactNode } from "react";

import { cx } from "../cx";

/**
 * CanCan icon discipline: one 24px grid, 1.5px stroke, square caps, miter
 * joins, 16/18px sizes. Glyphs are drawn for CanCan (systematized from the
 * original Vault Spine set); no ad-hoc per-view icons and no icon library
 * dependency. Add a glyph only when a second surface needs it.
 */
export type IconName =
  | "overview"
  | "sources"
  | "assets"
  | "transactions"
  | "review"
  | "money-flow"
  | "jobs"
  | "settings"
  | "assistant"
  | "check"
  | "chevron-down"
  | "chevron-left"
  | "chevron-right"
  | "x"
  | "lock"
  | "file";

const shapes: Record<IconName, ReactNode> = {
  overview: (
    <>
      <rect x="4" y="4" width="6" height="6" rx="1" />
      <rect x="14" y="4" width="6" height="6" rx="1" />
      <rect x="4" y="14" width="6" height="6" rx="1" />
      <rect x="14" y="14" width="6" height="6" rx="1" />
    </>
  ),
  sources: (
    <>
      <rect x="4" y="4" width="16" height="16" rx="2" />
      <path d="M4 10h16" />
    </>
  ),
  assets: (
    <>
      <circle cx="12" cy="12" r="7" />
      <path d="M12 8v4l3 1.8" />
    </>
  ),
  transactions: (
    <>
      <path d="M4 7h12" />
      <path d="M13 4l3 3-3 3" />
      <path d="M20 17H8" />
      <path d="M11 14l-3 3 3 3" />
    </>
  ),
  review: (
    <>
      <rect x="4" y="4" width="16" height="16" rx="2" />
      <path d="M8 12.5l2.8 2.8 4.7-5.8" />
    </>
  ),
  "money-flow": <path d="M4 17l4.5-4.5 3.5 3.5L19 9" />,
  jobs: <path d="M6 6h12M6 11h12M6 16h7" />,
  settings: (
    <>
      <circle cx="12" cy="12" r="2.6" />
      <path d="M12 4v2.4M12 17.6V20M4 12h2.4M17.6 12H20M6.3 6.3l1.7 1.7M16 16l1.7 1.7M17.7 6.3L16 8M8 16l-1.7 1.7" />
    </>
  ),
  assistant: (
    <path d="M5 4.5h14a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H10l-5 3.75v-15A1.5 1.5 0 0 1 5 4.5z" />
  ),
  check: <path d="M5 12.5l4.5 4.5L19 7.5" />,
  "chevron-down": <path d="M7 10l5 5 5-5" />,
  "chevron-left": <path d="M14 7l-5 5 5 5" />,
  "chevron-right": <path d="M10 7l5 5-5 5" />,
  x: <path d="M7 7l10 10M17 7L7 17" />,
  lock: (
    <>
      <rect x="6" y="11" width="12" height="9" rx="1.5" />
      <path d="M9 11V8a3 3 0 0 1 6 0v3" />
    </>
  ),
  file: (
    <>
      <path d="M7 3.5h7l4 4V20a.5.5 0 0 1-.5.5h-11A.5.5 0 0 1 6 20V4a.5.5 0 0 1 .5-.5z" />
      <path d="M14 3.5V8h4.5" />
    </>
  ),
};

export interface IconProps {
  name: IconName;
  /** Token-bound sizes only: 16 or 18 px. */
  size?: 16 | 18;
  className?: string;
}

export function Icon({ name, size = 18, className }: IconProps) {
  return (
    <svg
      aria-hidden="true"
      className={cx("shrink-0", className)}
      fill="none"
      height={size}
      stroke="currentColor"
      strokeLinecap="square"
      strokeLinejoin="miter"
      strokeWidth="1.5"
      viewBox="0 0 24 24"
      width={size}
    >
      {shapes[name]}
    </svg>
  );
}
