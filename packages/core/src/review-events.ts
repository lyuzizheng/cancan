export type ReviewEventType =
  | "credit_card_repayment"
  | "same_currency_transfer";

export interface ReviewSourceRecord {
  accountBalanceDelta: string;
  accountId: string;
  accountType: string;
  currency: string;
  id: string;
  instrumentId: string;
  postedOn: string;
}

export interface PreparedReviewEvent {
  eventClass: "posting";
  eventDate: string;
  eventType: ReviewEventType;
  legs: Array<{
    accountId: string;
    amountValue: string;
    currency: string;
    instrumentId: string;
  }>;
  sourceRecordIds: [string, string];
  spending: false;
}

export interface PreparedReversalEvent {
  eventClass: "posting";
  eventDate: string;
  eventType: `${ReviewEventType}_reversal`;
  legs: Array<{
    accountId: string;
    amountValue: string;
    currency: string;
    instrumentId: string;
  }>;
  spending: false;
}

export type RelationshipPreparation =
  | { event: PreparedReviewEvent; status: "ready" }
  | { reasons: string[]; status: "review" };

interface ExactDecimal {
  coefficient: bigint;
  scale: number;
}

const RELATIONSHIP_WINDOW_DAYS: Record<ReviewEventType, number> = {
  same_currency_transfer: 3,
  credit_card_repayment: 7,
};

export function relationshipWindowDays(eventType: ReviewEventType): number {
  return RELATIONSHIP_WINDOW_DAYS[eventType];
}

export function prepareReviewRelationship(input: {
  eventType: ReviewEventType;
  records: [ReviewSourceRecord, ReviewSourceRecord];
}): RelationshipPreparation {
  const [first, second] = input.records;
  const reasons = commonRelationshipReasons(first, second, input.eventType);
  const firstDelta = parseExactDecimal(first.accountBalanceDelta);
  const secondDelta = parseExactDecimal(second.accountBalanceDelta);

  if (!firstDelta || !secondDelta) {
    reasons.push("account_balance_delta_invalid");
  } else if (!equalMagnitude(firstDelta, secondDelta)) {
    reasons.push("relationship_magnitude_mismatch");
  } else if (firstDelta.coefficient === 0n || secondDelta.coefficient === 0n) {
    reasons.push("relationship_requires_non_zero_sides");
  } else if (input.eventType === "same_currency_transfer") {
    if ((firstDelta.coefficient < 0n) === (secondDelta.coefficient < 0n)) {
      reasons.push("transfer_requires_outgoing_and_incoming_sides");
    }
  } else if (!isCardRepayment(first, second, firstDelta, secondDelta)) {
    reasons.push("repayment_requires_cash_and_liability_decreases");
  }

  if (reasons.length > 0) {
    return { status: "review", reasons };
  }

  const eventDateRecord = canonicalDateRecord(input.eventType, first, second);
  const ordered = [first, second].sort((left, right) => left.id.localeCompare(right.id)) as [
    ReviewSourceRecord,
    ReviewSourceRecord,
  ];
  return {
    status: "ready",
    event: {
      eventType: input.eventType,
      eventClass: "posting",
      eventDate: eventDateRecord.postedOn,
      sourceRecordIds: [ordered[0].id, ordered[1].id],
      legs: ordered.map((record) => ({
        accountId: record.accountId,
        instrumentId: record.instrumentId,
        currency: record.currency,
        amountValue: record.accountBalanceDelta,
      })),
      spending: false,
    },
  };
}

export function findHistoricalRelationshipCandidates(input: {
  candidates: ReviewSourceRecord[];
  eventType: ReviewEventType;
  record: ReviewSourceRecord;
}): ReviewSourceRecord[] {
  return input.candidates.filter((candidate) => {
    if (candidate.id === input.record.id) {
      return false;
    }
    return (
      daysBetween(input.record.postedOn, candidate.postedOn) <=
        relationshipWindowDays(input.eventType) &&
      prepareReviewRelationship({
        eventType: input.eventType,
        records: [input.record, candidate],
      }).status === "ready"
    );
  });
}

export function prepareReviewReversal(input: {
  event: PreparedReviewEvent;
  eventDate: string;
}): PreparedReversalEvent {
  return {
    eventType: `${input.event.eventType}_reversal`,
    eventClass: "posting",
    eventDate: input.eventDate,
    legs: input.event.legs.map((leg) => ({
      ...leg,
      amountValue: invertExactDecimal(leg.amountValue),
    })),
    spending: false,
  };
}

