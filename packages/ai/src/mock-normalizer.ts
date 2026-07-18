import type { DocumentRoutingProposal } from "@cancan/parsers";

export interface NormalizeDocumentInput {
  documentId: string;
  mimeType: "application/pdf" | "text/csv";
  content: string;
}

export type MockNormalizerResult =
  | { status: "classified"; proposal: DocumentRoutingProposal }
  | { status: "needs_attention"; reason: "unsupported_document" };

const fixtureMarker = "CANCAN_SYNTHETIC_STATEMENT_V1";

export function normalizeWithMock(input: NormalizeDocumentInput): MockNormalizerResult {
  if (
    !input.content.includes(fixtureMarker) ||
    !input.content.includes("provider=synthetic-bank") ||
    !input.content.includes("statement_id=transfer-2026-07")
  ) {
    return { status: "needs_attention", reason: "unsupported_document" };
  }

  return {
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
  };
}
