import type {
  CanonicalExternalRecordInput,
  ExtractionBundle,
  ProviderRecordContract,
  SourceObservation,
  StructuredParseProposal,
} from "./contracts";
import type { ProviderDocumentPackage } from "./provider-document-package";
import { semanticDocumentKey } from "./validate-structured-proposal";

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

  const proposal: StructuredParseProposal = {
    document: {
      providerKey: "synthetic-bank",
      documentType: "transfer_export",
      statementId: "transfer-2026-07",
    },
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
  };
  const key = semanticDocumentKey(proposal.document);
  if (key === undefined) {
    throw new Error("synthetic transfer fixture requires a statement identity");
  }
  return {
    semanticDocumentKey: key,
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
    proposal,
  };
}

export type SyntheticProviderStatementPosting = {
  amount: string;
  description: string;
  eventType?: "credit_card_repayment";
  side: "debit" | "credit";
};

function providerMinorUnits(value: string): bigint {
  const match = /^(-?)(\d+)\.(\d{2})$/.exec(value);
  if (!match) {
    throw new Error(`invalid synthetic provider amount ${value}`);
  }
  const [, sign, whole, fraction] = match;
  const valueInMinorUnits = BigInt(`${whole}${fraction}`);
  return sign === "-" ? -valueInMinorUnits : valueInMinorUnits;
}

function providerDecimal(value: bigint): string {
  const sign = value < 0n ? "-" : "";
  const absolute = value < 0n ? -value : value;
  return `${sign}${absolute / 100n}.${(absolute % 100n).toString().padStart(2, "0")}`;
}

function nativeFixtureObservation(id: string, text: string, row?: number): SourceObservation {
  return {
    id,
    kind: "native_text",
    page: 1,
    ...(row !== undefined ? { row } : {}),
    textSpan: { start: 0, end: text.length },
    text,
    engine: "synthetic-provider-fixture",
    engineVersion: "1",
  };
}

function defaultProviderStatementPostings(
  providerPackage: ProviderDocumentPackage,
): readonly SyntheticProviderStatementPosting[] {
  const repayment = providerPackage.repaymentMappings[0];
  return repayment
    ? [{ ...repayment, amount: "20.00" }]
    : [{ description: "GROCERIES", side: "debit", amount: "20.00" }];
}

export function createSyntheticProviderStatementFixture(
  providerPackage: ProviderDocumentPackage,
  postings: readonly SyntheticProviderStatementPosting[] = defaultProviderStatementPostings(
    providerPackage,
  ),
): {
  semanticDocumentKey: string;
  extractionBundle: ExtractionBundle;
  proposal: StructuredParseProposal;
} {
  const providerAccountId = `${providerPackage.providerKey.toUpperCase()}-123456789`;
  const accountId = "statement-account";
  const statementId = `${providerPackage.providerKey}-${providerPackage.documentType}-2026-07`;
  let running = 10_000n;
  const rawRows: Record<string, unknown>[] = [
    {
      kind: "opening_balance",
      date: "2026-07-01",
      balance: "100.00",
      locator: { row: 1 },
    },
  ];
  const records: CanonicalExternalRecordInput[] = postings.map((posting, index) => {
    const sign =
      posting.side === "debit"
        ? providerPackage.debitBalanceSign
        : providerPackage.creditBalanceSign;
    const delta = BigInt(sign) * providerMinorUnits(posting.amount);
    running += delta;
    const raw = {
      kind: "posting",
      date: `2026-07-${String(index + 2).padStart(2, "0")}`,
      description: posting.description,
      ...(posting.side === "debit"
        ? { debit: posting.amount }
        : { credit: posting.amount }),
      balance: providerDecimal(running),
      locator: { row: index + 2 },
    };
    rawRows.push(raw);
    return {
      proposalRecordId: `posting-${index + 1}`,
      recordType: "transaction",
      postingStatus: "posted",
      ...(posting.eventType ? { eventType: posting.eventType } : {}),
      proposalAccountId: accountId,
      postedOn: raw.date,
      descriptionRaw: posting.description,
      amount: { value: posting.amount, currency: "SGD" },
      statementEntrySide: posting.side,
      accountBalanceDelta: {
        value: providerDecimal(delta),
        currency: "SGD",
      },
      balanceAfter: { value: raw.balance, currency: "SGD" },
      raw,
    };
  });
  const closingDate = `2026-07-${String(postings.length + 2).padStart(2, "0")}`;
  const closingRaw = {
    kind: "closing_balance",
    date: closingDate,
    balance: providerDecimal(running),
    locator: { row: postings.length + 2 },
  };
  rawRows.push(closingRaw);

  const observations: SourceObservation[] = rawRows.flatMap((raw, rowIndex) =>
    Object.entries(raw)
      .filter(
        (entry): entry is [string, string] =>
          entry[0] !== "kind" && typeof entry[1] === "string",
      )
      .map(([, text], columnIndex) => ({
        id: `cell-${rowIndex + 1}-${columnIndex + 1}`,
        kind: "table_cell" as const,
        row: rowIndex + 1,
        column: columnIndex + 1,
        text,
        engine: "synthetic-provider-fixture",
        engineVersion: "1",
      })),
  );
  let currentRow = rawRows.length + 1;
  observations.push(
    nativeFixtureObservation(
      "fixture-marker",
      [
        "CANCAN_SYNTHETIC_PROVIDER_STATEMENT_V1",
        `provider=${providerPackage.providerKey}`,
        `document_type=${providerPackage.documentType}`,
        `package_id=${providerPackage.packageId}`,
        `statement_id=${statementId}`,
      ].join(" "),
      currentRow++,
    ),
    nativeFixtureObservation(
      "fingerprint",
      providerPackage.fingerprint.requiredAnchors.join(" "),
      currentRow++,
    ),
    nativeFixtureObservation(
      "provider-account-id",
      `Account number ${providerAccountId}`,
      currentRow++,
    ),
    nativeFixtureObservation(
      "statement-currency",
      "Statement currency SGD",
      currentRow++,
    ),
  );

  const proposal: StructuredParseProposal = {
    document: {
      providerKey: providerPackage.providerKey,
      documentType: providerPackage.documentType,
      statementId,
      statementPeriod: { from: "2026-07-01", to: closingDate },
    },
    accounts: [
      {
        proposalAccountId: accountId,
        accountType: providerPackage.capabilities.accountType,
        providerAccountId,
        maskedIdentifier: "••6789",
        currency: "SGD",
      },
    ],
    openingSnapshots: [
      {
        proposalRecordId: "opening",
        recordType: "balance",
        proposalAccountId: accountId,
        postedOn: "2026-07-01",
        balanceAfter: { value: "100.00", currency: "SGD" },
        raw: rawRows[0] as Record<string, unknown>,
      },
    ],
    records,
    closingSnapshots: [
      {
        proposalRecordId: "closing",
        recordType: "balance",
        proposalAccountId: accountId,
        postedOn: closingDate,
        balanceAfter: { value: closingRaw.balance, currency: "SGD" },
        raw: closingRaw,
      },
    ],
  };
  const key = semanticDocumentKey(proposal.document);
  if (key === undefined) {
    throw new Error("synthetic provider fixture requires a statement identity");
  }
  return {
    semanticDocumentKey: key,
    extractionBundle: {
      sourceDocumentId: `document-${providerPackage.packageId}`,
      fileSha256: "c".repeat(64),
      mimeType: "application/pdf",
      observations,
      metadata: {
        extractionVersion: "native-observations-v2",
        observationCount: observations.length,
      },
    },
    proposal,
  };
}
