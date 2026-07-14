import { describe, expect, it } from "vitest";

import { prepareBalanceObservation } from "./prepare-balance-observation";

describe("prepareBalanceObservation", () => {
  it("prepares a source-backed balance anchor without inventing a posting", () => {
    const result = prepareBalanceObservation({
      autoCommitEnabled: true,
      commitIdempotencyKey: "balance:checking:2026-06-30",
      record: {
        id: "record-checking-opening",
        accountId: "account-checking",
        instrumentId: "instrument-sgd",
        observedOn: "2026-06-30",
        currency: "SGD",
        balanceValue: "1000.00",
        eligibility: {
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
        },
      },
    });

    expect(result).toEqual({
      status: "ready",
      event: {
        eventType: "balance_snapshot",
        eventClass: "observation",
        eventDate: "2026-06-30",
        commitIdempotencyKey: "balance:checking:2026-06-30",
        sourceRecordIds: ["record-checking-opening"],
        legs: [
          {
            accountId: "account-checking",
            instrumentId: "instrument-sgd",
            currency: "SGD",
            balanceValue: "1000.00",
          },
        ],
      },
    });
  });

  it("keeps a qualified balance observation in review when automatic add is disabled", () => {
    const result = prepareBalanceObservation({
      autoCommitEnabled: false,
      commitIdempotencyKey: "balance:checking:2026-06-30",
      record: {
        id: "record-checking-opening",
        accountId: "account-checking",
        instrumentId: "instrument-sgd",
        observedOn: "2026-06-30",
        currency: "SGD",
        balanceValue: "1000.00",
        eligibility: {
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
        },
      },
    });

    expect(result).toEqual({ status: "review", reasons: ["auto_commit_disabled"] });
  });
});
