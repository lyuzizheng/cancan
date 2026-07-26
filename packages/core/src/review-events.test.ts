import { describe, expect, it } from "vitest";

import {
  findHistoricalRelationshipCandidates,
  prepareReviewRelationship,
  prepareReviewReversal,
  relationshipWindowDays,
  type ReviewSourceRecord,
} from "./review-events";

function record(overrides: Partial<ReviewSourceRecord> = {}): ReviewSourceRecord {
  return {
    id: "record-hsbc-cash",
    accountId: "account-hsbc",
    accountType: "deposit_account",
    instrumentId: "instrument-sgd",
    postedOn: "2026-06-30",
    currency: "SGD",
    accountBalanceDelta: "-750.00",
    ...overrides,
  };
}

describe("review relationship rules", () => {
  it("finds a DBS card repayment across a month boundary from signed balance effects", () => {
    const cash = record();
    const card = record({
      id: "record-dbs-card",
      accountId: "account-dbs-card",
      accountType: "credit_card",
      postedOn: "2026-07-01",
      accountBalanceDelta: "-750.00",
    });

    expect(relationshipWindowDays("credit_card_repayment")).toBe(7);
    expect(
      findHistoricalRelationshipCandidates({
        eventType: "credit_card_repayment",
        record: cash,
        candidates: [card],
      }),
    ).toEqual([card]);
    expect(
      prepareReviewRelationship({
        eventType: "credit_card_repayment",
        records: [card, cash],
      }),
    ).toMatchObject({
      status: "ready",
      event: {
        eventType: "credit_card_repayment",
        eventDate: "2026-06-30",
        spending: false,
      },
    });
  });

  it("requires the non-card repayment side to be cash-like", () => {
    const card = record({
      id: "record-dbs-card",
      accountId: "account-dbs-card",
      accountType: "credit_card",
      postedOn: "2026-07-01",
    });

    for (const accountType of ["manual_liability", "brokerage_account"]) {
      expect(
        prepareReviewRelationship({
          eventType: "credit_card_repayment",
          records: [
            record({
              id: `record-${accountType}`,
              accountId: `account-${accountType}`,
              accountType,
            }),
            card,
          ],
        }),
      ).toEqual({
        status: "review",
        reasons: ["repayment_requires_cash_and_liability_decreases"],
      });
    }

    expect(
      prepareReviewRelationship({
        eventType: "credit_card_repayment",
        records: [record(), card],
      }),
    ).toMatchObject({ status: "ready" });
  });

  it("uses the outgoing transfer date and permits only the three-day transfer window", () => {
    const outgoing = record({ id: "record-outgoing", accountBalanceDelta: "-25.00" });
    const incoming = record({
      id: "record-incoming",
      accountId: "account-savings",
      postedOn: "2026-07-03",
      accountBalanceDelta: "25.00",
    });

    expect(relationshipWindowDays("same_currency_transfer")).toBe(3);
    expect(
      prepareReviewRelationship({
        eventType: "same_currency_transfer",
        records: [incoming, outgoing],
      }),
    ).toMatchObject({ status: "ready", event: { eventDate: "2026-06-30" } });
    expect(
      findHistoricalRelationshipCandidates({
        eventType: "same_currency_transfer",
        record: outgoing,
        candidates: [incoming],
      }),
    ).toEqual([incoming]);
  });

  it("keeps just-outside-window, partial, and sign-incompatible pairs in review", () => {
    const cash = record();
    const outsideWindow = record({
      id: "record-late-card",
      accountId: "account-card",
      accountType: "credit_card",
      postedOn: "2026-07-08",
      accountBalanceDelta: "-750.00",
    });
    const partial = record({
      id: "record-partial-card",
      accountId: "account-card",
      accountType: "credit_card",
      postedOn: "2026-07-01",
      accountBalanceDelta: "-700.00",
    });
    const wrongEffect = record({
      id: "record-card-increase",
      accountId: "account-card",
      accountType: "credit_card",
      postedOn: "2026-07-01",
      accountBalanceDelta: "750.00",
    });

    expect(
      findHistoricalRelationshipCandidates({
        eventType: "credit_card_repayment",
        record: cash,
        candidates: [outsideWindow, partial, wrongEffect],
      }),
    ).toEqual([]);
    expect(
      prepareReviewRelationship({
        eventType: "credit_card_repayment",
        records: [cash, outsideWindow],
      }),
    ).toEqual({ status: "review", reasons: ["relationship_date_window_exceeded"] });
  });

  it("builds a typed immutable reversal without turning repayments into spending", () => {
    const prepared = prepareReviewRelationship({
      eventType: "credit_card_repayment",
      records: [
        record(),
        record({
          id: "record-card",
          accountId: "account-card",
          accountType: "credit_card",
          postedOn: "2026-07-01",
        }),
      ],
    });
    if (prepared.status !== "ready") {
      throw new Error("expected repayment to be ready");
    }

    expect(prepareReviewReversal({ event: prepared.event, eventDate: "2026-07-02" })).toEqual({
      eventType: "credit_card_repayment_reversal",
      eventClass: "posting",
      eventDate: "2026-07-02",
      spending: false,
      legs: [
        {
          accountId: "account-card",
          instrumentId: "instrument-sgd",
          currency: "SGD",
          amountValue: "750.00",
        },
        {
          accountId: "account-hsbc",
          instrumentId: "instrument-sgd",
          currency: "SGD",
          amountValue: "750.00",
        },
      ],
    });
  });
});
