import type {
  ExtractionBundle,
  ProviderRecordContract,
  StructuredParseProposal,
} from "./contracts";

const syntheticRawKeys = new Set([
  "type",
  "date",
  "account",
  "description",
  "debit",
  "credit",
  "currency",
  "balance",
  "providerRecordId",
  "locator",
]);

function requiredString(raw: Record<string, unknown>, field: string): string {
  const value = raw[field];
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`synthetic row requires ${field}`);
  }
  return value;
}

function optionalString(raw: Record<string, unknown>, field: string): string | undefined {
  return raw[field] === undefined ? undefined : requiredString(raw, field);
}

function validateSyntheticRaw(raw: Record<string, unknown>): void {
  if (Object.keys(raw).some((key) => !syntheticRawKeys.has(key))) {
    throw new Error("synthetic row contains unsupported fields");
  }
  if (JSON.stringify(raw).length > 16_384) {
    throw new Error("synthetic row exceeds the bounded raw-record limit");
  }
  const locator = raw.locator;
  if (locator !== undefined && (!locator || Array.isArray(locator) || typeof locator !== "object")) {
    throw new Error("synthetic row locator must be an object");
  }
}

export const syntheticBankRecordContract: ProviderRecordContract = {
  inspect(raw) {
    validateSyntheticRaw(raw);
    const type = optionalString(raw, "type");
    const date = requiredString(raw, "date");
    const currency = requiredString(raw, "currency");
    const balance = requiredString(raw, "balance");
    const providerRecordId = optionalString(raw, "providerRecordId");

    if (type === "balance") {
      const account = requiredString(raw, "account");
      return {
        groundingValues: [
          type,
          date,
          account,
          balance,
          currency,
          ...(providerRecordId ? [providerRecordId] : []),
        ],
        identityProjection: {
          type,
          date,
          account,
          balance,
          currency,
        },
        canonical: {
          ...(providerRecordId ? { providerRecordId } : {}),
          postedOn: date,
          balanceAfter: { value: balance, currency },
        },
      };
    }
    if (type !== undefined) {
      throw new Error("synthetic transaction row has an unsupported type");
    }
    const description = requiredString(raw, "description");
    const debit = optionalString(raw, "debit");
    const credit = optionalString(raw, "credit");
    if ((debit ? 1 : 0) + (credit ? 1 : 0) !== 1) {
      throw new Error("synthetic transaction row requires exactly one debit or credit");
    }
    const amount = debit ?? (credit as string);
    const side = debit ? "debit" : "credit";
    return {
      groundingValues: [
        date,
        description,
        amount,
        currency,
        balance,
        ...(providerRecordId ? [providerRecordId] : []),
      ],
      identityProjection: {
        date,
        description,
        [side]: amount,
        currency,
      },
      canonical: {
        ...(providerRecordId ? { providerRecordId } : {}),
        postedOn: date,
        descriptionRaw: description,
        amount: { value: amount, currency },
        statementEntrySide: side,
        accountBalanceDelta: {
          value: debit ? `-${amount}` : amount,
          currency,
        },
        balanceAfter: { value: balance, currency },
      },
    };
  },
};

