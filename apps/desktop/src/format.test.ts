import { describe, expect, it } from "vitest";

import {
  batchGroupReasonLabel,
  batchGroupStatusLabel,
  eventTypeLabel,
  formatCurrencyAmount,
  formatLedgerDate,
  formatNativeAmount,
  reviewConflictMessage,
  reviewReasonLabel,
} from "./format";

describe("formatNativeAmount", () => {
  it("groups thousands without float conversion and preserves scale", () => {
    expect(formatNativeAmount("1234567.890")).toBe("1,234,567.890");
    expect(formatNativeAmount("1234.50")).toBe("1,234.50");
    expect(formatNativeAmount("0")).toBe("0");
    expect(formatNativeAmount("42")).toBe("42");
    expect(formatNativeAmount("999")).toBe("999");
    expect(formatNativeAmount("1000")).toBe("1,000");
  });

  it("keeps the sign on negative values", () => {
    expect(formatNativeAmount("-1234567.89")).toBe("-1,234,567.89");
    expect(formatNativeAmount("-0.05")).toBe("-0.05");
  });

  it("passes through values outside the exact-decimal shape", () => {
    expect(formatNativeAmount("1,234")).toBe("1,234");
    expect(formatNativeAmount("abc")).toBe("abc");
    expect(formatNativeAmount("")).toBe("");
    expect(formatNativeAmount("12.")).toBe("12.");
  });
});

describe("formatCurrencyAmount", () => {
  it("prefixes the currency code", () => {
    expect(formatCurrencyAmount("SGD", "1234.5")).toBe("SGD 1,234.5");
  });

  it("handles missing currency and amount", () => {
    expect(formatCurrencyAmount(null, "10")).toBe("10");
    expect(formatCurrencyAmount("SGD", null)).toBe("Amount missing");
  });
});

describe("formatLedgerDate", () => {
  it("renders ISO calendar dates without timezone shifts", () => {
    expect(formatLedgerDate("2026-07-19")).toBe("19 Jul 2026");
    expect(formatLedgerDate("2026-01-01")).toBe("1 Jan 2026");
  });

  it("passes through unexpected values", () => {
    expect(formatLedgerDate("2026-13-40")).toBe("2026-13-40");
    expect(formatLedgerDate("yesterday")).toBe("yesterday");
  });
});

describe("eventTypeLabel", () => {
  it("maps canonical event types to personal-finance language", () => {
    expect(eventTypeLabel("same_currency_transfer")).toBe("Transfer");
    expect(eventTypeLabel("credit_card_repayment")).toBe("Card repayment");
    expect(eventTypeLabel("same_currency_transfer_reversal")).toBe(
      "Transfer undone",
    );
    expect(eventTypeLabel("credit_card_repayment_reversal")).toBe(
      "Repayment undone",
    );
    expect(eventTypeLabel("purchase")).toBe("Purchase");
    expect(eventTypeLabel(null)).toBe("Record");
  });

  it("humanizes unknown event types", () => {
    expect(eventTypeLabel("balance_observation")).toBe("Balance update");
    expect(eventTypeLabel("fx_conversion")).toBe("Fx conversion");
  });
});

describe("reviewReasonLabel", () => {
  it("maps known reason codes and falls back safely", () => {
    expect(reviewReasonLabel("possible_card_repayment")).toBe(
      "Looks like a card repayment",
    );
    expect(reviewReasonLabel("source_file_deleted")).toBe("Source file deleted");
    expect(reviewReasonLabel("record_edited")).toBe("Edited record");
    expect(reviewReasonLabel("auto_commit_disabled")).toBe(
      "Automatic add is off",
    );
    expect(reviewReasonLabel("some_future_reason")).toBe("Needs your check");
  });
});

describe("reviewConflictMessage", () => {
  it("explains stale and invalid outcomes without leaking internals", () => {
    expect(reviewConflictMessage("stale_review_item")).toContain("changed");
    expect(reviewConflictMessage("invalid_review_edit")).toContain(
      "amount and date",
    );
    expect(reviewConflictMessage("relationship_needs_review")).toBe(
      "CanCan can’t confirm that link yet.",
    );
    expect(reviewConflictMessage("unknown_reason")).toBe(
      "CanCan couldn’t apply that change.",
    );
    expect(reviewConflictMessage(null)).toBe(
      "CanCan couldn’t apply that change.",
    );
  });
});

describe("batch outcome labels", () => {
  it("maps group statuses to personal-finance language", () => {
    expect(batchGroupStatusLabel("committed")).toBe("Added");
    expect(batchGroupStatusLabel("already_committed")).toBe("Already added");
    expect(batchGroupStatusLabel("stale")).toBe("Changed while adding");
    expect(batchGroupStatusLabel("still_needs_review")).toBe(
      "Still needs your check",
    );
    expect(batchGroupStatusLabel("future_status")).toBe("Needs your check");
  });

  it("maps group reasons to plain explanations", () => {
    expect(batchGroupReasonLabel("relationship_not_confirmed")).toBe(
      "its link isn’t confirmed yet",
    );
    expect(batchGroupReasonLabel("ambiguous_relationship")).toBe(
      "it has more than one possible link",
    );
    expect(batchGroupReasonLabel("core_preflight_failed")).toBe(
      "its details aren’t complete",
    );
    expect(batchGroupReasonLabel(null)).toBe("check its details");
  });
});
