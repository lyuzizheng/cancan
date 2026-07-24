import type { ExtractionBundle } from "@cancan/parsers";

export interface NormalizeDocumentInput {
  documentId: string;
  extractionBundle: ExtractionBundle;
}

export interface NormalizeCommand extends NormalizeDocumentInput {
  type: "normalize";
  requestId: string;
}

export type WorkerCommand = NormalizeCommand | { type: "shutdown" };

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

export function parseWorkerCommand(value: unknown): WorkerCommand | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  if (value.type === "shutdown" && hasExactKeys(value, ["type"])) {
    return { type: "shutdown" };
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
  extractionVersion: "native-observations-v1";
  observationCount: number;
} {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["extractionVersion", "observationCount"]) &&
    value.extractionVersion === "native-observations-v1" &&
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
      value.row === undefined &&
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
      value.row === undefined &&
      value.column === undefined &&
      value.textSpan === undefined &&
      (mimeType === "image/png" || mimeType === "image/jpeg"
        ? value.page === undefined && value.boundingBox === undefined
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
