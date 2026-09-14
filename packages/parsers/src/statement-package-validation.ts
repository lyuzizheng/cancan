import type {
  CanonicalExternalRecordInput,
  ProviderRecordContract,
  StructuredParseProposal,
  StructuredProposalValidation,
} from "./contracts";
import type { ProviderDocumentPackage } from "./provider-document-package";
import {
  groundingRegionObservations,
  parseRecordLocator,
  validateStructuredProposal,
} from "./validate-structured-proposal";

const rawKeys = new Set([
  "kind",
  "date",
  "description",
  "debit",
  "credit",
  "balance",
  "locator",
]);

function requiredString(raw: Record<string, unknown>, field: string): string {
  const value = raw[field];
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`statement row requires ${field}`);
  }
  return value;
}

function optionalString(raw: Record<string, unknown>, field: string): string | undefined {
  return raw[field] === undefined ? undefined : requiredString(raw, field);
}

function normalizedDescription(value: string): string {
  return value.trim().replace(/\s+/g, " ").toUpperCase();
}

function validateRaw(raw: Record<string, unknown>): void {
  if (Object.keys(raw).some((key) => !rawKeys.has(key)) || JSON.stringify(raw).length > 16_384) {
    throw new Error("unsupported statement row");
  }
  if (parseRecordLocator(raw).kind !== "valid") {
    throw new Error("statement row requires a valid locator");
  }
}

export function createStatementRecordContract(input: {
  debitBalanceSign: -1 | 1;
  creditBalanceSign: -1 | 1;
  repaymentMappings: ProviderDocumentPackage["repaymentMappings"];
}): ProviderRecordContract {
  return {
    inspect(raw) {
      validateRaw(raw);
      const kind = requiredString(raw, "kind");
      const date = requiredString(raw, "date");
      const balance = requiredString(raw, "balance");

      if (kind === "opening_balance" || kind === "closing_balance") {
        return {
          groundingValues: [date, balance],
          identityProjection: { date, balance },
          canonical: {
            postedOn: date,
            balanceAfter: { value: balance, currency: "SGD" },
          },
        };
      }
      if (kind !== "posting") {
        throw new Error("unsupported statement row kind");
      }

      const description = requiredString(raw, "description");
      const debit = optionalString(raw, "debit");
      const credit = optionalString(raw, "credit");
      if ((debit ? 1 : 0) + (credit ? 1 : 0) !== 1) {
        throw new Error("invalid posted statement row");
      }
      const side = debit ? "debit" : "credit";
      const amount = debit ?? (credit as string);
      const sign = side === "debit" ? input.debitBalanceSign : input.creditBalanceSign;
      const repayment = input.repaymentMappings.find(
        (mapping) =>
          mapping.side === side &&
          normalizedDescription(mapping.description) === normalizedDescription(description),
      );

      return {
        groundingValues: [
          date,
          description,
          amount,
          balance,
        ],
        identityProjection: {
          date,
          description,
          [side]: amount,
        },
        canonical: {
          postingStatus: "posted",
          ...(repayment ? { eventType: repayment.eventType } : {}),
          postedOn: date,
          descriptionRaw: description,
          amount: { value: amount, currency: "SGD" },
          statementEntrySide: side,
          accountBalanceDelta: {
            value: sign === -1 ? `-${amount}` : amount,
            currency: "SGD",
          },
          balanceAfter: { value: balance, currency: "SGD" },
        },
      };
    },
  };
}

function validDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return false;
  }
  const date = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(date.valueOf()) && date.toISOString().slice(0, 10) === value;
}

function twoDecimalMinorUnits(value: string): bigint | undefined {
  const match = /^(-?)(0|[1-9]\d*)\.(\d{2})$/.exec(value);
  if (!match) {
    return undefined;
  }
  const [, sign, whole, fraction] = match;
  const minorUnits = BigInt(`${whole}${fraction}`);
  return sign === "-" ? -minorUnits : minorUnits;
}

function fullProviderAccountId(value: string | undefined): value is string {
  return (
    value !== undefined &&
    /^[A-Za-z0-9-]{6,64}$/.test(value) &&
    !/[*•]/.test(value) &&
    !/[Xx]{3,}/.test(value)
  );
}

function observationContainsValue(text: string, expected: string): boolean {
  let start = text.indexOf(expected);
  while (start !== -1) {
    const before = text[start - 1];
    const after = text[start + expected.length];
    if (
      (!before || !/[A-Za-z0-9_-]/.test(before)) &&
      (!after || !/[A-Za-z0-9_-]/.test(after))
    ) {
      return true;
    }
    start = text.indexOf(expected, start + 1);
  }
  return false;
}

function documentGroundsValue(
  input: Parameters<typeof validateStructuredProposal>[0]["extractionBundle"],
  expected: string,
): boolean {
  return input.observations.some(({ text }) => observationContainsValue(text, expected));
}

