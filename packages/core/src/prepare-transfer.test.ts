import { describe, expect, it } from "vitest";

import { prepareSameCurrencyTransfer } from "./prepare-transfer";

const qualifiedRecord = {
  sourceConfigured: true,
  providerVerified: true,
  profileQualified: true,
  schemaValid: true,
  rawGrounded: true,
  deterministicValidationPassed: true,
  accountResolved: true,
  duplicateFree: true,
  allocationComplete: true,
  veryHighConfidence: true,
  requiredLegsPresent: true,
} as const;

function qualifiedSnapshot(recordId: string, value: string) {
  return { recordId, value, eligibility: qualifiedRecord };
}

describe("prepareSameCurrencyTransfer", () => {
  it("prepares one canonical transfer when both source-account windows close exactly", () => {
    const result = prepareSameCurrencyTransfer({
      autoCommitEnabled: true,
      commitIdempotencyKey: "transfer:checking:savings:2026-07-01:250.00:SGD",
      records: [
        {
          id: "record-checking-out",
          accountId: "account-checking",
          instrumentId: "instrument-sgd",
          postedOn: "2026-07-01",
          currency: "SGD",
          accountBalanceDelta: "-250.00",
          eligibility: qualifiedRecord,
        },
        {
          id: "record-savings-in",
          accountId: "account-savings",
          instrumentId: "instrument-sgd",
          postedOn: "2026-07-01",
          currency: "SGD",
          accountBalanceDelta: "250.00",
          eligibility: qualifiedRecord,
        },
      ],
      reconciliationWindows: [
        {
          accountId: "account-checking",
          currency: "SGD",
          opening: qualifiedSnapshot("record-checking-opening", "1000.00"),
          closing: qualifiedSnapshot("record-checking-closing", "750.00"),
        },
        {
          accountId: "account-savings",
          currency: "SGD",
          opening: qualifiedSnapshot("record-savings-opening", "100.00"),
          closing: qualifiedSnapshot("record-savings-closing", "350.00"),
        },
      ],
    });

    expect(result).toMatchObject({
      status: "ready",
      event: {
        eventType: "same_currency_transfer",
        eventClass: "posting",
        eventDate: "2026-07-01",
        sourceRecordIds: ["record-checking-out", "record-savings-in"],
        legs: [
          {
            accountId: "account-checking",
            instrumentId: "instrument-sgd",
            currency: "SGD",
            amountValue: "-250.00",
          },
          {
            accountId: "account-savings",
            instrumentId: "instrument-sgd",
            currency: "SGD",
            amountValue: "250.00",
          },
        ],
      },
    });
  });

  it("keeps the transfer in review when one account window has a one-cent residual", () => {
    const result = prepareSameCurrencyTransfer({
      autoCommitEnabled: true,
      commitIdempotencyKey: "transfer-with-residual",
      records: [
        {
          id: "record-checking-out",
          accountId: "account-checking",
          instrumentId: "instrument-sgd",
          postedOn: "2026-07-01",
          currency: "SGD",
          accountBalanceDelta: "-250.00",
          eligibility: qualifiedRecord,
        },
        {
          id: "record-savings-in",
          accountId: "account-savings",
          instrumentId: "instrument-sgd",
          postedOn: "2026-07-01",
          currency: "SGD",
          accountBalanceDelta: "250.00",
          eligibility: qualifiedRecord,
        },
      ],
      reconciliationWindows: [
        {
          accountId: "account-checking",
          currency: "SGD",
          opening: qualifiedSnapshot("record-checking-opening", "1000.00"),
          closing: qualifiedSnapshot("record-checking-closing", "750.01"),
        },
        {
          accountId: "account-savings",
          currency: "SGD",
          opening: qualifiedSnapshot("record-savings-opening", "100.00"),
          closing: qualifiedSnapshot("record-savings-closing", "350.00"),
        },
      ],
    });

    expect(result).toEqual({
      status: "review",
      reasons: ["reconciliation_gap:account-checking:SGD"],
    });
  });

  it("derives reconciliation effects from the source records", () => {
    const result = prepareSameCurrencyTransfer({
      autoCommitEnabled: true,
      commitIdempotencyKey: "transfer-with-mismatched-records",
      records: [
        {
          id: "record-checking-out",
          accountId: "account-checking",
          instrumentId: "instrument-sgd",
          postedOn: "2026-07-01",
          currency: "SGD",
          accountBalanceDelta: "-200.00",
          eligibility: qualifiedRecord,
        },
        {
          id: "record-savings-in",
          accountId: "account-savings",
          instrumentId: "instrument-sgd",
          postedOn: "2026-07-01",
          currency: "SGD",
          accountBalanceDelta: "200.00",
          eligibility: qualifiedRecord,
        },
      ],
      reconciliationWindows: [
        {
          accountId: "account-checking",
          currency: "SGD",
          opening: qualifiedSnapshot("record-checking-opening", "1000.00"),
          closing: qualifiedSnapshot("record-checking-closing", "750.00"),
        },
        {
          accountId: "account-savings",
          currency: "SGD",
          opening: qualifiedSnapshot("record-savings-opening", "100.00"),
          closing: qualifiedSnapshot("record-savings-closing", "350.00"),
        },
      ],
    });

    expect(result).toEqual({
      status: "review",
      reasons: [
        "reconciliation_gap:account-checking:SGD",
        "reconciliation_gap:account-savings:SGD",
      ],
    });
  });

  it("rejects a zero-value pair as a transfer", () => {
    const result = prepareSameCurrencyTransfer({
      autoCommitEnabled: true,
      commitIdempotencyKey: "zero-transfer",
      records: [
        {
          id: "record-checking-zero",
          accountId: "account-checking",
          instrumentId: "instrument-sgd",
          postedOn: "2026-07-01",
          currency: "SGD",
          accountBalanceDelta: "-0.00",
          eligibility: qualifiedRecord,
        },
        {
          id: "record-savings-zero",
          accountId: "account-savings",
          instrumentId: "instrument-sgd",
          postedOn: "2026-07-01",
          currency: "SGD",
          accountBalanceDelta: "0.00",
          eligibility: qualifiedRecord,
        },
      ],
      reconciliationWindows: [
        {
          accountId: "account-checking",
          currency: "SGD",
          opening: qualifiedSnapshot("record-checking-opening", "1000.00"),
          closing: qualifiedSnapshot("record-checking-closing", "1000.00"),
        },
        {
          accountId: "account-savings",
          currency: "SGD",
          opening: qualifiedSnapshot("record-savings-opening", "100.00"),
          closing: qualifiedSnapshot("record-savings-closing", "100.00"),
        },
      ],
    });

    expect(result).toEqual({
      status: "review",
      reasons: ["transfer_requires_opposite_non_zero_sides"],
    });
  });
});
