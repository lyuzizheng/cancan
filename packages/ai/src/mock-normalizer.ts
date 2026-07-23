import type { DocumentRoutingProposal } from "@cancan/parsers";

import type { NormalizeDocumentInput } from "./worker-protocol";

export type MockNormalizerResult =
  | { status: "classified"; proposal: DocumentRoutingProposal }
  | { status: "needs_attention"; reason: "unsupported_document" };

const fixtureMarker = "CANCAN_SYNTHETIC_STATEMENT_V1";

export function normalizeWithMock(input: NormalizeDocumentInput): MockNormalizerResult {
  const contains = (token: string) =>
    input.extractionBundle.observations.some((observation) => observation.text.includes(token));
  if (
    !contains(fixtureMarker) ||
    !contains("provider=synthetic-bank") ||
    !contains("statement_id=transfer-2026-07")
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