export function createSyntheticTransferFixture(): {
  semanticDocumentKey: string;
  extractionBundle: ExtractionBundle;
  proposal: StructuredParseProposal;
} {
  const observations = [
    ["balance", "2026-06-30", "checking-001", "1000.00", "SGD"],
    ["2026-07-01", "Transfer to savings", "250.00", "SGD", "750.00"],
    ["balance", "2026-07-01", "checking-001", "750.00", "SGD"],
    ["balance", "2026-06-30", "savings-002", "100.00", "SGD"],
    ["2026-07-01", "Transfer from checking", "250.00", "SGD", "350.00"],
    ["balance", "2026-07-01", "savings-002", "350.00", "SGD"],
  ].flatMap((values, rowIndex) =>
    values.map((text, columnIndex) => ({
      id: `cell-${rowIndex + 1}-${columnIndex + 1}`,
      kind: "table_cell" as const,
      row: rowIndex + 1,
      column: columnIndex + 1,
      text,
      engine: "synthetic-fixture",
      engineVersion: "1",
    })),
  );

  return {
    semanticDocumentKey: "synthetic:transfer:2026-07",
    extractionBundle: {
      sourceDocumentId: "document-transfer",
      fileSha256: "b".repeat(64),
      mimeType: "text/csv",
      observations,
      metadata: {
        extractionVersion: "native-observations-v1",
        observationCount: observations.length,
      },
    },
    proposal: {
      document: { providerKey: "synthetic-bank", documentType: "transfer_export" },
      accounts: [
        {
          proposalAccountId: "account-checking",
          accountType: "deposit_account",
          providerAccountId: "checking-001",
          maskedIdentifier: "••001",
          currency: "SGD",
        },
        {
          proposalAccountId: "account-savings",
          accountType: "deposit_account",
          providerAccountId: "savings-002",
          maskedIdentifier: "••002",
          currency: "SGD",
        },
      ],
      openingSnapshots: [
        {
          proposalRecordId: "record-checking-opening",
          recordType: "balance",
          eventType: "balance_snapshot",
          proposalAccountId: "account-checking",
          postedOn: "2026-06-30",
          balanceAfter: { value: "1000.00", currency: "SGD" },
          raw: {
            type: "balance",
            date: "2026-06-30",
            account: "checking-001",
            balance: "1000.00",
            currency: "SGD",
            locator: { row: 1 },
          },
        },
        {
          proposalRecordId: "record-savings-opening",
          recordType: "balance",
          eventType: "balance_snapshot",
          proposalAccountId: "account-savings",
          postedOn: "2026-06-30",
          balanceAfter: { value: "100.00", currency: "SGD" },
          raw: {
            type: "balance",
            date: "2026-06-30",
            account: "savings-002",
            balance: "100.00",
            currency: "SGD",
            locator: { row: 4 },
          },
        },
      ],
      records: [
        {
          proposalRecordId: "record-checking-out",
          recordType: "transaction",
          eventType: "same_currency_transfer",
          proposalAccountId: "account-checking",
          postedOn: "2026-07-01",
          descriptionRaw: "Transfer to savings",
          amount: { value: "250.00", currency: "SGD" },
          statementEntrySide: "debit",
          accountBalanceDelta: { value: "-250.00", currency: "SGD" },
          balanceAfter: { value: "750.00", currency: "SGD" },
          raw: {
            date: "2026-07-01",
            description: "Transfer to savings",
            debit: "250.00",
            currency: "SGD",
            balance: "750.00",
            locator: { row: 2 },
          },
        },
        {
          proposalRecordId: "record-savings-in",
          recordType: "transaction",
          eventType: "same_currency_transfer",
          proposalAccountId: "account-savings",
          postedOn: "2026-07-01",
          descriptionRaw: "Transfer from checking",
          amount: { value: "250.00", currency: "SGD" },
          statementEntrySide: "credit",
          accountBalanceDelta: { value: "250.00", currency: "SGD" },
          balanceAfter: { value: "350.00", currency: "SGD" },
          raw: {
            date: "2026-07-01",
            description: "Transfer from checking",
            credit: "250.00",
            currency: "SGD",
            balance: "350.00",
            locator: { row: 5 },
          },
        },
      ],
      closingSnapshots: [
        {
          proposalRecordId: "record-checking-closing",
          recordType: "balance",
          eventType: "balance_snapshot",
          proposalAccountId: "account-checking",
          postedOn: "2026-07-01",
          balanceAfter: { value: "750.00", currency: "SGD" },
          raw: {
            type: "balance",
            date: "2026-07-01",
            account: "checking-001",
            balance: "750.00",
            currency: "SGD",
            locator: { row: 3 },
          },
        },
        {
          proposalRecordId: "record-savings-closing",
          recordType: "balance",
          eventType: "balance_snapshot",
          proposalAccountId: "account-savings",
          postedOn: "2026-07-01",
          balanceAfter: { value: "350.00", currency: "SGD" },
          raw: {
            type: "balance",
            date: "2026-07-01",
            account: "savings-002",
            balance: "350.00",
            currency: "SGD",
            locator: { row: 6 },
          },
        },
      ],
    },
  };
}
