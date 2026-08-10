import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import type { ReactNode } from "react";

export interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
  side?: "top" | "right" | "bottom" | "left";
}

/**
 * CanCan tooltip — Vault-obsidian chip, 180ms pop-in / 140ms pop-out pair,
 * 150ms hover-intent delay (JS-side, outside the motion rhythm table), never
 * used for information the user cannot reach another way.
 */
export function Tooltip({ content, children, side = "top" }: TooltipProps) {
  return (
    <TooltipPrimitive.Provider delayDuration={150}>
      <TooltipPrimitive.Root>
        <TooltipPrimitive.Trigger asChild>{children}</TooltipPrimitive.Trigger>
        <TooltipPrimitive.Portal>
          <TooltipPrimitive.Content
            className="z-50 max-w-64 rounded-xs bg-vault-obsidian px-2 py-1 text-xs text-vault-text data-[state=delayed-open]:animate-pop-in data-[state=closed]:animate-pop-out motion-reduce:animate-none"
            side={side}
            sideOffset={4}
          >
            {content}
          </TooltipPrimitive.Content>
        </TooltipPrimitive.Portal>
      </TooltipPrimitive.Root>
    </TooltipPrimitive.Provider>
  );
}
