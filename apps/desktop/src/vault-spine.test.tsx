import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { VaultSpine } from "./vault-spine";

describe("VaultSpine", () => {
  it("announces each nav count with its own phrase; digits stay presentation-only", () => {
    const markup = renderToStaticMarkup(
      <VaultSpine
        activeView="tasks"
        inert={false}
        onNavigate={() => undefined}
        reviewCount={3}
        tasksCount={2}
        vaultStatus="unlocked"
      />,
    );

    expect(markup).toContain("2 need action");
    expect(markup).toContain("3 to review");
    expect(markup).not.toContain("2 to review");
    expect(markup).toContain('aria-hidden="true">2</span>');
    expect(markup).toContain('aria-hidden="true">3</span>');
  });

  it("uses the singular verb for a single actionable task", () => {
    const markup = renderToStaticMarkup(
      <VaultSpine
        activeView="overview"
        inert={false}
        onNavigate={() => undefined}
        reviewCount={null}
        tasksCount={1}
        vaultStatus="unlocked"
      />,
    );

    expect(markup).toContain("1 needs action");
  });
});
