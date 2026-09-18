import type { ExtractionBundle } from "@cancan/parsers";
import {
  findHistoricalRelationshipCandidates,
  prepareReviewRelationship,
  prepareReviewReversal,
  type PreparedReviewEvent,
  type ReviewEventType,
  type ReviewSourceRecord,
} from "@cancan/core";

export interface NormalizeDocumentInput {
  documentId: string;
  extractionBundle: ExtractionBundle;
}

export interface NormalizeCommand extends NormalizeDocumentInput {
  type: "normalize";
  requestId: string;
}

export type CoreCommand =
  | {
      input: {
        candidates: ReviewSourceRecord[];
        eventType: ReviewEventType;
        record: ReviewSourceRecord;
      };
      operation: "find_relationship_candidates";
      requestId: string;
      type: "core";
    }
  | {
      input: {
        eventType: ReviewEventType;
        records: [ReviewSourceRecord, ReviewSourceRecord];
      };
      operation: "prepare_review_relationship";
      requestId: string;
      type: "core";
    }
  | {
      input: { event: PreparedReviewEvent; eventDate: string };
      operation: "prepare_review_reversal";
      requestId: string;
      type: "core";
    };

export type CoreCommandResult =
  | {
      candidates: Array<{ id: string }>;
      status: "candidates";
    }
  | ReturnType<typeof prepareReviewRelationship>
  | { event: ReturnType<typeof prepareReviewReversal>; status: "ready" };

export type WorkerCommand = NormalizeCommand | CoreCommand | { type: "shutdown" };

/**
 * Spec 0015 parser error codes on the worker wire protocol. The three
 * deterministic rejections stay distinguishable from transient
 * provider/network failures so the job engine never spends money retrying
 * them. Exhaustive on purpose: a new code must declare its retryability here
 * before it compiles.
 */
export type WorkerErrorCode =
  | "invalid_command"
  | "command_failed"
  | "normalizer_budget_exhausted"
  | "structured_proposal_invalid"
  | "evidence_grounding_failed";

const WORKER_ERROR_RETRYABLE: Record<WorkerErrorCode, boolean> = {
  invalid_command: false,
  command_failed: true,
  normalizer_budget_exhausted: false,
  structured_proposal_invalid: false,
  evidence_grounding_failed: false,
};

export function workerErrorIsRetryable(code: WorkerErrorCode): boolean {
  return WORKER_ERROR_RETRYABLE[code];
}

export function workerErrorMessage(code: WorkerErrorCode): {
  type: "error";
  code: WorkerErrorCode;
  retryable: boolean;
} {
  return { type: "error", code, retryable: workerErrorIsRetryable(code) };
}

const REVIEW_EVENT_TYPES = new Set<ReviewEventType>([
  "credit_card_repayment",
  "same_currency_transfer",
]);
const REVIEW_RECORD_KEYS = [
  "accountBalanceDelta",
  "accountId",
  "accountType",
  "currency",
  "id",
  "instrumentId",
  "postedOn",
];

const BUNDLE_KEYS = [
  "fileSha256",
  "metadata",
  "mimeType",
  "observations",
  "sourceDocumentId",
];
const OBSERVATION_REQUIRED_KEYS = ["engine", "engineVersion", "id", "kind", "text"];
const OBSERVATION_OPTIONAL_KEYS = [
  "boundingBox",
  "column",
  "confidence",
  "page",
  "row",
  "textSpan",
];
const OBSERVATION_KINDS = new Set([
  "native_text",
  "ocr_text",
  "table_cell",
]);

const MAX_OBSERVATIONS_PER_BUNDLE = 10_000;
const MAX_RELATIONSHIP_CANDIDATES = 100;

export function parseWorkerCommand(value: unknown): WorkerCommand | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  if (value.type === "shutdown" && hasExactKeys(value, ["type"])) {
    return { type: "shutdown" };
  }
  if (value.type === "core") {
    return parseCoreCommand(value);
  }
  if (
    !hasExactKeys(value, ["documentId", "extractionBundle", "requestId", "type"]) ||
    value.type !== "normalize" ||
    !isNonEmptyString(value.requestId) ||
    !isNormalizeDocumentInput({
      documentId: value.documentId,
      extractionBundle: value.extractionBundle,
    })
  ) {
    return undefined;
  }
  return value as unknown as NormalizeCommand;
}

export function runCoreCommand(command: CoreCommand): CoreCommandResult {
  switch (command.operation) {
    case "find_relationship_candidates":
      return {
        status: "candidates",
        candidates: findHistoricalRelationshipCandidates(command.input).map(({ id }) => ({ id })),
      };
    case "prepare_review_relationship":
      return prepareReviewRelationship(command.input);
    case "prepare_review_reversal":
      return {
        status: "ready",
        event: prepareReviewReversal(command.input),
      };
  }
}

