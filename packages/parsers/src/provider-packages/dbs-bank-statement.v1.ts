import type { ProviderDocumentPackage } from "../provider-document-package";
import {
  createStatementRecordContract,
  validateStatementPackage,
} from "../statement-package-validation";

const recordContract = createStatementRecordContract({
  debitBalanceSign: -1,
  creditBalanceSign: 1,
  repaymentMappings: [],
});

export const dbsBankStatementV1: ProviderDocumentPackage = {
  packageId: "dbs/bank_statement@1",
  providerKey: "dbs",
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
    "Extract only posted DBS SGD bank-statement rows. Preserve the full provider account ID, statement Debit/Credit side, exact two-decimal values, raw row, opening balance, and closing balance. Never infer identity from a masked number.",
  reviewOnly: true,
  fingerprint: {
    requiredAnchors: ["DBS", "Statement of Account", "WITHDRAWAL", "DEPOSIT", "BALANCE"],
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
    "requires a full provider account ID and exact opening-to-closing reconciliation",
  ],
  debitBalanceSign: -1,
  creditBalanceSign: 1,
  repaymentMappings: [],
  recordContract,
  validate(input) {
    return validateStatementPackage(dbsBankStatementV1, input);
  },
};
