import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Button } from "../primitives/button";
import { ActionBar } from "./action-bar";
import { EmptyState } from "./empty-state";
import { LedgerHeader } from "./ledger-header";
import { MetricRow } from "./metric-row";
import { MonogramTile, monogramInitials } from "./monogram-tile";
import { Panel } from "./panel";
import { SectionHeader } from "./section-header";
import { StatusPoint } from "./status-point";

describe("monogramInitials", () => {
  it("derives two-letter monograms from display names", () => {
    expect(monogramInitials("DBS Multiplier Account")).toBe("DM");
    expect(monogramInitials("Wise")).toBe("WI");
    expect(monogramInitials("   ")).toBe("··");
  });
});

describe("MonogramTile", () => {
  it("renders initials in the ruled tile", () => {
    const markup = renderToStaticMarkup(<MonogramTile name="DBS" />);

    expect(markup).toContain("DB");
    expect(markup).toContain("font-mono");
    expect(markup).toContain("border-ledger-rule");
  });
});

describe("Panel", () => {
  it("is a porcelain hairline surface", () => {
    const markup = renderToStaticMarkup(<Panel>content</Panel>);

    expect(markup).toContain("rounded-md");
    expect(markup).toContain("bg-ledger-porcelain");
  });
});

describe("SectionHeader", () => {
  it("renders status point, title, count, and action", () => {
    const markup = renderToStaticMarkup(
      <SectionHeader
        action={<Button variant="text">View all</Button>}
        count={3}
        title="Tasks"
        tone="attention"
      />,
    );

    expect(markup).toContain("Tasks");
    expect(markup).toContain(">3</span>");
    expect(markup).toContain("View all");
    expect(markup).toContain("ml-auto");
  });
});

describe("LedgerHeader", () => {
  it("renders the mono eyebrow, the scoped serif page title, and actions", () => {
    const markup = renderToStaticMarkup(
      <LedgerHeader
        actions={<Button variant="quiet">Refresh</Button>}
        eyebrow="Command Center"
        title="Your money, organized"
      />,
    );

    expect(markup).toContain("Command Center");
    expect(markup).toContain("tracking-mono-label");
    expect(markup).toContain("font-serif");
    expect(markup).toContain("font-display");
    expect(markup).toContain("Your money, organized");
    expect(markup).toContain("Refresh");
    expect(markup).toContain("border-ledger-rule");
  });
});

describe("StatusPoint", () => {
  it("maps tones to signal colors", () => {
    expect(renderToStaticMarkup(<StatusPoint tone="healthy" />)).toContain(
      "bg-signal-emerald",
    );
    expect(renderToStaticMarkup(<StatusPoint tone="attention" />)).toContain(
      "bg-signal-amber",
    );
  });
});

describe("EmptyState", () => {
  it("uses the scoped serif display role", () => {
    const markup = renderToStaticMarkup(
      <EmptyState body="Save a statement to begin." title="No evidence yet" />,
    );

    expect(markup).toContain("font-serif");
    expect(markup).toContain("font-display");
    expect(markup).toContain("No evidence yet");
    expect(markup).toContain("Save a statement to begin.");
  });
});

describe("MetricRow", () => {
  it("keeps the open-ledger rhythm with tabular values", () => {
    const markup = renderToStaticMarkup(
      <MetricRow label="DBS Multiplier" meta="As of 19 Jul 2026" value="SGD 12,456.78" />,
    );

    expect(markup).toContain("border-b");
    expect(markup).toContain("tabular-nums");
    expect(markup).toContain("SGD 12,456.78");
    expect(markup).toContain("font-mono");
  });
});

describe("ActionBar", () => {
  it("groups actions on one gap rhythm", () => {
    const markup = renderToStaticMarkup(
      <ActionBar align="end">
        <Button variant="quiet">Cancel</Button>
        <Button>Save</Button>
      </ActionBar>,
    );

    expect(markup).toContain("justify-end");
    expect(markup).toContain("gap-2");
  });
});
