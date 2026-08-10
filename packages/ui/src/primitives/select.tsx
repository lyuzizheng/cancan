import * as SelectPrimitive from "@radix-ui/react-select";
import { forwardRef } from "react";

import { cx } from "../cx";
import { Icon } from "../icons/icon";

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectProps {
  value?: string;
  onValueChange?: (value: string) => void;
  options: SelectOption[];
  placeholder?: string;
  ariaLabel?: string;
  disabled?: boolean;
  size?: "sm" | "md";
}

/**
 * CanCan select — compact options control over Radix Select (full keyboard
 * and screen-reader behavior), styled with Ledger tokens only.
 */
export const Select = forwardRef<HTMLButtonElement, SelectProps>(function Select(
  { value, onValueChange, options, placeholder, ariaLabel, disabled, size = "md" },
  ref,
) {
  return (
    <SelectPrimitive.Root
      disabled={disabled}
      onValueChange={onValueChange}
      value={value}
    >
      <SelectPrimitive.Trigger
        aria-label={ariaLabel}
        className={cx(
          "inline-flex w-full items-center justify-between gap-2 rounded-sm border border-ledger-rule bg-ledger-porcelain px-2.5 text-sm text-ledger-ink transition duration-120 ease-mech",
          "hover:bg-ledger-mineral focus-visible:outline-2 focus-visible:outline-accent-go-deep focus-visible:outline-offset-1",
          "disabled:cursor-not-allowed disabled:opacity-60 data-[placeholder]:text-ledger-text-muted",
          size === "md" ? "h-8" : "h-7",
        )}
        ref={ref}
      >
        <SelectPrimitive.Value placeholder={placeholder} />
        <SelectPrimitive.Icon>
          <Icon className="text-ledger-text-muted" name="chevron-down" size={16} />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Content
          className="z-50 overflow-hidden rounded-md border border-ledger-rule bg-ledger-porcelain p-1 data-[state=open]:animate-pop-in data-[state=closed]:animate-pop-out motion-reduce:animate-none"
          position="popper"
          sideOffset={4}
        >
          <SelectPrimitive.Viewport>
            {options.map((option) => (
              <SelectPrimitive.Item
                className={cx(
                  "flex h-7 cursor-pointer items-center justify-between gap-2 rounded-xs px-2 text-sm text-ledger-ink outline-none transition-colors duration-120 ease-mech",
                  "data-[highlighted]:bg-ledger-mineral data-[state=checked]:font-medium",
                )}
                key={option.value}
                value={option.value}
              >
                <SelectPrimitive.ItemText>{option.label}</SelectPrimitive.ItemText>
                <SelectPrimitive.ItemIndicator>
                  <Icon name="check" size={16} />
                </SelectPrimitive.ItemIndicator>
              </SelectPrimitive.Item>
            ))}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  );
});