function parseCoreCommand(value: Record<string, unknown>): CoreCommand | undefined {
  if (!isNonEmptyString(value.requestId) || typeof value.operation !== "string") {
    return undefined;
  }
  if (value.operation === "find_relationship_candidates") {
    if (
      !hasExactKeys(value, ["input", "operation", "requestId", "type"]) ||
      !isRelationshipCandidatesInput(value.input)
    ) {
      return undefined;
    }
    return value as unknown as CoreCommand;
  }
  if (value.operation === "prepare_review_relationship") {
    if (
      !hasExactKeys(value, ["input", "operation", "requestId", "type"]) ||
      !isRelationshipPreparationInput(value.input)
    ) {
      return undefined;
    }
    return value as unknown as CoreCommand;
  }
  if (value.operation === "prepare_review_reversal") {
    if (
      !hasExactKeys(value, ["input", "operation", "requestId", "type"]) ||
      !isReviewReversalInput(value.input)
    ) {
      return undefined;
    }
    return value as unknown as CoreCommand;
  }
  return undefined;
}

function isRelationshipCandidatesInput(value: unknown): boolean {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["candidates", "eventType", "record"]) &&
    isReviewEventType(value.eventType) &&
    isReviewSourceRecord(value.record) &&
    Array.isArray(value.candidates) &&
    value.candidates.length <= MAX_RELATIONSHIP_CANDIDATES &&
    value.candidates.every(isReviewSourceRecord)
  );
}

function isRelationshipPreparationInput(value: unknown): boolean {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["eventType", "records"]) &&
    isReviewEventType(value.eventType) &&
    Array.isArray(value.records) &&
    value.records.length === 2 &&
    value.records.every(isReviewSourceRecord)
  );
}

function isReviewReversalInput(value: unknown): boolean {
  if (!isRecord(value) || !hasExactKeys(value, ["event", "eventDate"]) || !isIsoDate(value.eventDate)) {
    return false;
  }
  return isPreparedReviewEvent(value.event);
}

function isPreparedReviewEvent(value: unknown): boolean {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["eventClass", "eventDate", "eventType", "legs", "sourceRecordIds", "spending"]) ||
    value.eventClass !== "posting" ||
    !isReviewEventType(value.eventType) ||
    !isIsoDate(value.eventDate) ||
    value.spending !== false ||
    !Array.isArray(value.sourceRecordIds) ||
    value.sourceRecordIds.length !== 2 ||
    !value.sourceRecordIds.every(isNonEmptyString) ||
    !Array.isArray(value.legs) ||
    value.legs.length !== 2 ||
    !value.legs.every(isPreparedReviewLeg)
  ) {
    return false;
  }
  return true;
}

function isPreparedReviewLeg(value: unknown): boolean {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["accountId", "amountValue", "currency", "instrumentId"]) &&
    isNonEmptyString(value.accountId) &&
    isExactDecimal(value.amountValue) &&
    isNonEmptyString(value.currency) &&
    isNonEmptyString(value.instrumentId)
  );
}

function isReviewSourceRecord(value: unknown): value is ReviewSourceRecord {
  return (
    isRecord(value) &&
    hasExactKeys(value, REVIEW_RECORD_KEYS) &&
    isExactDecimal(value.accountBalanceDelta) &&
    isNonEmptyString(value.accountId) &&
    isNonEmptyString(value.accountType) &&
    isNonEmptyString(value.currency) &&
    isNonEmptyString(value.id) &&
    isNonEmptyString(value.instrumentId) &&
    isIsoDate(value.postedOn)
  );
}

function isReviewEventType(value: unknown): value is ReviewEventType {
  return typeof value === "string" && REVIEW_EVENT_TYPES.has(value as ReviewEventType);
}

function isIsoDate(value: unknown): boolean {
  return typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value);
}

function isExactDecimal(value: unknown): value is string {
  return typeof value === "string" && /^-?(0|[1-9]\d*)(?:\.\d+)?$/.test(value);
}

function isNormalizeDocumentInput(value: unknown): value is NormalizeDocumentInput {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["documentId", "extractionBundle"]) &&
    isNonEmptyString(value.documentId) &&
    isExtractionBundle(value.extractionBundle) &&
    value.extractionBundle.sourceDocumentId === value.documentId
  );
}

