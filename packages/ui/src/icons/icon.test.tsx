import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Icon } from "./icon";

describe("Icon", () => {
  it("renders every glyph on the 24px grid with the stroke discipline", () => {
    const names = [
      "overview",
      "sources",
      "assets",
      "transactions",
      "review",
      "money-flow",
      "jobs",
      "settings",
      "assistant",
      "check",
      "chevron-down",
      "chevron-right",
      "x",
      "lock",
      "refresh",
      "file",
      "alert",
      "eye",
    ] as const;

    for (const name of names) {
      const markup = renderToStaticMarkup(<Icon name={name} />);
      expect(markup).toContain('viewBox="0 0 24 24"');
      expect(markup).toContain('stroke-width="1.5"');
      expect(markup).toContain('stroke-linecap="square"');
      expect(markup).toContain('aria-hidden="true"');
    }
  });

  it("binds sizes to 16/18px tokens", () => {
    expect(renderToStaticMarkup(<Icon name="check" size={16} />)).toContain(
      'width="16"',
    );
    expect(renderToStaticMarkup(<Icon name="check" size={18} />)).toContain(
      'width="18"',
    );
  });
});
