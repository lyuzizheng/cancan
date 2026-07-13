import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AppShell } from "./app-shell";

describe("AppShell", () => {
  it("renders the local-first development shell", () => {
    const markup = renderToStaticMarkup(<AppShell />);

    expect(markup).toContain("CanCan");
    expect(markup).toContain("Local-first foundation");
  });
});
