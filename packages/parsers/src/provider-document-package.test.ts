import { describe, expect, it } from "vitest";

import {
  selectProviderDocumentPackage,
  type ProviderDocumentPackage,
} from "./provider-document-package";
import {
  createSyntheticProviderStatementFixture,
  type SyntheticProviderStatementPosting,
} from "./testing";

type Posting = SyntheticProviderStatementPosting;

function selected(
  providerKey: string,
  documentType: string,
): ProviderDocumentPackage {
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

function minorUnits(value: string): bigint {
  const match = /^(-?)(\d+)\.(\d{2})$/.exec(value);
  if (!match) {
    throw new Error(`invalid test amount ${value}`);
  }
  const [, sign, whole, fraction] = match;
  const valueInMinorUnits = BigInt(`${whole}${fraction}`);
  return sign === "-" ? -valueInMinorUnits : valueInMinorUnits;
}

function decimal(value: bigint): string {
  const sign = value < 0n ? "-" : "";
  const absolute = value < 0n ? -value : value;
  return `${sign}${absolute / 100n}.${(absolute % 100n).toString().padStart(2, "0")}`;
}

function fixture(
  providerPackage: ProviderDocumentPackage,
  postings: readonly Posting[],
): ReturnType<typeof createSyntheticProviderStatementFixture> {
  return createSyntheticProviderStatementFixture(providerPackage, postings);
}

function namedCases(providerPackage: ProviderDocumentPackage) {
  const repayment = providerPackage.repaymentMappings[0];
  return [
    {
      name: "normal",
      postings: [{ description: "GROCERIES", side: "debit", amount: "20.00" }] as const,
    },
    {
      name: "edge",
      postings: [{ description: "REFUND", side: "credit", amount: "5.00" }] as const,
    },
    {
      name: "reconciliation",
      postings: repayment
        ? [
            {
              description: repayment.description,
              side: repayment.side,
              amount: "20.00",
              eventType: repayment.eventType,
            },
          ]
        : [
            { description: "TRANSFER OUT", side: "debit", amount: "20.00" },
            { description: "TRANSFER IN", side: "credit", amount: "5.00" },
          ],
    },
  ] satisfies readonly { name: string; postings: readonly Posting[] }[];
}

describe("provider document packages", () => {
  for (const providerPackage of packages) {
    for (const testCase of namedCases(providerPackage)) {
      it(`${providerPackage.packageId} validates its ${testCase.name} case`, async () => {
        const result = await providerPackage.validate(
          fixture(providerPackage, testCase.postings),
        );

        expect(result.status).toBe("valid");
      });
    }
  }

  it.each(packages)("$packageId accepts its positive fingerprint", async (providerPackage) => {
    const result = await providerPackage.validate(
      fixture(providerPackage, [
        { description: "GROCERIES", side: "debit", amount: "20.00" },
      ]),
    );

    expect(result.status).toBe("valid");
  });

  it.each(packages)(
    "$packageId rejects a negatively discriminated fingerprint",
    async (providerPackage) => {
      const input = fixture(providerPackage, [
        { description: "GROCERIES", side: "debit", amount: "20.00" },
      ]);
      const fingerprint = input.extractionBundle.observations.find(
        ({ id }) => id === "fingerprint",
      );
      if (!fingerprint) {
        throw new Error("missing fingerprint observation");
      }
      fingerprint.text += ` ${providerPackage.fingerprint.excludedAnchorGroups[0]?.join(" ")}`;

      const result = await providerPackage.validate(input);

      expect(result).toMatchObject({
        status: "invalid",
        errors: expect.arrayContaining([{ code: "fingerprint_mismatch" }]),
      });
    },
  );

  it.each(packages)("$packageId rejects sign inversion", async (providerPackage) => {
    const input = fixture(providerPackage, [
      { description: "GROCERIES", side: "debit", amount: "20.00" },
    ]);
    const record = input.proposal.records[0];
    if (!record?.accountBalanceDelta) {
      throw new Error("missing fixture delta");
    }
    record.accountBalanceDelta.value =
      record.accountBalanceDelta.value.startsWith("-") ? "20.00" : "-20.00";

    const result = await providerPackage.validate(input);

    expect(result).toMatchObject({
      status: "invalid",
      errors: expect.arrayContaining([
        { proposalRecordId: "posting-1", code: "canonical_record_mismatch" },
      ]),
    });
  });

  it.each(packages)(
    "$packageId rejects a 0.01 closing residual",
    async (providerPackage) => {
      const input = fixture(providerPackage, [
        { description: "GROCERIES", side: "debit", amount: "20.00" },
      ]);
      const closing = input.proposal.closingSnapshots[0];
      if (!closing?.balanceAfter) {
        throw new Error("missing closing snapshot");
      }
      const changed = decimal(minorUnits(closing.balanceAfter.value) + 1n);
      closing.balanceAfter.value = changed;
      closing.raw.balance = changed;
      const closingRow = input.proposal.records.length + 2;
      const previous = decimal(minorUnits(changed) - 1n);
      const exactBalanceObservation = input.extractionBundle.observations.find(
        ({ row, text }) => row === closingRow && text === previous,
      );
      if (!exactBalanceObservation) {
        throw new Error("missing closing balance observation");
      }
      exactBalanceObservation.text = changed;

      const result = await providerPackage.validate(input);

      expect(result).toMatchObject({
        status: "invalid",
        errors: expect.arrayContaining([{ code: "statement_reconciliation_failed" }]),
      });
    },
  );

  it.each(packages)(
    "$packageId rejects out-of-period dates, non-SGD accounts, and missing full account IDs",
    async (providerPackage) => {
      const mutations = [
        (input: ReturnType<typeof fixture>) => {
          input.proposal.document.statementPeriod = {
            from: "2026-08-01",
            to: "2026-08-31",
          };
          return "date_outside_statement_period";
        },
        (input: ReturnType<typeof fixture>) => {
          const account = input.proposal.accounts[0];
          if (account) {
            account.currency = "USD";
          }
          return "account_contract_mismatch";
        },
        (input: ReturnType<typeof fixture>) => {
          const account = input.proposal.accounts[0];
          if (account) {
            delete account.providerAccountId;
          }
          return "provider_account_id_required";
        },
        (input: ReturnType<typeof fixture>) => {
          const account = input.proposal.accounts[0];
          if (account) {
            account.providerAccountId = account.maskedIdentifier;
          }
          return "provider_account_id_required";
        },
        (input: ReturnType<typeof fixture>) => {
          const account = input.proposal.accounts[0];
          if (account) {
            account.providerAccountId = "UNGROUNDED-123456";
          }
          return "provider_account_id_not_grounded";
        },
        (input: ReturnType<typeof fixture>) => {
          const currency = input.extractionBundle.observations.find(
            ({ id }) => id === "statement-currency",
          );
          if (currency) {
            currency.text = "Statement currency USD";
          }
          return "currency_not_grounded";
        },
      ] as const;

      for (const mutate of mutations) {
        const input = fixture(providerPackage, [
          { description: "GROCERIES", side: "debit", amount: "20.00" },
        ]);
        const code = mutate(input);
        const result = await providerPackage.validate(input);
        expect(result).toMatchObject({
          status: "invalid",
          errors: expect.arrayContaining([expect.objectContaining({ code })]),
        });
      }
    },
  );

  it.each(packages)(
    "$packageId rejects a row assembled from incoherent observations",
    async (providerPackage) => {
      const input = fixture(providerPackage, [
        { description: "GROCERIES", side: "debit", amount: "20.00" },
      ]);
      const record = input.proposal.records[0];
      const opening = input.proposal.openingSnapshots[0];
      if (!record?.balanceAfter || !opening?.balanceAfter) {
        throw new Error("missing fixture balances");
      }
      record.raw.balance = opening.balanceAfter.value;
      record.balanceAfter.value = opening.balanceAfter.value;

      const result = await providerPackage.validate(input);

      expect(result).toMatchObject({
        status: "invalid",
        errors: expect.arrayContaining([
          { proposalRecordId: "posting-1", code: "raw_record_not_grounded" },
        ]),
      });
    },
  );

  it.each(packages)("$packageId accepts posted rows only", async (providerPackage) => {
    const input = fixture(providerPackage, [
      { description: "GROCERIES", side: "debit", amount: "20.00" },
    ]);
    const record = input.proposal.records[0];
    if (!record) {
      throw new Error("missing posting status fixture");
    }
    record.postingStatus = "provisional";

    const result = await providerPackage.validate(input);

    expect(result.status).toBe("invalid");
  });

  it.each([
    {
      providerPackage: packages[0],
      anchors: ["DBS", "Statement of Account", "WITHDRAWAL", "DEPOSIT", "BALANCE"],
    },
    {
      providerPackage: packages[1],
      anchors: ["DBS", "Account Statement", "CREDIT LIMIT", "PAYMENT DUE DATE", "PREVIOUS BALANCE"],
    },
    {
      providerPackage: packages[1],
      anchors: ["DBS", "Statement of Account", "CREDIT LIMIT", "PAYMENT DUE DATE", "PREVIOUS BALANCE"],
    },
    {
      providerPackage: packages[2],
      anchors: ["HSBC", "ACCOUNT", "STATEMENT", "WITHDRAWAL", "DEPOSIT", "BALANCE"],
    },
    {
      providerPackage: packages[2],
      anchors: ["HSBC", "Account Statement", "WITHDRAWAL", "DEPOSIT", "BALANCE"],
    },
  ])(
    "$providerPackage.packageId accepts observed safe anchor set %#",
    async ({ providerPackage, anchors }) => {
      const input = fixture(providerPackage, [
        { description: "GROCERIES", side: "debit", amount: "20.00" },
      ]);
      const fingerprint = input.extractionBundle.observations.find(
        ({ id }) => id === "fingerprint",
      );
      if (!fingerprint) {
        throw new Error("missing fingerprint observation");
      }
      fingerprint.text = anchors.join(" ");

      const result = await providerPackage.validate(input);

      expect(result.status).toBe("valid");
    },
  );

  it.each(packages)(
    "$packageId rejects unsupported repayment mapping",
    async (providerPackage) => {
      const input = fixture(providerPackage, [
        { description: "GROCERIES", side: "debit", amount: "20.00" },
      ]);
      const record = input.proposal.records[0];
      if (!record) {
        throw new Error("missing posting");
      }
      record.eventType = "credit_card_repayment";

      const result = await providerPackage.validate(input);

      expect(result).toMatchObject({
        status: "invalid",
        errors: expect.arrayContaining([
          {
            proposalRecordId: "posting-1",
            code: "repayment_mapping_unsupported",
          },
        ]),
      });
    },
  );

  it.each(packages)(
    "$packageId validates exact provider, document type, and MIME metadata",
    async (providerPackage) => {
      const mutations = [
        (input: ReturnType<typeof fixture>) => {
          input.proposal.document.providerKey = `${providerPackage.providerKey}-other`;
          return "provider_mismatch";
        },
        (input: ReturnType<typeof fixture>) => {
          input.proposal.document.documentType = `${providerPackage.documentType}_other`;
          return "document_type_mismatch";
        },
        (input: ReturnType<typeof fixture>) => {
          input.extractionBundle.mimeType = "text/csv";
          return "mime_type_mismatch";
        },
      ] as const;

      for (const mutate of mutations) {
        const input = fixture(providerPackage, [
          { description: "GROCERIES", side: "debit", amount: "20.00" },
        ]);
        const code = mutate(input);
        const result = await providerPackage.validate(input);
        expect(result).toMatchObject({
          status: "invalid",
          errors: expect.arrayContaining([expect.objectContaining({ code })]),
        });
      }
    },
  );

  it("selects only exact provider, document type, and PDF MIME triples", () => {
    expect(
      selectProviderDocumentPackage({
        providerKey: "dbs",
        documentType: "credit_card_statement",
        mimeType: "application/pdf",
      })?.packageId,
    ).toBe("dbs/credit_card_statement@1");
    expect(
      selectProviderDocumentPackage({
        providerKey: "DBS",
        documentType: "credit_card_statement",
        mimeType: "application/pdf",
      }),
    ).toBeUndefined();
    expect(
      selectProviderDocumentPackage({
        providerKey: "dbs",
        documentType: "credit-card-statement",
        mimeType: "application/pdf",
      }),
    ).toBeUndefined();
    expect(
      selectProviderDocumentPackage({
        providerKey: "dbs",
        documentType: "credit_card_statement",
        mimeType: "text/csv",
      }),
    ).toBeUndefined();
  });
});
