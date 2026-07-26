import { describe, expect, it } from "vitest";
import { createSyntheticTransferFixture } from "@cancan/parsers/testing";

import { normalizeWithMock } from "./mock-normalizer";

function fixtureInput() {
  const fixture = createSyntheticTransferFixture();
  fixture.extractionBundle.sourceDocumentId = "document-fixture";
  fixture.extractionBundle.observations.push(
    {
      id: "csv-row-1-column-6",
      kind: "table_cell",
      row: 1,
      column: 6,
      text: "CANCAN_SYNTHETIC_STATEMENT_V1",
      engine: "rust-csv",
      engineVersion: "1.4.0",
    },
    {
      id: "csv-row-2-column-6",
      kind: "table_cell",
      row: 2,
      column: 6,
      text: "provider=synthetic-bank",
      engine: "rust-csv",
      engineVersion: "1.4.0",
    },
    {
      id: "csv-row-3-column-6",
      kind: "table_cell",
      row: 3,
      column: 6,
      text: "statement_id=transfer-2026-07",
      engine: "rust-csv",
      engineVersion: "1.4.0",
    },
  );
  fixture.extractionBundle.metadata.observationCount = fixture.extractionBundle.observations.length;
  return {
    documentId: "document-fixture",
    extractionBundle: fixture.extractionBundle,
  };
}

describe("mock document normalizer", () => {
  it("returns the validated full synthetic proposal", async () => {
    const result = await normalizeWithMock(fixtureInput());

    expect(result.status).toBe("classified");
    if (result.status !== "classified") {
      throw new Error("expected the validated synthetic proposal");
    }
    expect(result.proposal.status).toBe("valid");
    expect(result.proposal.accounts).toHaveLength(2);
    expect(result.proposal.openingSnapshots).toHaveLength(2);
    expect(result.proposal.records).toHaveLength(2);
    expect(result.proposal.closingSnapshots).toHaveLength(2);
    expect(
      [
        ...result.proposal.openingSnapshots,
        ...result.proposal.records,
        ...result.proposal.closingSnapshots,
      ].every(
        (record) =>
          record.stableRecordKey.length > 0 &&
          record.validation.schemaValid &&
          record.validation.rawGrounded &&
          record.validation.deterministicValidationPassed,
      ),
    ).toBe(true);
  });

  it("does not guess for unsupported evidence", async () => {
    expect(
      await normalizeWithMock({
        documentId: "document-unknown",
        extractionBundle: {
          sourceDocumentId: "document-unknown",
          fileSha256: "b".repeat(64),
          mimeType: "application/pdf",
          metadata: { extractionVersion: "native-observations-v1", observationCount: 1 },
          observations: [
            {
              id: "pdf-page-1-native-text",
              kind: "native_text",
              page: 1,
              textSpan: { start: 0, end: 21 },
              text: "unknown provider text",
              engine: "pdfkit",
              engineVersion: "macos-page-string-v1",
            },
          ],
        },
      }),
    ).toEqual({ status: "needs_attention", reason: "unsupported_document" });
  });
});
