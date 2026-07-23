import { describe, expect, it } from "vitest";

import { normalizeWithMock } from "./mock-normalizer";

function fixtureInput() {
  return {
    documentId: "document-fixture",
    extractionBundle: {
      sourceDocumentId: "document-fixture",
      fileSha256: "a".repeat(64),
      mimeType: "text/csv" as const,
      metadata: { extractionVersion: "native-observations-v1", observationCount: 3 },
      observations: [
        {
          id: "csv-row-1-column-1",
          kind: "table_cell" as const,
          row: 1,
          column: 1,
          text: "CANCAN_SYNTHETIC_STATEMENT_V1",
          engine: "rust-csv",
          engineVersion: "1.4.0",
        },
        {
          id: "csv-row-2-column-1",
          kind: "table_cell" as const,
          row: 2,
          column: 1,
          text: "provider=synthetic-bank",
          engine: "rust-csv",
          engineVersion: "1.4.0",
        },
        {
          id: "csv-row-3-column-1",
          kind: "table_cell" as const,
          row: 3,
          column: 1,
          text: "statement_id=transfer-2026-07",
          engine: "rust-csv",
          engineVersion: "1.4.0",
        },
      ],
    },
  };
}

describe("mock document normalizer", () => {
  it("classifies only the synthetic fixture and returns provider/account evidence", () => {
    const result = normalizeWithMock(fixtureInput());

    expect(result).toEqual({
      status: "classified",
      proposal: {
        document: {
          providerKey: "synthetic-bank",
          documentType: "transfer_export",
          statementId: "transfer-2026-07",
          statementPeriod: { from: "2026-07-01", to: "2026-07-31" },
        },
        accounts: [
          {
            proposalAccountId: "account-checking",
            accountType: "deposit_account",
            providerAccountId: "checking-001",
            maskedIdentifier: "••001",
            currency: "SGD",
          },
        ],
      },
    });
  });

  it("does not guess for unsupported evidence", () => {
    expect(
      normalizeWithMock({
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