function commonRelationshipReasons(
  first: ReviewSourceRecord,
  second: ReviewSourceRecord,
  eventType: ReviewEventType,
): string[] {
  const reasons: string[] = [];
  if (first.accountId === second.accountId) {
    reasons.push("relationship_requires_distinct_accounts");
  }
  if (first.currency !== second.currency) {
    reasons.push("relationship_currency_mismatch");
  }
  if (
    daysBetween(first.postedOn, second.postedOn) > relationshipWindowDays(eventType)
  ) {
    reasons.push("relationship_date_window_exceeded");
  }
  return reasons;
}

function canonicalDateRecord(
  eventType: ReviewEventType,
  first: ReviewSourceRecord,
  second: ReviewSourceRecord,
): ReviewSourceRecord {
  if (eventType === "same_currency_transfer") {
    return negativeDeltaRecord(first, second);
  }
  return first.accountType === "credit_card" ? second : first;
}

function negativeDeltaRecord(
  first: ReviewSourceRecord,
  second: ReviewSourceRecord,
): ReviewSourceRecord {
  const firstDelta = parseExactDecimal(first.accountBalanceDelta);
  const secondDelta = parseExactDecimal(second.accountBalanceDelta);
  if (firstDelta && firstDelta.coefficient < 0n) {
    return first;
  }
  if (secondDelta && secondDelta.coefficient < 0n) {
    return second;
  }
  return first;
}

function isCardRepayment(
  first: ReviewSourceRecord,
  second: ReviewSourceRecord,
  firstDelta: ExactDecimal,
  secondDelta: ExactDecimal,
): boolean {
  const firstIsCard = first.accountType === "credit_card";
  const secondIsCard = second.accountType === "credit_card";
  const nonCardAccountType = firstIsCard ? second.accountType : first.accountType;
  const nonCardIsCashLike =
    nonCardAccountType === "deposit_account" ||
    nonCardAccountType === "currency_balance" ||
    nonCardAccountType === "cash_balance";
  return (
    firstIsCard !== secondIsCard &&
    nonCardIsCashLike &&
    firstDelta.coefficient < 0n &&
    secondDelta.coefficient < 0n
  );
}

function daysBetween(first: string, second: string): number {
  const firstDay = parseIsoDate(first);
  const secondDay = parseIsoDate(second);
  if (firstDay === undefined || secondDay === undefined) {
    return Number.POSITIVE_INFINITY;
  }
  return Math.abs(firstDay - secondDay) / (24 * 60 * 60 * 1000);
}

function parseIsoDate(value: string): number | undefined {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return undefined;
  }
  const date = new Date(`${value}T00:00:00.000Z`);
  return date.toISOString().slice(0, 10) === value ? date.valueOf() : undefined;
}

function parseExactDecimal(value: string): ExactDecimal | undefined {
  const match = /^(-?)(0|[1-9]\d*)(?:\.(\d+))?$/.exec(value);
  if (!match) {
    return undefined;
  }
  const fraction = match[3] ?? "";
  return {
    coefficient: (match[1] === "-" ? -1n : 1n) * BigInt(`${match[2]}${fraction}`),
    scale: fraction.length,
  };
}

function equalMagnitude(first: ExactDecimal, second: ExactDecimal): boolean {
  const scale = Math.max(first.scale, second.scale);
  return abs(scaleTo(first, scale)) === abs(scaleTo(second, scale));
}

function invertExactDecimal(value: string): string {
  const parsed = parseExactDecimal(value);
  if (!parsed) {
    throw new Error("reversal requires an exact decimal amount");
  }
  const coefficient = -parsed.coefficient;
  const sign = coefficient < 0n ? "-" : "";
  const magnitude = abs(coefficient).toString().padStart(parsed.scale + 1, "0");
  if (parsed.scale === 0) {
    return `${sign}${magnitude}`;
  }
  return `${sign}${magnitude.slice(0, -parsed.scale)}.${magnitude.slice(-parsed.scale)}`;
}

function scaleTo(value: ExactDecimal, scale: number): bigint {
  return value.coefficient * 10n ** BigInt(scale - value.scale);
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}
