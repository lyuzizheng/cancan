import type {
  CanonicalExternalRecordInput,
  ExtractionBundle,
  ProviderRecordContract,
  SourceObservation,
  StructuredDocumentIdentity,
  StructuredParseProposal,
  StructuredProposalValidation,
  ValidatedExternalRecord,
} from "./contracts";

/**
 * Canonical semantic document key. Byte-identical to the trusted host
 * derivation (`derive_semantic_document_key` in
 * `apps/desktop/src-tauri/src/runtime/mod.rs`):
 * `providerKey` + (`:${providerRootId}` when present) + `:${statementId}`.
 * Returns `undefined` when no canonical identity exists (missing/empty
 * statement ID); callers must reject rather than fall back.
 */
export function semanticDocumentKey(
  document: StructuredDocumentIdentity,
): string | undefined {
  const statementId = document.statementId;
  if (statementId === undefined || statementId === null || statementId.length === 0) {
    return undefined;
  }
  const providerRootId = document.providerRootId;
  if (providerRootId === undefined || providerRootId === null) {
    return `${document.providerKey}:${statementId}`;
  }
  return `${document.providerKey}:${providerRootId}:${statementId}`;
}

function normalized(value: string): string {
  return value.trim().replace(/\s+/g, " ");
}

function observationGroupKey(observation: SourceObservation): string {
  if (observation.row !== undefined) {
    return `page:${observation.page ?? 0}:row:${observation.row}`;
  }
  return `observation:${observation.id}`;
}

function containsDelimitedValue(text: string, expected: string, adjacent: RegExp): boolean {
  let start = text.indexOf(expected);
  while (start !== -1) {
    const before = text[start - 1];
    const after = text[start + expected.length];
    if ((!before || !adjacent.test(before)) && (!after || !adjacent.test(after))) {
      return true;
    }
    start = text.indexOf(expected, start + 1);
  }
  return false;
}

function numericBoundaryIsValid(text: string, index: number, neighborIndex: number): boolean {
  const adjacent = text[index];
  if (adjacent === undefined) {
    return true;
  }
  if (/[A-Za-z0-9_+-]/.test(adjacent)) {
    return false;
  }
  return !/[.,]/.test(adjacent) || !/\d/.test(text[neighborIndex] ?? "");
}

function containsNumericToken(text: string, expected: string): boolean {
  let start = text.indexOf(expected);
  while (start !== -1) {
    const end = start + expected.length;
    if (
      numericBoundaryIsValid(text, start - 1, start - 2) &&
      numericBoundaryIsValid(text, end, end + 1)
    ) {
      return true;
    }
    start = text.indexOf(expected, start + 1);
  }
  return false;
}

function nonTableTextGroundsValue(text: string, expected: string): boolean {
  if (/^\d{4}-\d{2}-\d{2}$/.test(expected)) {
    return containsDelimitedValue(text, expected, /[A-Za-z0-9_-]/);
  }
  if (exactDecimal(expected) || /^-?[1-9]\d{0,2}(?:,\d{3})+(?:\.\d+)?$/.test(expected)) {
    return containsNumericToken(text, expected);
  }
  if (/^[A-Z]{3}$/.test(expected)) {
    return containsDelimitedValue(text, expected, /[A-Za-z0-9_]/);
  }
  return text.includes(expected);
}

export const MAX_LOCATOR_ROW_SPAN = 4;

export type ParsedRecordLocator =
  | { kind: "absent" }
  | { kind: "malformed" }
  | { kind: "valid"; page?: number; row: number; rowEnd?: number };

function isLocatorCoordinate(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1;
}

/**
 * Parses `raw.locator` into a row range the validator narrows grounding to.
 * Shape: `{ page?: number, row: number, rowEnd?: number }` with
 * `0 <= rowEnd - row <= MAX_LOCATOR_ROW_SPAN`. `row` is required: a rowless
 * locator is malformed and rejects the record. Identity ignores the locator.
 */
