import { describe, expect, it } from "vitest";

import { normalizeDisplayAmount, twoDecimalMinorUnits } from "./amount-normalization";

describe("normalizeDisplayAmount", () => {
  it.each([
    ["0.00", "0.00"],
    ["20.00", "20.00"],
    ["1,234.56", "1234.56"],
    ["12,345,678.90", "12345678.90"],
    ["1,234", "1234"],
    ["-1,234.56", "-1234.56"],
    ["0", "0"],
  ])("normalizes %s to %s", (input, expected) => {
    expect(normalizeDisplayAmount(input)).toBe(expected);
  });

  it.each([
    "",
    "1,23.56",
    "12,34.56",
    "1,2345.56",
    ",123.56",
    "123,.56",
    "01,234.56",
    "1,234.5678.9",
    "+1,234.56",
    " 1,234.56",
    "1,234.56 ",
    "ABC",
    "1.234,56",
    "--1,234.56",
  ])("rejects malformed display amount %s", (input) => {
    expect(normalizeDisplayAmount(input)).toBeUndefined();
  });

  it("preserves value equality across display and canonical forms", () => {
    expect(normalizeDisplayAmount("1,234.56")).toBe(normalizeDisplayAmount("1234.56"));
    expect(normalizeDisplayAmount("-12,345,678.90")).toBe("-12345678.90");
    expect(normalizeDisplayAmount("0.00")).toBe("0.00");
  });
});

describe("twoDecimalMinorUnits with display grouping", () => {
  it.each([
    ["100.00", 10000n],
    ["1,234.56", 123456n],
    ["12,345,678.90", 1234567890n],
    ["-1,234.56", -123456n],
    ["0.00", 0n],
  ])("parses %s to %s minor units", (input, expected) => {
    expect(twoDecimalMinorUnits(input)).toBe(expected);
  });

  it.each(["20.0", "20", "1,234.567", "ABC", "1,23.56"])(
    "rejects non-two-decimal %s",
    (input) => {
      expect(twoDecimalMinorUnits(input)).toBeUndefined();
    },
  );
});
