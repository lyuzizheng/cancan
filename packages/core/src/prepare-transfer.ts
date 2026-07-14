export interface RecordEligibility {
  sourceConfigured: boolean;
  providerVerified: boolean;
  profileQualified: boolean;
  schemaValid: boolean;
  rawGrounded: boolean;
  deterministicValidationPassed: boolean;
  accountResolved: boolean;
  duplicateFree: boolean;
  allocationComplete: boolean;
  veryHighConfidence: boolean;
  requiredLegsPresent: boolean;
}

export interface TransferSourceRecord {
  id: string;
  accountId: string;
  instrumentId: string;
  postedOn: string;
  currency: string;
  accountBalanceDelta: string;
  eligibility: RecordEligibility;
}

export interface ReconciliationWindow {
  accountId: string;
  currency: string;
  opening: ReconciliationSnapshot;
  closing: ReconciliationSnapshot;
}

export interface ReconciliationSnapshot {
  recordId: string;
  value: string;
  eligibility: RecordEligibility;
}

export interface PreparedTransferEvent {
  eventType: "same_currency_transfer";
  eventClass: "posting";
  eventDate: string;
  commitIdempotencyKey: string;
  sourceRecordIds: string[];
  legs: Array<{
    accountId: string;
    instrumentId: string;
    currency: string;
    amountValue: string;
  }>;
}

export type TransferPreparation =
  | { status: "ready"; event: PreparedTransferEvent }
  | { status: "review"; reasons: string[] };

interface ExactDecimal {
  coefficient: bigint;
  scale: number;
}

function parseExactDecimal(value: string): ExactDecimal | undefined {
  const match = /^(-?)(0|[1-9]\d*)(?:\.(\d+))?$/.exec(value);
  if (!match) {
    return undefined;
  }

  const fraction = match[3] ?? "";
  const sign = match[1] === "-" ? -1n : 1n;
  return {
    coefficient: sign * BigInt(`${match[2]}${fraction}`),
    scale: fraction.length,
  };
}

function scaleTo(value: ExactDecimal, scale: number): bigint {
  return value.coefficient * 10n ** BigInt(scale - value.scale);
}

function exactSumEquals(values: string[], expected: string): boolean {
  const parsedValues = values.map(parseExactDecimal);
  const parsedExpected = parseExactDecimal(expected);
  if (parsedValues.some((value) => value === undefined) || !parsedExpected) {
    return false;
  }

  const exactValues = parsedValues as ExactDecimal[];
  const scale = Math.max(parsedExpected.scale, ...exactValues.map((value) => value.scale));
  const sum = exactValues.reduce((total, value) => total + scaleTo(value, scale), 0n);
  return sum === scaleTo(parsedExpected, scale);
}

function recordIsQualified(record: { eligibility: RecordEligibility }): boolean {
  return Object.values(record.eligibility).every(Boolean);
}

export function prepareSameCurrencyTransfer(input: {
  autoCommitEnabled: boolean;
  commitIdempotencyKey: string;
  records: TransferSourceRecord[];
  reconciliationWindows: ReconciliationWindow[];
}): TransferPreparation {
  const reasons: string[] = [];

  if (!input.autoCommitEnabled) {
    reasons.push("auto_commit_disabled");
  }
  if (input.records.length !== 2) {
    reasons.push("transfer_requires_two_source_records");
  }
  if (!input.records.every(recordIsQualified)) {
    reasons.push("record_not_qualified");
  }

  const first = input.records[0];
  const second = input.records[1];
  if (!first || !second) {
    return { status: "review", reasons };
  }

  if (first.accountId === second.accountId) {
    reasons.push("transfer_requires_distinct_accounts");
  }
  if (first.currency !== second.currency) {
    reasons.push("transfer_currency_mismatch");
  }
  if (first.postedOn !== second.postedOn) {
    reasons.push("transfer_date_mismatch");
  }
  const firstDelta = parseExactDecimal(first.accountBalanceDelta);
  const secondDelta = parseExactDecimal(second.accountBalanceDelta);
  if (
    firstDelta &&
    secondDelta &&
    (firstDelta.coefficient === 0n ||
      secondDelta.coefficient === 0n ||
      (firstDelta.coefficient < 0n) === (secondDelta.coefficient < 0n))
  ) {
    reasons.push("transfer_requires_opposite_non_zero_sides");
  }
  if (!exactSumEquals([first.accountBalanceDelta, second.accountBalanceDelta], "0")) {
    reasons.push("transfer_sides_do_not_close");
  }

  for (const window of input.reconciliationWindows) {
    if (!recordIsQualified(window.opening) || !recordIsQualified(window.closing)) {
      reasons.push(`snapshot_not_qualified:${window.accountId}:${window.currency}`);
      continue;
    }
    const recordEffects = input.records
      .filter(
        (record) =>
          record.accountId === window.accountId && record.currency === window.currency,
      )
      .map((record) => record.accountBalanceDelta);
    if (!exactSumEquals([window.opening.value, ...recordEffects], window.closing.value)) {
      reasons.push(`reconciliation_gap:${window.accountId}:${window.currency}`);
    }
  }

  const expectedWindows = new Set(
    input.records.map((record) => `${record.accountId}:${record.currency}`),
  );
  const suppliedWindows = new Set(
    input.reconciliationWindows.map((window) => `${window.accountId}:${window.currency}`),
  );
  if ([...expectedWindows].some((key) => !suppliedWindows.has(key))) {
    reasons.push("missing_reconciliation_window");
  }

  if (reasons.length > 0) {
    return { status: "review", reasons };
  }

  return {
    status: "ready",
    event: {
      eventType: "same_currency_transfer",
      eventClass: "posting",
      eventDate: first.postedOn,
      commitIdempotencyKey: input.commitIdempotencyKey,
      sourceRecordIds: [first.id, second.id],
      legs: input.records.map((record) => ({
        accountId: record.accountId,
        instrumentId: record.instrumentId,
        currency: record.currency,
        amountValue: record.accountBalanceDelta,
      })),
    },
  };
}
