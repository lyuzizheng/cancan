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

function rawRecordIsGrounded(values: string[], bundle: ExtractionBundle): boolean {
  if (
    values.length === 0 ||
    values.some((value) => typeof value !== "string" || normalized(value).length === 0)
  ) {
    return false;
  }

  const groups = new Map<string, SourceObservation[]>();
  for (const observation of bundle.observations) {
    const key = observationGroupKey(observation);
    const observations = groups.get(key) ?? [];
    observations.push(observation);
    groups.set(key, observations);
  }

  return [...groups.values()].some((observations) =>
    values.every((value) => {
      const expected = normalized(value);
      return observations.some((observation) => {
        const text = normalized(observation.text);
        return (
          text === expected ||
          (observation.kind !== "table_cell" && nonTableTextGroundsValue(text, expected))
        );
      });
    }),
  );
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

    if (!rawRecordIsGrounded(inspection.groundingValues, input.extractionBundle)) {
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
