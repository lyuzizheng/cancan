import { dbsBankStatementV1 } from "./provider-packages/dbs-bank-statement.v1";
import { dbsCreditCardStatementV1 } from "./provider-packages/dbs-credit-card-statement.v1";
import { hsbcBankStatementV1 } from "./provider-packages/hsbc-bank-statement.v1";
import type {
  ExtractionBundle,
  ProviderRecordContract,
  StructuredParseProposal,
  StructuredProposalValidation,
} from "./contracts";

export interface ProviderDocumentPackage {
  readonly packageId: string;
  readonly providerKey: string;
  readonly documentType: string;
  readonly mimeTypes: readonly string[];
  readonly versions: {
    readonly package: string;
    readonly parser: string;
    readonly skill: string;
    readonly prompt: string;
    readonly schema: string;
    readonly toolContract: string;
    readonly validator: string;
  };
  readonly normalizationPrompt: string;
  readonly reviewOnly: true;
  readonly fingerprint: {
    readonly requiredAnchors: readonly string[];
    readonly excludedAnchorGroups: readonly (readonly string[])[];
  };
  readonly capabilities: {
    readonly accountType: "deposit_account" | "credit_card";
    readonly currency: "SGD";
    readonly recordTypes: readonly ["balance", "transaction"];
    readonly postingStatus: "posted";
  };
  readonly limitations: readonly string[];
  readonly debitBalanceSign: -1 | 1;
  readonly creditBalanceSign: -1 | 1;
  readonly repaymentMappings: readonly {
    readonly description: string;
    readonly side: "debit" | "credit";
    readonly eventType: "credit_card_repayment";
  }[];
  readonly recordContract: ProviderRecordContract;
  validate(input: {
    semanticDocumentKey: string;
    extractionBundle: ExtractionBundle;
    proposal: StructuredParseProposal;
  }): Promise<StructuredProposalValidation>;
}

const providerDocumentPackages = [
  dbsBankStatementV1,
  dbsCreditCardStatementV1,
  hsbcBankStatementV1,
] as const satisfies readonly ProviderDocumentPackage[];

export function selectProviderDocumentPackage(input: {
  providerKey: string;
  documentType: string;
  mimeType: string;
}): ProviderDocumentPackage | undefined {
  return providerDocumentPackages.find(
    (providerPackage) =>
      providerPackage.providerKey === input.providerKey &&
      providerPackage.documentType === input.documentType &&
      providerPackage.mimeTypes.includes(input.mimeType),
  );
}
