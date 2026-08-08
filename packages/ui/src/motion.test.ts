import { describe, expect, it } from "vitest";

import { easeMech, motion, prefersReducedMotion, transitionFor } from "./motion";

describe("motion tokens", () => {
  it("exposes the 120/180/240/320 rhythm and the single ease-mech curve", () => {
    expect(motion).toEqual({ feedback: 120, state: 180, transition: 240, slow: 320 });
    expect(easeMech).toBe("cubic-bezier(0.19, 1, 0.22, 1)");
  });

  it("builds transition shorthands on the ease-mech curve", () => {
    expect(transitionFor(["opacity"], motion.feedback)).toBe(
      "opacity 120ms cubic-bezier(0.19, 1, 0.22, 1)",
    );
    expect(transitionFor(["color", "background-color"])).toBe(
      "color 180ms cubic-bezier(0.19, 1, 0.22, 1), background-color 180ms cubic-bezier(0.19, 1, 0.22, 1)",
    );
  });

  it("reports no reduced-motion preference outside a browser", () => {
    expect(prefersReducedMotion()).toBe(false);
  });
});
