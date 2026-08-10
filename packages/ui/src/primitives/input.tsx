import { forwardRef, type InputHTMLAttributes } from "react";

import { cx } from "../cx";

/**
 * CanCan input — 32px compact control, radius.sm, Ledger chrome.
 * `numeric` enables tabular figures for amount/number entry.
 */
export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  numeric?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { numeric = false, className, ...rest },
  ref,
) {
  return (
    <input
      className={cx(
        "h-8 w-full rounded-sm border border-ledger-rule bg-ledger-porcelain px-2.5 text-sm text-ledger-ink transition duration-120 ease-mech placeholder:text-ledger-text-muted",
        "focus-visible:outline-2 focus-visible:outline-accent-go-deep focus-visible:outline-offset-1",
        "disabled:cursor-not-allowed disabled:opacity-60",
        numeric && "tabular-nums",
        className,
      )}
      ref={ref}
      {...rest}
    />
  );
});
