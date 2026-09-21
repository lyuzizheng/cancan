import { describe, expect, it } from "vitest";

import {
  normalizeDisplayAmount,
  selectProviderDocumentPackage,
  twoDecimalMinorUnits,
  type ProviderDocumentPackage,
} from "./index";
import { createSyntheticProviderStatementFixture } from "./testing";
import type { SyntheticStatementFixture } from "./testing";

function selected(providerKey: string, documentType: string): ProviderDocumentPackage {
  const providerPackage = selectProviderDocumentPackage({
    providerKey,
    documentType,
    mimeType: "application/pdf",
  });
  if (!providerPackage) {
    throw new Error(`missing package for ${providerKey}/${documentType}`);
  }
  return providerPackage;
}

const packages = [
  selected("dbs", "bank_statement"),
  selected("dbs", "credit_card_statement"),
  selected("hsbc", "bank_statement"),
] as const;

function grouped(value: string): string {
  const negative = value.startsWith("-");
  const rest = negative ? value.slice(1) : value;
  const [whole = "", fraction] = rest.split(".");
  const groupedWhole = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${negative ? "-" : ""}${groupedWhole}${fraction === undefined ? "" : `.${fraction}`}`;
}

interface StatementRowLocator {
  row: number;
}

function recordRow(raw: Record<string, unknown>): number {
  const locator = raw.locator;
  if (locator === null || typeof locator !== "object" || Array.isArray(locator)) {
    throw new Error("fixture row requires an object locator");
  }
  if (!("row" in locator)) {
    throw new Error("fixture row locator requires a row");
  }
  const row = (locator as StatementRowLocator).row;
  if (typeof row !== "number") {
    throw new Error("fixture row locator requires a numeric row");
  }
  return row;
}

/**
 * Rewrite amount/balance observation texts to thousands-separated display
 * form. With `raw: "display"` the raw row keeps the display string (the
 * DBS/HSBC extractor path); with `raw: "plain"` only observations change,
 * exercising numeric-equivalence grounding for comma-stripped extracts.
 */
function groupedFixture(
  input: SyntheticStatementFixture,
  raw: "display" | "plain",
): SyntheticStatementFixture {
  const rows = [
    ...input.proposal.openingSnapshots,
    ...input.proposal.records,
    ...input.proposal.closingSnapshots,
  ];
  for (const record of rows) {
    const rawRecord = record.raw;
    const row = recordRow(rawRecord);
    for (const field of ["debit", "credit", "balance"] as const) {
      if (!(field in rawRecord)) {
        continue;
      }
      const value = rawRecord[field];
      if (typeof value !== "string") {
        continue;
      }
      const display = grouped(value);
      const observation = input.extractionBundle.observations.find(
        (candidate) => candidate.row === row && candidate.text === value,
      );
      if (!observation) {
        throw new Error(`missing row ${row} observation ${value}`);
      }
      observation.text = display;
      if (raw === "display") {
        rawRecord[field] = display;
      }
    }
  }
  return input;
}

function postingSide(providerPackage: ProviderDocumentPackage): "debit" | "credit" {
  return providerPackage.debitBalanceSign === 1 ? "debit" : "credit";
}

describe("thousands-separated statement amounts", () => {
  it.each(packages)(
    "$packageId books a row whose raw amounts use display grouping",
    async (providerPackage) => {
      const input = groupedFixture(
        createSyntheticProviderStatementFixture(providerPackage, [
          { description: "SALARY", side: postingSide(providerPackage), amount: "1234.56" },
        ]),
        "display",
      );

      const result = await providerPackage.validate(input);

      expect(result.status).toBe("valid");
    },
  );

  it.each(packages)(
    "$packageId keeps display grounding with canonical exact-decimal values",
    async (providerPackage) => {
      const input = groupedFixture(
        createSyntheticProviderStatementFixture(providerPackage, [
          { description: "SALARY", side: postingSide(providerPackage), amount: "1234.56" },
        ]),
        "display",
      );
      const record = input.proposal.records[0];
      if (!record?.amount || !record.accountBalanceDelta || !record.balanceAfter) {
        throw new Error("missing fixture money fields");
      }

      const rawAmount = record.raw.debit ?? record.raw.credit;
      expect(rawAmount).toBe("1,234.56");
      for (const money of [record.amount, record.accountBalanceDelta, record.balanceAfter]) {
        expect(money.value).not.toContain(",");
        expect(normalizeDisplayAmount(money.value)).toBe(money.value);
      }
      const inspection = providerPackage.recordContract.inspect(record.raw);
      expect(inspection.groundingValues).toContain("1,234.56");
      expect(inspection.canonical.amount?.value).toBe("1234.56");
      const canonicalAmount = inspection.canonical.amount?.value;
      if (canonicalAmount === undefined) {
        throw new Error("missing canonical amount");
      }
      expect(twoDecimalMinorUnits("1,234.56")).toBe(twoDecimalMinorUnits(canonicalAmount));
    },
  );

  it.each(packages)(
    "$packageId grounds a plain raw amount against a grouped observation",
    async (providerPackage) => {
      const input = groupedFixture(
        createSyntheticProviderStatementFixture(providerPackage, [
          { description: "SALARY", side: postingSide(providerPackage), amount: "1234.56" },
        ]),
        "plain",
      );

      const result = await providerPackage.validate(input);

      expect(result.status).toBe("valid");
    },
  );

  it.each(packages)("$packageId still rejects a forged grouped amount", async (providerPackage) => {
    const input = groupedFixture(
      createSyntheticProviderStatementFixture(providerPackage, [
        { description: "SALARY", side: postingSide(providerPackage), amount: "1234.56" },
      ]),
      "display",
    );
    const record = input.proposal.records[0];
    if (!record?.amount) {
      throw new Error("missing fixture amount");
    }
    record.amount.value = "1234.57";

    const result = await providerPackage.validate(input);

    expect(result).toMatchObject({
      status: "invalid",
      errors: expect.arrayContaining([
        { proposalRecordId: "posting-1", code: "canonical_record_mismatch" },
      ]),
    });
  });
});
