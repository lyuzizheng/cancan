import type { RecordEligibility } from "./prepare-transfer";

export interface PreparedBalanceObservationEvent {
  eventType: "balance_snapshot";
  eventClass: "observation";
  eventDate: string;
  commitIdempotencyKey: string;
  sourceRecordIds: [string];
  legs: [
    {
      accountId: string;
      instrumentId: string;
      currency: string;
      balanceValue: string;
    },
  ];
}

export type BalanceObservationPreparation =
  | { status: "ready"; event: PreparedBalanceObservationEvent }
  | { status: "review"; reasons: string[] };

export function prepareBalanceObservation(input: {
  autoCommitEnabled: boolean;
  commitIdempotencyKey: string;
  record: {
    id: string;
    accountId: string;
    instrumentId: string;
    observedOn: string;
    currency: string;
    balanceValue: string;
    eligibility: RecordEligibility;
  };
}): BalanceObservationPreparation {
  const reasons: string[] = [];
  if (!input.autoCommitEnabled) {
    reasons.push("auto_commit_disabled");
  }
  if (!Object.values(input.record.eligibility).every(Boolean)) {
    reasons.push("record_not_qualified");
  }
  if (!/^-?(0|[1-9]\d*)(?:\.\d+)?$/.test(input.record.balanceValue)) {
    reasons.push("balance_value_invalid");
  }
  if (reasons.length > 0) {
    return { status: "review", reasons };
  }

  return {
    status: "ready",
    event: {
      eventType: "balance_snapshot",
      eventClass: "observation",
      eventDate: input.record.observedOn,
      commitIdempotencyKey: input.commitIdempotencyKey,
      sourceRecordIds: [input.record.id],
      legs: [
        {
          accountId: input.record.accountId,
          instrumentId: input.record.instrumentId,
          currency: input.record.currency,
          balanceValue: input.record.balanceValue,
        },
      ],
    },
  };
}