export function parseRecordLocator(raw: Record<string, unknown>): ParsedRecordLocator {
  const locator = raw.locator;
  if (locator === undefined) {
    return { kind: "absent" };
  }
  if (locator === null || Array.isArray(locator) || typeof locator !== "object") {
    return { kind: "malformed" };
  }
  const { page, row, rowEnd } = locator as Record<string, unknown>;
  if (page !== undefined && !isLocatorCoordinate(page)) {
    return { kind: "malformed" };
  }
  if (!isLocatorCoordinate(row)) {
    return { kind: "malformed" };
  }
  const startRow = row as number;
  if (rowEnd !== undefined) {
    if (
      !isLocatorCoordinate(rowEnd) ||
      (rowEnd as number) < startRow ||
      (rowEnd as number) - startRow > MAX_LOCATOR_ROW_SPAN
    ) {
      return { kind: "malformed" };
    }
  }
  return {
    kind: "valid",
    ...(page === undefined ? {} : { page }),
    row: startRow,
    ...(rowEnd === undefined ? {} : { rowEnd: rowEnd as number }),
  };
}

function locatorMatchesObservation(
  locator: { page?: number; row: number; rowEnd?: number },
  observation: SourceObservation,
): boolean {
  if (observation.row === undefined) {
    return false;
  }
  if (
    locator.page !== undefined &&
    observation.page !== undefined &&
    observation.page !== locator.page
  ) {
    return false;
  }
  const rowEnd = locator.rowEnd ?? locator.row;
  return observation.row >= locator.row && observation.row <= rowEnd;
}

/**
 * Observations inside a record's locator range, merged into one grounding
 * group. Empty when the locator is absent/malformed or points at a range with
 * no observations. Pageless observations (CSV rows) match any locator page.
 */
export function groundingRegionObservations(
  bundle: ExtractionBundle,
  raw: Record<string, unknown>,
): SourceObservation[] {
  const locator = parseRecordLocator(raw);
  if (locator.kind !== "valid") {
    return [];
  }
  return bundle.observations.filter((observation) =>
    locatorMatchesObservation(locator, observation),
  );
}

function observationMatchesValue(observation: SourceObservation, expected: string): boolean {
  const text = normalized(observation.text);
  return (
    text === expected ||
    (observation.kind !== "table_cell" && nonTableTextGroundsValue(text, expected))
  );
}

function groundingHitsForValues(
  observations: SourceObservation[],
  values: string[],
): SourceObservation[] | undefined {
  const used: SourceObservation[] = [];
  for (const value of values) {
    const expected = normalized(value);
    const hit = observations.find((observation) => observationMatchesValue(observation, expected));
    if (hit === undefined) {
      return undefined;
    }
    used.push(hit);
  }
  return used;
}

function rawRecordIsGrounded(
  values: string[],
  bundle: ExtractionBundle,
  raw: Record<string, unknown>,
): boolean {
  if (
    values.length === 0 ||
    values.some((value) => typeof value !== "string" || normalized(value).length === 0)
  ) {
    return false;
  }

  const locator = parseRecordLocator(raw);
  if (locator.kind === "malformed") {
    return false;
  }
  if (locator.kind === "valid") {
    const region = groundingRegionObservations(bundle, raw);
    if (region.length === 0) {
      return false;
    }
    const used = groundingHitsForValues(region, values);
    if (used === undefined) {
      return false;
    }
    if (region.some(({ kind }) => kind === "table_cell")) {
      const anchored = used.some(
        (observation) => observation.kind === "table_cell" && observation.row === locator.row,
      );
      if (!anchored) {
        return false;
      }
    }
    return true;
  }

  const pool = bundle.observations.some((observation) => observation.row !== undefined)
    ? bundle.observations.filter((observation) => observation.row !== undefined)
    : bundle.observations;
  const groups = new Map<string, SourceObservation[]>();
  for (const observation of pool) {
    const key = observationGroupKey(observation);
    const observations = groups.get(key) ?? [];
    observations.push(observation);
    groups.set(key, observations);
  }

  return [...groups.values()].some((observations) => groundingHitsForValues(observations, values) !== undefined);
}