function rawString(record: CanonicalExternalRecordInput, field: string): string | undefined {
  const value = record.raw[field];
  return typeof value === "string" ? value : undefined;
}

/**
 * Foreign-currency tripwire for statement rows. Only SG-plausible settlement
 * codes count, and only when the code sits next to an amount-like number, so
 * ordinary description words (TOP-UP, ANG MO KIO, SDN BHD) never trip it.
 * A region with no such code inherits document-level SGD grounding.
 */
const FOREIGN_SETTLEMENT_CURRENCIES: ReadonlySet<string> = new Set([
  "USD", "EUR", "GBP", "JPY", "AUD", "HKD", "CNY", "MYR", "THB", "IDR",
  "INR", "PHP", "VND", "KRW", "TWD", "NZD", "CAD", "CHF",
]);

const AMOUNT_ADJACENT_CURRENCY = new RegExp(
  String.raw`\b(${[...FOREIGN_SETTLEMENT_CURRENCIES].join("|")})\b\s*[-+]?[\d,]*\d(?:\.\d+)?` +
    String.raw`|\b[-+]?[\d,]*\d(?:\.\d+)?\s*\b(${[...FOREIGN_SETTLEMENT_CURRENCIES].join("|")})\b`,
);

function groundingRegionHasForeignCurrency(text: string): boolean {
  return AMOUNT_ADJACENT_CURRENCY.test(text.toUpperCase());
}
export async function validateStatementPackage(
  providerPackage: ProviderDocumentPackage,
  input: {
    semanticDocumentKey: string;
    extractionBundle: Parameters<typeof validateStructuredProposal>[0]["extractionBundle"];
    proposal: StructuredParseProposal;
  },
): Promise<StructuredProposalValidation> {
  const shared = await validateStructuredProposal({
    ...input,
    recordContract: providerPackage.recordContract,
  });
  if (shared.status === "invalid") {
    return shared;
  }

  const errors: Array<{ proposalRecordId?: string; code: string }> = [];
  const add = (code: string, proposalRecordId?: string) =>
    errors.push(proposalRecordId === undefined ? { code } : { proposalRecordId, code });
  const text = input.extractionBundle.observations
    .map(({ text: observationText }) => observationText)
    .join("\n")
    .toLowerCase();

  if (input.proposal.document.providerKey !== providerPackage.providerKey) {
    add("provider_mismatch");
  }
  if (input.proposal.document.documentType !== providerPackage.documentType) {
    add("document_type_mismatch");
  }
  const statementId = input.proposal.document.statementId;
  if (
    statementId === undefined ||
    statementId.length === 0 ||
    !documentGroundsValue(input.extractionBundle, statementId)
  ) {
    add("statement_id_not_grounded");
  }
  const providerRootId = input.proposal.document.providerRootId;
  if (
    providerRootId !== undefined &&
    (providerRootId.length === 0 ||
      !documentGroundsValue(input.extractionBundle, providerRootId))
  ) {
    add("provider_root_id_not_grounded");
  }
  if (!providerPackage.mimeTypes.includes(input.extractionBundle.mimeType)) {
    add("mime_type_mismatch");
  }
  if (
    providerPackage.fingerprint.requiredAnchors.some(
      (anchor) => !text.includes(anchor.toLowerCase()),
    ) ||
    providerPackage.fingerprint.excludedAnchorGroups.some((anchors) =>
      anchors.every((anchor) => text.includes(anchor.toLowerCase())),
    )
  ) {
    add("fingerprint_mismatch");
  }

  const from = input.proposal.document.statementPeriod?.from;
  const to = input.proposal.document.statementPeriod?.to;
  if (!from || !to || !validDate(from) || !validDate(to) || from > to) {
    add("statement_period_invalid");
  }

  const accountById = new Map(
    input.proposal.accounts.map((account) => [account.proposalAccountId, account]),
  );
  if (input.proposal.accounts.length === 0) {
    add("account_required");
  }
  for (const account of input.proposal.accounts) {
    if (
      account.accountType !== providerPackage.capabilities.accountType ||
      account.currency !== "SGD"
    ) {
      add("account_contract_mismatch");
    }
    if (
      !fullProviderAccountId(account.providerAccountId) ||
      account.providerAccountId === account.maskedIdentifier
    ) {
      add("provider_account_id_required");
    } else if (!documentGroundsValue(input.extractionBundle, account.providerAccountId)) {
      add("provider_account_id_not_grounded");
    }
  }
  if (!documentGroundsValue(input.extractionBundle, "SGD")) {
    add("currency_not_grounded");
  }

  const allRecords = [
    ...input.proposal.openingSnapshots,
    ...input.proposal.records,
    ...input.proposal.closingSnapshots,
  ];
  for (const snapshot of [
    ...input.proposal.openingSnapshots,
    ...input.proposal.closingSnapshots,
  ]) {
    if (snapshot.recordType !== "balance") {
      add("balance_snapshot_required", snapshot.proposalRecordId);
    }
  }
  for (const record of allRecords) {
    const account = record.proposalAccountId
      ? accountById.get(record.proposalAccountId)
      : undefined;
    if (!account) {
      add("proposal_account_required", record.proposalRecordId);
    }
    const date = record.postedOn;
    if (!date || !from || !to || date < from || date > to) {
      add("date_outside_statement_period", record.proposalRecordId);
    }
    const money = [record.amount, record.accountBalanceDelta, record.balanceAfter, record.valuation];
    if (
      money.some(
        (value) =>
          value !== undefined &&
          (value.currency !== "SGD" || twoDecimalMinorUnits(value.value) === undefined),
      )
    ) {
      add("currency_or_precision_unsupported", record.proposalRecordId);
    }
    const region = groundingRegionObservations(input.extractionBundle, record.raw);
    const regionText = region.map(({ text }) => text).join("\n");
    if (groundingRegionHasForeignCurrency(regionText)) {
      add("currency_or_precision_unsupported", record.proposalRecordId);
    }
  }

  for (const record of input.proposal.records) {
    if (
      record.recordType !== "transaction" ||
      record.postingStatus !== "posted" ||
      !record.amount ||
      !record.accountBalanceDelta ||
      !record.statementEntrySide
    ) {
      add("posted_transaction_required", record.proposalRecordId);
      continue;
    }
    const sign =
      record.statementEntrySide === "debit"
        ? providerPackage.debitBalanceSign
        : providerPackage.creditBalanceSign;
    const expectedDelta =
      sign === -1 ? `-${record.amount.value}` : record.amount.value;
    if (record.accountBalanceDelta.value !== expectedDelta) {
      add("statement_side_sign_mismatch", record.proposalRecordId);
    }
    const description = rawString(record, "description");
    const repayment = providerPackage.repaymentMappings.find(
      (mapping) =>
        mapping.side === record.statementEntrySide &&
        description !== undefined &&
        normalizedDescription(mapping.description) === normalizedDescription(description),
    );
    if (record.eventType !== repayment?.eventType) {
      add("repayment_mapping_unsupported", record.proposalRecordId);
    }
  }

  for (const account of input.proposal.accounts) {
    const opening = input.proposal.openingSnapshots.filter(
      ({ proposalAccountId }) => proposalAccountId === account.proposalAccountId,
    );
    const closing = input.proposal.closingSnapshots.filter(
      ({ proposalAccountId }) => proposalAccountId === account.proposalAccountId,
    );
    const postings = input.proposal.records.filter(
      ({ proposalAccountId }) => proposalAccountId === account.proposalAccountId,
    );
    if (opening.length !== 1 || closing.length !== 1) {
      add("opening_closing_snapshot_required");
      continue;
    }
    const openingRecord = opening[0] as CanonicalExternalRecordInput;
    const closingRecord = closing[closing.length - 1] as CanonicalExternalRecordInput;
    const snapshotIds = new Set([openingRecord.proposalRecordId, closingRecord.proposalRecordId]);
    const chainRows = [openingRecord, ...(postings as CanonicalExternalRecordInput[]), closingRecord].map(
      (record) => {
        const locator = parseRecordLocator(record.raw);
        const isSnapshot = snapshotIds.has(record.proposalRecordId);
        const delta = isSnapshot
          ? 0n
          : record.accountBalanceDelta
            ? twoDecimalMinorUnits(record.accountBalanceDelta.value)
            : undefined;
        return {
          record,
          order: locator.kind === "valid" ? locator.row : undefined,
          balance: record.balanceAfter ? twoDecimalMinorUnits(record.balanceAfter.value) : undefined,
          delta,
        };
      },
    );
    if (chainRows.some(({ order }) => order === undefined)) {
      add("statement_reconciliation_failed");
      continue;
    }
    const ordered = chainRows.sort((left, right) => (left.order as number) - (right.order as number));
    const head = ordered[0];
    if (head?.balance === undefined) {
      add("statement_reconciliation_failed", head?.record.proposalRecordId);
      continue;
    }
    let running = head.balance;
    for (const link of ordered.slice(1)) {
      if (link.delta === undefined) {
        add("statement_reconciliation_failed", link.record.proposalRecordId);
        break;
      }
      if (link.balance === undefined || running + link.delta !== link.balance) {
        add("statement_reconciliation_failed", link.record.proposalRecordId);
        break;
      }
      running = link.balance;
    }
  }

  return errors.length === 0 ? shared : { status: "invalid", errors };
}
