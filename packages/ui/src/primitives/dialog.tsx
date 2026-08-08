import * as DialogPrimitive from "@radix-ui/react-dialog";
import type { ComponentProps, ReactNode } from "react";

import { cx } from "../cx";
import { Icon } from "../icons/icon";
import { Button } from "./button";

/**
 * CanCan modal — Radix supplies focus trap, scroll lock, and Escape/outside
 * dismissal; CanCan tokens supply identity. Porcelain panel at radius.md on a
 * Vault-obsidian scrim; motion follows the 180/240ms ease-mech rhythm with a
 * reduced-motion fallback.
 */
export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogClose = DialogPrimitive.Close;

export function DialogContent({
  className,
  children,
  ...rest
}: ComponentProps<typeof DialogPrimitive.Content>) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay
        className={cx(
          "fixed inset-0 z-40 bg-vault-obsidian/60",
          "data-[state=open]:animate-overlay-in data-[state=closed]:animate-overlay-out motion-reduce:animate-none",
        )}
      />
      <DialogPrimitive.Content
        className={cx(
          "fixed left-1/2 top-1/2 z-50 w-11/12 max-w-md -translate-x-1/2 -translate-y-1/2 rounded-md border border-ledger-rule bg-ledger-porcelain p-6",
          "focus:outline-none",
          "data-[state=open]:animate-dialog-in data-[state=closed]:animate-dialog-out motion-reduce:animate-none",
          className,
        )}
        {...rest}
      >
        {children}
        <DialogPrimitive.Close asChild>
          <Button
            aria-label="Close"
            className="absolute right-3 top-3 px-1.5"
            icon={<Icon name="x" size={16} />}
            size="sm"
            variant="text"
          />
        </DialogPrimitive.Close>
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}

export function DialogTitle({
  className,
  ...rest
}: ComponentProps<typeof DialogPrimitive.Title>) {
  return (
    <DialogPrimitive.Title
      className={cx("text-lg font-medium text-ledger-ink", className)}
      {...rest}
    />
  );
}

export function DialogDescription({
  className,
  ...rest
}: ComponentProps<typeof DialogPrimitive.Description>) {
  return (
    <DialogPrimitive.Description
      className={cx("mt-2 text-sm text-ledger-text-muted", className)}
      {...rest}
    />
  );
}

export function DialogActions({ children }: { children: ReactNode }) {
  return <div className="mt-6 flex flex-wrap items-center gap-2">{children}</div>;
}
