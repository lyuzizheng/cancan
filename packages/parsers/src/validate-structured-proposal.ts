import type {
  CanonicalExternalRecordInput,
  ExtractionBundle,
  ProviderRecordContract,
  SourceObservation,
  StructuredParseProposal,
  StructuredProposalValidation,
  ValidatedExternalRecord,
} from "./contracts";

function normalized(value: string): string {
  return value.trim().replace(/\s+/g, " ");
}

function observationGroupKey(observation: SourceObservation): string {
  if (observation.row !== undefined) {
    return `page:${observation.page ?? 0}:row:${observation.row}`;
  }
  return `observation:${observation.id}`;
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
        return text === expected || (observation.kind !== "table_cell" && text.includes(expected));
      });
    }),
  );
}

function stableJson(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, entry]) => `${JSON.stringify(key)}:${stableJson(entry)}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value);
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

function recordSchemaIsValid(record: CanonicalExternalRecordInput): boolean {
  if (record.postedOn && !validDateOnly(record.postedOn)) {
    return false;
  }
  if (record.transactionOn && !validDateOnly(record.transactionOn)) {
    return false;
  }
  if (record.postedAt && !/(?:Z|[+-]\d{2}:\d{2})$/.test(record.postedAt)) {
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

    const identity = record.providerRecordId
      ? `provider:${input.proposal.document.providerKey}:${record.providerRecordId}`
      : await sha256(`${input.semanticDocumentKey}:${stableJson(inspection.identityProjection)}`);
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
