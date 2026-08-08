import type { HTMLAttributes } from "react";

import { cx } from "../cx";

export interface ActionBarProps extends HTMLAttributes<HTMLDivElement> {
  align?: "start" | "end";
}

/** Row of related actions — one gap rhythm, wrap-safe, optionally right-docked. */
export function ActionBar({ align = "start", className, ...rest }: ActionBarProps) {
  return (
    <div
      className={cx(
        "flex flex-wrap items-center gap-2",
        align === "end" && "justify-end",
        className,
      )}
      {...rest}
    />
  );
}