function isExtractionBundle(value: unknown): value is ExtractionBundle {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, BUNDLE_KEYS) ||
    !isNonEmptyString(value.sourceDocumentId) ||
    typeof value.fileSha256 !== "string" ||
    !/^[a-f0-9]{64}$/.test(value.fileSha256) ||
    (value.mimeType !== "application/pdf" &&
      value.mimeType !== "text/csv" &&
      value.mimeType !== "image/png" &&
      value.mimeType !== "image/jpeg") ||
    !isExtractionMetadata(value.metadata) ||
    !Array.isArray(value.observations) ||
    value.observations.length > MAX_OBSERVATIONS_PER_BUNDLE ||
    value.metadata.observationCount !== value.observations.length ||
    !value.observations.every((observation) =>
      isSourceObservation(observation, value.mimeType),
    )
  ) {
    return false;
  }
  const observationIds = new Set(
    value.observations.map((observation) => (observation as SourceObservationRecord).id),
  );
  return observationIds.size === value.observations.length;
}

type SourceObservationRecord = Record<string, unknown> & { id: string };

function isExtractionMetadata(value: unknown): value is {
  extractionVersion: "native-observations-v1" | "native-observations-v2";
  observationCount: number;
} {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["extractionVersion", "observationCount"]) &&
    (value.extractionVersion === "native-observations-v1" ||
      value.extractionVersion === "native-observations-v2") &&
    typeof value.observationCount === "number" &&
    Number.isSafeInteger(value.observationCount) &&
    value.observationCount >= 0
  );
}

function isSourceObservation(value: unknown, mimeType: unknown): boolean {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, OBSERVATION_REQUIRED_KEYS, OBSERVATION_OPTIONAL_KEYS) ||
    !isNonEmptyString(value.id) ||
    !isNonEmptyString(value.engine) ||
    !isNonEmptyString(value.engineVersion) ||
    typeof value.text !== "string" ||
    typeof value.kind !== "string" ||
    !OBSERVATION_KINDS.has(value.kind) ||
    !isOptionalPositiveInteger(value.page) ||
    !isOptionalPositiveInteger(value.row) ||
    !isOptionalPositiveInteger(value.column) ||
    !isOptionalTextSpan(value.textSpan) ||
    !isOptionalBoundingBox(value.boundingBox) ||
    !isOptionalFiniteNumber(value.confidence)
  ) {
    return false;
  }
  if (value.kind === "native_text") {
    return (
      value.page !== undefined &&
      value.textSpan !== undefined &&
      isTextSpanWithinText(value.textSpan, value.text) &&
      value.column === undefined &&
      value.boundingBox === undefined &&
      value.confidence === undefined
    );
  }
  if (value.kind === "table_cell") {
    return (
      value.row !== undefined &&
      value.column !== undefined &&
      value.page === undefined &&
      value.textSpan === undefined &&
      value.boundingBox === undefined &&
      value.confidence === undefined
    );
  }
  if (value.kind === "ocr_text") {
    return (
      value.column === undefined &&
      value.textSpan === undefined &&
      (mimeType === "image/png" || mimeType === "image/jpeg"
        ? value.page === undefined && value.boundingBox === undefined && value.row === undefined
        : value.page !== undefined && value.boundingBox !== undefined)
    );
  }
  return false;
}

function isOptionalPositiveInteger(value: unknown): boolean {
  return (
    value === undefined ||
    (typeof value === "number" && Number.isSafeInteger(value) && value > 0)
  );
}

function isOptionalFiniteNumber(value: unknown): boolean {
  return (
    value === undefined ||
    (typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 1)
  );
}

function isOptionalTextSpan(value: unknown): boolean {
  if (value === undefined) {
    return true;
  }
  if (!isRecord(value) || !hasExactKeys(value, ["end", "start"])) {
    return false;
  }
  const { end, start } = value;
  return (
    typeof start === "number" &&
    typeof end === "number" &&
    Number.isSafeInteger(start) &&
    Number.isSafeInteger(end) &&
    start >= 0 &&
    end >= start
  );
}

function isTextSpanWithinText(value: unknown, text: string): boolean {
  return (
    isRecord(value) &&
    typeof value.end === "number" &&
    value.end <= text.length
  );
}

function isOptionalBoundingBox(value: unknown): boolean {
  if (value === undefined) {
    return true;
  }
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["height", "width", "x", "y"]) ||
    ![value.x, value.y, value.width, value.height].every(
      (coordinate) =>
        typeof coordinate === "number" &&
        Number.isFinite(coordinate) &&
        coordinate >= 0 &&
        coordinate <= 1,
    )
  ) {
    return false;
  }
  return (
    (value.width as number) > 0 &&
    (value.height as number) > 0 &&
    (value.x as number) + (value.width as number) <= 1 &&
    (value.y as number) + (value.height as number) <= 1
  );
}

function hasExactKeys(
  value: Record<string, unknown>,
  required: string[],
  optional: string[] = [],
): boolean {
  const allowed = new Set([...required, ...optional]);
  return required.every((key) => key in value) && Object.keys(value).every((key) => allowed.has(key));
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && !Array.isArray(value) && typeof value === "object";
}
