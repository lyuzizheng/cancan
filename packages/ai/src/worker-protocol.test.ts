import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { parseWorkerCommand } from "./worker-protocol";

const rustNormalizerCommandFixture: unknown = JSON.parse(
  readFileSync(
    new URL("../fixtures/normalizer-command-v1.json", import.meta.url),
    "utf8",
  ),
);

function validCommand() {
  return {
    type: "normalize",
    requestId: "request-fixture",
    documentId: "document-fixture",
    extractionBundle: {
      sourceDocumentId: "document-fixture",
      fileSha256: "a".repeat(64),
      mimeType: "text/csv",
      metadata: { extractionVersion: "native-observations-v1", observationCount: 1 },
      observations: [
        {
          id: "csv-row-1-column-1",
          kind: "table_cell",
          row: 1,
          column: 1,
          text: "CANCAN_SYNTHETIC_STATEMENT_V1",
          engine: "rust-csv",
          engineVersion: "1.4.0",
        },
      ],
    },
  };
}

describe("normalizer worker protocol", () => {
  it("accepts the observation-bundle protocol", () => {
    expect(parseWorkerCommand(validCommand())).toMatchObject({
      type: "normalize",
      documentId: "document-fixture",
      requestId: "request-fixture",
    });
  });

  it("accepts the exact command fixture serialized by Rust", () => {
    expect(parseWorkerCommand(rustNormalizerCommandFixture)).toMatchObject({
      type: "normalize",
      documentId: "document-smoke",
      requestId: "build-smoke",
    });
  });

  it("accepts image OCR text without page or coordinate claims", () => {
    const command = validCommand();
    command.extractionBundle.mimeType = "image/png";
    command.extractionBundle.observations[0] = {
      id: "image-ocr-text-1",
      kind: "ocr_text",
      text: "CANCAN_SYNTHETIC_STATEMENT_V1",
      confidence: 0.99,
      engine: "apple-vision",
      engineVersion: "revision-3-accurate",
    } as never;

    expect(parseWorkerCommand(command)).toMatchObject({
      type: "normalize",
      documentId: "document-fixture",
    });
  });

  it("rejects observation kinds without a current producer", () => {
    const command = validCommand();
    command.extractionBundle.observations[0] = {
      id: "pdf-page-1-region",
      kind: "document_region",
      page: 1,
      text: "",
      boundingBox: { x: 0, y: 0, width: 1, height: 1 },
      engine: "fixture",
      engineVersion: "1",
    } as never;

    expect(parseWorkerCommand(command)).toBeUndefined();
  });

  it("rejects missing, lossy, and malformed extraction bundles", () => {
    const missingBundle = validCommand();
    delete (missingBundle as { extractionBundle?: unknown }).extractionBundle;

    const lossyContent = { ...validCommand(), content: "raw statement text" };

    const malformedObservation = validCommand();
    delete (
      malformedObservation.extractionBundle.observations[0] as {
        engineVersion?: unknown;
      }
    ).engineVersion;

    const mismatchedDocument = validCommand();
    mismatchedDocument.extractionBundle.sourceDocumentId = "other-document";

    const wrongVersion = validCommand();
    wrongVersion.extractionBundle.metadata.extractionVersion = "wrong-version";

    const wrongCount = validCommand();
    wrongCount.extractionBundle.metadata.observationCount = 999;

    const duplicateObservation = validCommand();
    const firstObservation = duplicateObservation.extractionBundle.observations[0]!;
    duplicateObservation.extractionBundle.observations.push({
      ...firstObservation,
    });
    duplicateObservation.extractionBundle.metadata.observationCount = 2;

    expect(parseWorkerCommand(missingBundle)).toBeUndefined();
    expect(parseWorkerCommand(lossyContent)).toBeUndefined();
    expect(parseWorkerCommand(malformedObservation)).toBeUndefined();
    expect(parseWorkerCommand(mismatchedDocument)).toBeUndefined();
    expect(parseWorkerCommand(wrongVersion)).toBeUndefined();
    expect(parseWorkerCommand(wrongCount)).toBeUndefined();
    expect(parseWorkerCommand(duplicateObservation)).toBeUndefined();
  });

  it("rejects spans, confidence, and coordinates outside the observation contract", () => {
    const spanPastText = validCommand();
    spanPastText.extractionBundle.observations[0] = {
      id: "pdf-page-1-native-text",
      kind: "native_text",
      page: 1,
      text: "账😊",
      textSpan: { start: 0, end: 4 },
      engine: "pdfkit",
      engineVersion: "macos-page-string-v1",
    } as never;

    const invalidConfidence = validCommand();
    invalidConfidence.extractionBundle.observations[0] = {
      id: "pdf-page-1-ocr-text",
      kind: "ocr_text",
      page: 1,
      text: "statement",
      boundingBox: { x: 0, y: 0, width: 1, height: 1 },
      confidence: 1.1,
      engine: "vision",
      engineVersion: "fixture",
    } as never;

    const negativeBox = validCommand();
    negativeBox.extractionBundle.mimeType = "application/pdf";
    negativeBox.extractionBundle.observations[0] = {
      id: "pdf-page-1-ocr-text",
      kind: "ocr_text",
      page: 1,
      text: "statement",
      boundingBox: { x: -1, y: 0, width: 1, height: 1 },
      engine: "vision",
      engineVersion: "fixture",
    } as never;

    const zeroSizeBox = validCommand();
    zeroSizeBox.extractionBundle.mimeType = "application/pdf";
    zeroSizeBox.extractionBundle.observations[0] = {
      id: "pdf-page-1-ocr-text",
      kind: "ocr_text",
      page: 1,
      text: "statement",
      boundingBox: { x: 0, y: 0, width: 0, height: 1 },
      engine: "vision",
      engineVersion: "fixture",
    } as never;

    const oversizedBox = validCommand();
    oversizedBox.extractionBundle.mimeType = "application/pdf";
    oversizedBox.extractionBundle.observations[0] = {
      id: "pdf-page-1-ocr-text",
      kind: "ocr_text",
      page: 1,
      text: "statement",
      boundingBox: { x: 0.75, y: 0, width: 0.5, height: 1 },
      engine: "vision",
      engineVersion: "fixture",
    } as never;

    expect(parseWorkerCommand(spanPastText)).toBeUndefined();
    expect(parseWorkerCommand(invalidConfidence)).toBeUndefined();
    expect(parseWorkerCommand(negativeBox)).toBeUndefined();
    expect(parseWorkerCommand(zeroSizeBox)).toBeUndefined();
    expect(parseWorkerCommand(oversizedBox)).toBeUndefined();
  });
});
