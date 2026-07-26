import type { ProviderDocumentPackage } from "../provider-document-package";
import {
  createStatementRecordContract,
  validateStatementPackage,
} from "../statement-package-validation";

const repaymentMappings = [
  {
    description: "DBS CARD PAYMENT",
    side: "debit",
    eventType: "credit_card_repayment",
  },
] as const;

const recordContract = createStatementRecordContract({
  debitBalanceSign: -1,
  creditBalanceSign: 1,
  repaymentMappings,
});

export const hsbcBankStatementV1: ProviderDocumentPackage = {
  packageId: "hsbc/bank_statement@1",
  providerKey: "hsbc",
  documentType: "bank_statement",
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
    "Extract only posted HSBC SGD bank-statement rows. Preserve the full provider account ID, statement Debit/Credit side, exact two-decimal values, raw row, opening balance, and closing balance. Map only an explicit Debit row labelled DBS CARD PAYMENT as a credit-card repayment. Never infer identity from a masked number.",
  reviewOnly: true,
  fingerprint: {
    requiredAnchors: ["HSBC", "ACCOUNT", "STATEMENT", "WITHDRAWAL", "DEPOSIT", "BALANCE"],
    excludedAnchorGroups: [["CREDIT LIMIT", "PAYMENT DUE DATE", "PREVIOUS BALANCE"]],
  },
  capabilities: {
    accountType: "deposit_account",
    currency: "SGD",
    recordTypes: ["balance", "transaction"],
    postingStatus: "posted",
  },
  limitations: [
    "SGD PDF account statements only",
    "posted rows with explicit Debit or Credit columns only",
    "repayment mapping is limited to an explicit Debit DBS CARD PAYMENT row",
  ],
  debitBalanceSign: -1,
  creditBalanceSign: 1,
  repaymentMappings,
  recordContract,
  validate(input) {
    return validateStatementPackage(hsbcBankStatementV1, input);
  },
};
