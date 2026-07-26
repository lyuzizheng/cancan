import {
  createSyntheticTransferFixture,
  syntheticBankRecordContract,
} from "@cancan/parsers/testing";
import {
  validateStructuredProposal,
  type StructuredProposalValidation,
} from "@cancan/parsers";

import type { NormalizeDocumentInput } from "./worker-protocol";

export type MockNormalizerResult =
  | {
      status: "classified";
      proposal: Extract<StructuredProposalValidation, { status: "valid" }>;
    }
  | { status: "needs_attention"; reason: "unsupported_document" };

const fixtureMarker = "CANCAN_SYNTHETIC_STATEMENT_V1";

export async function normalizeWithMock(
  input: NormalizeDocumentInput,
): Promise<MockNormalizerResult> {
  const contains = (token: string) =>
    input.extractionBundle.observations.some((observation) => observation.text.includes(token));
  if (
    !contains(fixtureMarker) ||
    !contains("provider=synthetic-bank") ||
    !contains("statement_id=transfer-2026-07")
  ) {
    return { status: "needs_attention", reason: "unsupported_document" };
  }

  const fixture = createSyntheticTransferFixture();
  const proposal = {
    ...fixture.proposal,
    document: {
      ...fixture.proposal.document,
      statementId: "transfer-2026-07",
      statementPeriod: { from: "2026-07-01", to: "2026-07-31" },
    },
  };
  const validation = await validateStructuredProposal({
    semanticDocumentKey: "synthetic-bank:transfer-2026-07",
    extractionBundle: input.extractionBundle,
    proposal,
    recordContract: syntheticBankRecordContract,
  });
  if (validation.status !== "valid") {
    return { status: "needs_attention", reason: "unsupported_document" };
  }

  return {
    status: "classified",
    proposal: validation,
  };
}
