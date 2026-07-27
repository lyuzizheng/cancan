import { describe, expect, it } from "vitest";

import {
  accountTypeLabel,
  batchGroupReasonLabel,
  batchGroupStatusLabel,
  eventTypeLabel,
  formatCurrencyAmount,
  formatLedgerDate,
  formatLedgerMonth,
  formatNativeAmount,
  localInboxScanSummaryText,
  localIsoToday,
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
    expect(batchGroupReasonLabel("relationship_not_selected")).toBe(
      "select both linked records before adding",
    );
    expect(batchGroupReasonLabel("core_preflight_failed")).toBe(
      "its details aren’t complete",
    );
    expect(batchGroupReasonLabel(null)).toBe("check its details");
  });
});

describe("formatLedgerMonth", () => {
  it("renders ISO dates and months as a full month name", () => {
    expect(formatLedgerMonth("2026-06-30")).toBe("June 2026");
    expect(formatLedgerMonth("2026-06")).toBe("June 2026");
    expect(formatLedgerMonth("2026-01-01")).toBe("January 2026");
  });

  it("passes through values that are not ISO months", () => {
    expect(formatLedgerMonth("2026-13-01")).toBe("2026-13-01");
    expect(formatLedgerMonth("last month")).toBe("last month");
  });
});

describe("localIsoToday", () => {
  it("pads month and day without timezone conversion", () => {
    expect(localIsoToday(new Date(2026, 6, 19))).toBe("2026-07-19");
    expect(localIsoToday(new Date(2026, 0, 5))).toBe("2026-01-05");
  });
});

describe("accountTypeLabel", () => {
  it("maps known account types and capitalizes unknown ones", () => {
    expect(accountTypeLabel("deposit_account")).toBe("Bank account");
    expect(accountTypeLabel("credit_card")).toBe("Credit card");
    expect(accountTypeLabel("manual_liability")).toBe("Liability");
    expect(accountTypeLabel("money_market")).toBe("Money market");
    expect(accountTypeLabel("")).toBe("Account");
  });
});

describe("localInboxScanSummaryText", () => {
  it("summarizes a scan in plain language", () => {
    expect(
      localInboxScanSummaryText({
        alreadyPresent: 12,
        deferred: 1,
        imported: 3,
        suppressed: 2,
      }),
    ).toBe("3 added; 12 already in CanCan; 1 to try again later; 2 kept deleted.");
  });

  it("says nothing new when every bucket is zero", () => {
    expect(
      localInboxScanSummaryText({
        alreadyPresent: 0,
        deferred: 0,
        imported: 0,
        suppressed: 0,
      }),
    ).toBe("Nothing new to add.");
  });

  it("omits empty buckets", () => {
    expect(
      localInboxScanSummaryText({
        alreadyPresent: 0,
        deferred: 0,
        imported: 1,
        suppressed: 0,
      }),
    ).toBe("1 added.");
  });
});
