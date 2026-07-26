import type { ProviderDocumentPackage } from "../provider-document-package";
import {
  createStatementRecordContract,
  validateStatementPackage,
} from "../statement-package-validation";

const repaymentMappings = [
  {
    description: "PAYMENT - THANK YOU",
    side: "credit",
    eventType: "credit_card_repayment",
  },
] as const;

const recordContract = createStatementRecordContract({
  debitBalanceSign: 1,
  creditBalanceSign: -1,
  repaymentMappings,
});

export const dbsCreditCardStatementV1: ProviderDocumentPackage = {
  packageId: "dbs/credit_card_statement@1",
  providerKey: "dbs",
  documentType: "credit_card_statement",
  mimeTypes: ["application/pdf"],
  versions: {
    package: "1.0.0",
    parser: "1.0.0",
    skill: "1.0.0",
    prompt: "1.0.0",
    schema: "1.0.0",
    toolContract: "1.0.0",
    validator: "1.0.0",
  },
  normalizationPrompt:
    "Extract only posted DBS SGD credit-card statement rows. Preserve the full provider card ID, statement Debit/Credit side, exact two-decimal values, raw row, opening liability, and closing liability. Map only an explicit Credit row labelled PAYMENT - THANK YOU as a credit-card repayment. Never infer identity from a masked card number.",
  reviewOnly: true,
  fingerprint: {
    requiredAnchors: ["DBS", "CREDIT LIMIT", "PAYMENT DUE DATE", "PREVIOUS BALANCE"],
    excludedAnchorGroups: [["WITHDRAWAL", "DEPOSIT", "BALANCE"]],
  },
  capabilities: {
    accountType: "credit_card",
    currency: "SGD",
    recordTypes: ["balance", "transaction"],
    postingStatus: "posted",
  },
  limitations: [
    "SGD PDF credit-card statements only",
    "posted rows with explicit Debit or Credit columns only",
    "repayment mapping is limited to an explicit Credit PAYMENT - THANK YOU row",
  ],
  debitBalanceSign: 1,
  creditBalanceSign: -1,
  repaymentMappings,
  recordContract,
  validate(input) {
    return validateStatementPackage(dbsCreditCardStatementV1, input);
  },
};
