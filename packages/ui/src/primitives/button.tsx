import type { ButtonHTMLAttributes, ReactNode } from "react";

import { cx } from "../cx";

/**
 * CanCan button — the single action control.
 * Geometry: 32/28px heights, radius.sm, 120ms ease-mech color feedback.
 * `strong` is the editorial confirmation moment (go fill + go-ink text,
 * a verified 9.10:1 pair); green stays a deliberate choice, never the
 * default for every action (spec 0011 semantic usage).
 */
export type ButtonVariant = "primary" | "strong" | "quiet" | "text" | "danger";

const variants: Record<ButtonVariant, string> = {
  primary:
    "bg-vault-obsidian text-ledger-porcelain hover:bg-vault-graphite",
  strong: "bg-accent-go-bright text-accent-go-ink hover:bg-accent-go",
  quiet:
    "border border-ledger-rule bg-ledger-porcelain text-ledger-ink hover:bg-ledger-mineral",
  text: "bg-transparent text-ledger-ink hover:bg-ledger-mineral hover:text-accent-go-deep",
  danger:
    "border border-ledger-rule bg-ledger-porcelain text-signal-danger-text hover:border-signal-cinnabar",
};

const sizes = {
  md: "h-8 px-3.5 text-sm",
  sm: "h-7 px-2.5 text-sm",
} as const;

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: keyof typeof sizes;
  icon?: ReactNode;
}

export function Button({
  variant = "primary",
  size = "md",
  icon,
  className,
  children,
  type = "button",
  ...rest
}: ButtonProps) {
  return (
    <button
      className={cx(
        "inline-flex select-none items-center justify-center gap-1.5 rounded-sm font-medium transition-colors duration-120 ease-mech",
        "focus-visible:outline-2 focus-visible:outline-accent-go-deep focus-visible:outline-offset-2",
        "disabled:cursor-progress disabled:opacity-60",
        variants[variant],
        sizes[size],
        className,
      )}
      type={type}
      {...rest}
    >
      {icon}
      {children}
    </button>
  );
}
