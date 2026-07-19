import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AppShell } from "./app-shell";

describe("AppShell", () => {
  it("renders application content inside the shared shell", () => {
    const markup = renderToStaticMarkup(
      <AppShell>
        <p>Vault content</p>
      </AppShell>,
    );

    expect(markup).toContain('class="vault-app"');
    expect(markup).toContain("Vault content");
  });
});
