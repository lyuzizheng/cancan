import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Badge } from "./badge";
import { Button } from "./button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
  DialogTrigger,
} from "./dialog";
import { Input } from "./input";
import { Select } from "./select";
import { Skeleton } from "./skeleton";
import { Tooltip } from "./tooltip";

describe("Button", () => {
  it("applies the variant, size, and motion discipline", () => {
    const markup = renderToStaticMarkup(
      <Button size="sm" variant="strong">
        Confirm
      </Button>,
    );

    expect(markup).toContain("bg-accent-go-bright");
    expect(markup).toContain("text-accent-go-ink");
    expect(markup).toContain("h-7");
    expect(markup).toContain("rounded-sm");
    expect(markup).toContain("duration-120");
    expect(markup).toContain("ease-mech");
    expect(markup).toContain("Confirm");
  });

  it("defaults to the neutral primary action", () => {
    const markup = renderToStaticMarkup(<Button>Save</Button>);

    expect(markup).toContain("bg-vault-obsidian");
    expect(markup).toContain("h-8");
  });
});

describe("Input", () => {
  it("renders Ledger chrome and opt-in tabular numerals", () => {
    const plain = renderToStaticMarkup(<Input aria-label="Note" />);
    const numeric = renderToStaticMarkup(<Input aria-label="Amount" numeric />);

    expect(plain).toContain("border-ledger-rule");
    expect(plain).toContain("h-8");
    expect(plain).not.toContain("tabular-nums");
    expect(numeric).toContain("tabular-nums");
  });
});

describe("Select", () => {
  it("renders the CanCan trigger over the Radix root", () => {
    const markup = renderToStaticMarkup(
      <Select
        ariaLabel="Account decision"
        options={[
          { value: "accept", label: "Accept" },
          { value: "dismiss", label: "Dismiss" },
        ]}
        placeholder="Choose"
      />,
    );

    expect(markup).toContain("Choose");
    expect(markup).toContain('aria-label="Account decision"');
    expect(markup).toContain("border-ledger-rule");
  });
});

describe("Dialog", () => {
  it("renders the trigger; content stays portaled until opened", () => {
    const markup = renderToStaticMarkup(
      <Dialog>
        <DialogTrigger asChild>
          <Button>Open</Button>
        </DialogTrigger>
        <DialogContent>
          <DialogTitle>Evidence</DialogTitle>
          <DialogDescription>Bounded preview</DialogDescription>
        </DialogContent>
      </Dialog>,
    );

    expect(markup).toContain("Open");
    expect(markup).not.toContain("Bounded preview");
  });
});

describe("Tooltip", () => {
  it("renders the trigger without eager content", () => {
    const markup = renderToStaticMarkup(
      <Tooltip content="Encrypted on this Mac">
        <Button variant="text">Vault</Button>
      </Tooltip>,
    );

    expect(markup).toContain("Vault");
    expect(markup).not.toContain("Encrypted on this Mac");
  });
});

describe("Skeleton", () => {
  it("uses the bounded pulse with a reduced-motion escape", () => {
    const markup = renderToStaticMarkup(<Skeleton className="h-3 w-24" />);

    expect(markup).toContain("motion-safe:animate-pulse");
    expect(markup).toContain('aria-hidden="true"');
  });
});

describe("Badge", () => {
  it("renders counts in the mono pill discipline", () => {
    const markup = renderToStaticMarkup(<Badge tone="attention">3</Badge>);

    expect(markup).toContain("rounded-pill");
    expect(markup).toContain("font-mono");
    expect(markup).toContain("text-signal-amber-text");
    expect(markup).toContain(">3</span>");
  });
});