/**
 * Deterministic JSON serialization for financial identity hashing. Mirrors
 * `JSON.stringify` value semantics (object `undefined` members omitted, array
 * `undefined` holes become `null`, top-level `undefined` unrepresentable) but
 * orders object keys by UTF-16 code-unit comparison, which — unlike
 * `localeCompare` — is stable across ICU versions and never collapses distinct
 * strings (e.g. `ﬀ` vs `ff`) to equal.
 */
function stableJson(value: unknown): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableJson(entry) ?? "null").join(",")}]`;
  }
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
      .flatMap(([key, entry]) => {
        const encoded = stableJson(entry);
        return encoded === undefined ? [] : [`${JSON.stringify(key)}:${encoded}`];
      });
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value) as string | undefined;
}

function canonicalFieldsMatch(
  record: CanonicalExternalRecordInput,
  canonical: Partial<CanonicalExternalRecordInput>,
): boolean {
  const actual = record as unknown as Record<string, unknown>;
  return Object.entries(canonical).every(
    ([field, expected]) => stableJson(actual[field]) === stableJson(expected),
  );
}

async function sha256(value: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function duplicateValues(values: string[]): boolean {
  return new Set(values).size !== values.length;
}

function exactDecimal(value: string, allowNegative = true): boolean {
  const pattern = allowNegative
    ? /^-?(0|[1-9]\d*)(?:\.\d+)?$/
    : /^(0|[1-9]\d*)(?:\.\d+)?$/;
  return pattern.test(value);
}

function validDateOnly(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return false;
  }
  const date = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(date.valueOf()) && date.toISOString().slice(0, 10) === value;
}

function validOffsetTimestamp(value: string): boolean {
  const match = /^(\d{4}-\d{2}-\d{2})T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?(?:Z|[+-](?:(?:0\d|1[0-3]):[0-5]\d|14:00))$/.exec(
    value,
  );
  const date = match?.[1];
  return date !== undefined && validDateOnly(date);
}

function recordSchemaIsValid(record: CanonicalExternalRecordInput): boolean {
  if (
    record.postingStatus !== undefined &&
    record.postingStatus !== "provisional" &&
    record.postingStatus !== "posted"
  ) {
    return false;
  }
  if (record.postedOn && !validDateOnly(record.postedOn)) {
    return false;
  }
  if (record.transactionOn && !validDateOnly(record.transactionOn)) {
    return false;
  }
  if (record.postedAt !== undefined && !validOffsetTimestamp(record.postedAt)) {
    return false;
  }
  if (record.amount && !exactDecimal(record.amount.value, false)) {
    return false;
  }
  if (record.accountBalanceDelta && !exactDecimal(record.accountBalanceDelta.value)) {
    return false;
  }
  if (record.balanceAfter && !exactDecimal(record.balanceAfter.value)) {
    return false;
  }
  if (record.valuation && !exactDecimal(record.valuation.value)) {
    return false;
  }
  if (record.quantity && !exactDecimal(record.quantity)) {
    return false;
  }
  return record.raw !== null && !Array.isArray(record.raw) && typeof record.raw === "object";
}

export async function validateStructuredProposal(input: {
  semanticDocumentKey: string;
  extractionBundle: ExtractionBundle;
  proposal: StructuredParseProposal;
  recordContract: ProviderRecordContract;
}): Promise<StructuredProposalValidation> {
  const semanticKeyBytes = new TextEncoder().encode(input.semanticDocumentKey).length;
  if (input.semanticDocumentKey.length === 0 || semanticKeyBytes > 256) {
    return { status: "invalid", errors: [{ code: "semantic_document_key_invalid" }] };
  }
  if (semanticDocumentKey(input.proposal.document) !== input.semanticDocumentKey) {
    return { status: "invalid", errors: [{ code: "semantic_document_key_mismatch" }] };
  }
  const recordInputs = [
    ...input.proposal.openingSnapshots,
    ...input.proposal.records,
    ...input.proposal.closingSnapshots,
  ];
  const errors: Array<{ proposalRecordId?: string; code: string }> = [];

  if (duplicateValues(input.extractionBundle.observations.map(({ id }) => id))) {
    errors.push({ code: "duplicate_observation_id" });
  }
  if (duplicateValues(input.proposal.accounts.map(({ proposalAccountId }) => proposalAccountId))) {
    errors.push({ code: "duplicate_proposal_account_id" });
  }
  if (duplicateValues(recordInputs.map(({ proposalRecordId }) => proposalRecordId))) {
    errors.push({ code: "duplicate_proposal_record_id" });
  }
  const providerRecordIds = recordInputs.flatMap(({ providerRecordId }) =>
    providerRecordId === undefined ? [] : [providerRecordId],
  );
  if (duplicateValues(providerRecordIds)) {
    errors.push({ code: "duplicate_provider_record_id" });
  }

  const accountIds = new Set(input.proposal.accounts.map(({ proposalAccountId }) => proposalAccountId));
  const validated: ValidatedExternalRecord[] = [];
  const occurrenceByIdentity = new Map<string, number>();

  for (const record of recordInputs) {
    if (!recordSchemaIsValid(record)) {
      errors.push({ proposalRecordId: record.proposalRecordId, code: "schema_invalid" });
      continue;
    }
    if (record.proposalAccountId && !accountIds.has(record.proposalAccountId)) {
      errors.push({ proposalRecordId: record.proposalRecordId, code: "unknown_proposal_account" });
      continue;
    }

    let inspection;
    try {
      inspection = input.recordContract.inspect(record.raw);
    } catch {
      errors.push({ proposalRecordId: record.proposalRecordId, code: "raw_record_shape_invalid" });
      continue;
    }

    if (!rawRecordIsGrounded(inspection.groundingValues, input.extractionBundle, record.raw)) {
      errors.push({ proposalRecordId: record.proposalRecordId, code: "raw_record_not_grounded" });
      continue;
    }
    if (
      record.providerRecordId !== undefined &&
      inspection.canonical.providerRecordId !== record.providerRecordId
    ) {
      errors.push({
        proposalRecordId: record.proposalRecordId,
        code: "provider_record_id_not_grounded",
      });
      continue;
    }
    if (!canonicalFieldsMatch(record, inspection.canonical)) {
      errors.push({ proposalRecordId: record.proposalRecordId, code: "canonical_record_mismatch" });
      continue;
    }
    const projectedIdentity = stableJson(inspection.identityProjection);
    if (projectedIdentity === undefined) {
      errors.push({ proposalRecordId: record.proposalRecordId, code: "canonical_record_mismatch" });
      continue;
    }

    const identity = record.providerRecordId
      ? `provider:${input.proposal.document.providerKey}:${record.providerRecordId}`
      : await sha256(`${input.semanticDocumentKey}:${projectedIdentity}`);
    const stableRecordKey = record.providerRecordId
      ? identity
      : `${identity}:${(occurrenceByIdentity.get(identity) ?? 0) + 1}`;
    if (!record.providerRecordId) {
      occurrenceByIdentity.set(identity, (occurrenceByIdentity.get(identity) ?? 0) + 1);
    }

    validated.push({
      ...record,
      stableRecordKey,
      validation: {
        schemaValid: true,
        rawGrounded: true,
        deterministicValidationPassed: true,
      },
    });
  }

  if (errors.length > 0) {
    return { status: "invalid", errors };
  }

  const byId = new Map(validated.map((record) => [record.proposalRecordId, record]));
  const select = (records: CanonicalExternalRecordInput[]) =>
    records.map(({ proposalRecordId }) => byId.get(proposalRecordId) as ValidatedExternalRecord);

  return {
    status: "valid",
    document: input.proposal.document,
    accounts: input.proposal.accounts,
    openingSnapshots: select(input.proposal.openingSnapshots),
    records: select(input.proposal.records),
    closingSnapshots: select(input.proposal.closingSnapshots),
  };
}
