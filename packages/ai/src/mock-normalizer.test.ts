import { describe, expect, it } from "vitest";

import { normalizeWithMock } from "./mock-normalizer";

describe("mock document normalizer", () => {
  it("classifies only the synthetic fixture and returns provider/account evidence", () => {
    const result = normalizeWithMock({
      documentId: "document-fixture",
      mimeType: "text/csv",
      content: [
        "CANCAN_SYNTHETIC_STATEMENT_V1",
        "provider=synthetic-bank",
        "statement_id=transfer-2026-07",
      ].join("\n"),
    });

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
        mimeType: "application/pdf",
        content: "%PDF unknown provider",
      }),
    ).toEqual({ status: "needs_attention", reason: "unsupported_document" });
  });
});
