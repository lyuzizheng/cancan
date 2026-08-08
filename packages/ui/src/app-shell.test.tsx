import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AppShell, LedgerColumn, LedgerRegion } from "./app-shell";

describe("AppShell", () => {
  it("renders application content inside the shared shell", () => {
    const markup = renderToStaticMarkup(
      <AppShell>
        <p>Vault content</p>
      </AppShell>,
    );

    expect(markup).toContain("flex-col");
    expect(markup).toContain("md:flex-row");
    expect(markup).toContain("bg-ledger-mineral");
    expect(markup).toContain("Vault content");
  });
});

describe("LedgerRegion", () => {
  it("renders the flexible Ledger column with the gutter token", () => {
    const markup = renderToStaticMarkup(
      <LedgerRegion aria-busy="true">
        <p>Ledger content</p>
      </LedgerRegion>,
    );

    expect(markup).toContain("<section");
    expect(markup).toContain("p-ledger-gutter");
    expect(markup).toContain('aria-busy="true"');
    expect(markup).toContain("Ledger content");
  });
});

describe("LedgerColumn", () => {
  it("caps content at the Ledger measure token", () => {
    const markup = renderToStaticMarkup(
      <LedgerColumn>
        <p>Measured content</p>
      </LedgerColumn>,
    );

    expect(markup).toContain("max-w-ledger-measure");
    expect(markup).toContain("Measured content");
  });
});
