import { describe, expect, it } from "vitest";

import { semanticDocumentKey, validateStructuredProposal } from "./index";
import type { ExtractionBundle, StructuredParseProposal } from "./index";
import { createSyntheticTransferFixture, syntheticBankRecordContract } from "./testing";

describe("validateStructuredProposal", () => {
  it("grounds normalized records to coherent source rows", async () => {
    const result = await validateStructuredProposal({
      ...createSyntheticTransferFixture(),
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toMatchObject({
      status: "valid",
      openingSnapshots: [
        { proposalRecordId: "record-checking-opening" },
        { proposalRecordId: "record-savings-opening" },
      ],
      records: [
        {
          proposalRecordId: "record-checking-out",
          validation: {
            schemaValid: true,
            rawGrounded: true,
            deterministicValidationPassed: true,
          },
        },
        {
          proposalRecordId: "record-savings-in",
          validation: {
            schemaValid: true,
            rawGrounded: true,
            deterministicValidationPassed: true,
          },
        },
      ],
      closingSnapshots: [
        { proposalRecordId: "record-checking-closing" },
        { proposalRecordId: "record-savings-closing" },
      ],
    });
  });

  it("rejects a raw record assembled from values on different source rows", async () => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    if (!record) {
      throw new Error("synthetic fixture is missing its first record");
    }
    record.raw.balance = "350.00";
    record.balanceAfter = { value: "350.00", currency: "SGD" };

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "raw_record_not_grounded",
        },
      ],
    });
  });

  it("keeps stable record identity independent from optional locator metadata", async () => {
    const original = createSyntheticTransferFixture();
    const relocated = createSyntheticTransferFixture();
    const relocatedRecord = relocated.proposal.records[0];
    if (!relocatedRecord) {
      throw new Error("synthetic fixture is missing its first record");
    }
    relocatedRecord.raw.locator = { page: 4, row: 99, confidence: 0.72 };

    const first = await validateStructuredProposal({
      ...original,
      recordContract: syntheticBankRecordContract,
    });
    const second = await validateStructuredProposal({
      ...relocated,
      recordContract: syntheticBankRecordContract,
    });
    if (first.status !== "valid" || second.status !== "valid") {
      throw new Error("synthetic fixtures did not validate");
    }

    expect(second.records[0]?.stableRecordKey).toBe(first.records[0]?.stableRecordKey);
  });

  it("rejects non-canonical decimal forms before a record can be eligible", async () => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    const amountObservation = fixture.extractionBundle.observations.find(
      ({ row, column }) => row === 2 && column === 3,
    );
    if (!record || !amountObservation) {
      throw new Error("synthetic fixture is incomplete");
    }
    record.raw.debit = "2.5e2";
    record.amount = { value: "2.5e2", currency: "SGD" };
    record.accountBalanceDelta = { value: "-2.5e2", currency: "SGD" };
    amountObservation.text = "2.5e2";

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "schema_invalid",
        },
      ],
    });
  });

  it.each(["pending", "POSTED", ""])(
    "rejects an invalid postingStatus: %s",
    async (postingStatus) => {
      const fixture = createSyntheticTransferFixture();
      const record = fixture.proposal.records[0];
      if (!record) {
        throw new Error("synthetic fixture is missing its first record");
      }
      record.postingStatus = postingStatus as "posted";

      const result = await validateStructuredProposal({
        ...fixture,
        recordContract: syntheticBankRecordContract,
      });

      expect(result).toEqual({
        status: "invalid",
        errors: [
          {
            proposalRecordId: "record-checking-out",
            code: "schema_invalid",
          },
        ],
      });
    },
  );

  it("preserves a valid postingStatus in the normalized proposal", async () => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    if (!record) {
      throw new Error("synthetic fixture is missing its first record");
    }
    record.postingStatus = "posted";

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result.status).toBe("valid");
    if (result.status !== "valid") {
      throw new Error("expected the synthetic proposal to validate");
    }
    expect(result.records[0]).toMatchObject({
      proposalRecordId: "record-checking-out",
      postingStatus: "posted",
    });
  });

  it("does not treat a numeric substring as an exact table-cell match", async () => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    if (!record) {
      throw new Error("synthetic fixture is missing its first record");
    }
    record.raw.debit = "25";
    record.amount = { value: "25", currency: "SGD" };
    record.accountBalanceDelta = { value: "-25", currency: "SGD" };

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "raw_record_not_grounded",
        },
      ],
    });
  });

  it.each([
    {
      label: "numeric substring",
      column: 3,
      sourceText: "250.00",
      mutateRecord(record: ReturnType<typeof createSyntheticTransferFixture>["proposal"]["records"][number]) {
        record.raw.debit = "25";
        record.amount = { value: "25", currency: "SGD" };
        record.accountBalanceDelta = { value: "-25", currency: "SGD" };
      },
    },
    {
      label: "numeric identifier substring",
      column: 3,
      sourceText: "REF25A",
      mutateRecord(record: ReturnType<typeof createSyntheticTransferFixture>["proposal"]["records"][number]) {
        record.raw.debit = "25";
        record.amount = { value: "25", currency: "SGD" };
        record.accountBalanceDelta = { value: "-25", currency: "SGD" };
      },
    },
    {
      label: "date prefix",
      column: 1,
      sourceText: "2026-07-010",
      mutateRecord() {},
    },
    {
      label: "currency substring",
      column: 4,
      sourceText: "XSGD",
      mutateRecord() {},
    },
    {
      label: "currency identifier substring",
      column: 4,
      sourceText: "REF_SGD_X",
      mutateRecord() {},
    },
  ])("does not ground a $label from OCR text", async ({ column, sourceText, mutateRecord }) => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    const observation = fixture.extractionBundle.observations.find(
      ({ row, column: observationColumn }) => row === 2 && observationColumn === column,
    );
    if (!record || !observation) {
      throw new Error("synthetic fixture is incomplete");
    }
    observation.kind = "ocr_text";
    observation.text = sourceText;
    mutateRecord(record);

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "raw_record_not_grounded",
        },
      ],
    });
  });

  it("grounds an exact numeric OCR token followed by punctuation", async () => {
    const fixture = createSyntheticTransferFixture();
    const observation = fixture.extractionBundle.observations.find(
      ({ row, column }) => row === 2 && column === 3,
    );
    if (!observation) {
      throw new Error("synthetic fixture is missing its debit observation");
    }
    observation.kind = "ocr_text";
    observation.text = "Amount 250.00, SGD";

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result.status).toBe("valid");
  });

  it.each([
    { sourceText: "11,250.00", expectedStatus: "invalid" },
    { sourceText: "Amount 1,250.00, SGD", expectedStatus: "valid" },
  ] as const)(
    "handles a grouped numeric OCR token without substring matching: $sourceText",
    async ({ sourceText, expectedStatus }) => {
      const fixture = createSyntheticTransferFixture();
      const observation = fixture.extractionBundle.observations.find(
        ({ row, column }) => row === 2 && column === 3,
      );
      if (!observation) {
        throw new Error("synthetic fixture is missing its debit observation");
      }
      observation.kind = "ocr_text";
      observation.text = sourceText;
      const recordContract: typeof syntheticBankRecordContract = {
        inspect(raw) {
          const inspection = syntheticBankRecordContract.inspect(raw);
          return raw.description === "Transfer to savings"
            ? {
                ...inspection,
                groundingValues: inspection.groundingValues.map((value) =>
                  value === "250.00" ? "1,250.00" : value,
                ),
              }
            : inspection;
        },
      };

      const result = await validateStructuredProposal({ ...fixture, recordContract });

      if (expectedStatus === "valid") {
        expect(result.status).toBe("valid");
      } else {
        expect(result).toEqual({
          status: "invalid",
          errors: [
            {
              proposalRecordId: "record-checking-out",
              code: "raw_record_not_grounded",
            },
          ],
        });
      }
    },
  );

  it.each([
    "",
    "not-a-dateZ",
    "2026-07-01T12:00:00+99:99",
    "2026-07-01T12:00:00+23:59",
    "2026-07-01T12:00:00+14:01",
    "2026-02-30T12:00:00Z",
    "2026-07-01T24:00:00Z",
  ])("rejects an invalid postedAt timestamp: %s", async (postedAt) => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    if (!record) {
      throw new Error("synthetic fixture is missing its first record");
    }
    record.postedAt = postedAt;

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "schema_invalid",
        },
      ],
    });
  });

  it.each([
    "2026-07-01T12:34:56Z",
    "2026-07-01T12:34:56.789+08:00",
    "2026-07-01T12:34:56+14:00",
  ])(
    "accepts a complete offset-bearing postedAt timestamp: %s",
    async (postedAt) => {
      const fixture = createSyntheticTransferFixture();
      const record = fixture.proposal.records[0];
      if (!record) {
        throw new Error("synthetic fixture is missing its first record");
      }
      record.postedAt = postedAt;

      const result = await validateStructuredProposal({
        ...fixture,
        recordContract: syntheticBankRecordContract,
      });

      expect(result.status).toBe("valid");
    },
  );

  it("rejects a proposal-supplied provider record ID that is not grounded", async () => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    if (!record) {
      throw new Error("synthetic fixture is missing its first record");
    }
    record.providerRecordId = "hallucinated-provider-id";

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "provider_record_id_not_grounded",
        },
      ],
    });
  });

  it("rejects a grounded provider record ID reused by two records", async () => {
    const fixture = createSyntheticTransferFixture();
    fixture.proposal.records.forEach((record, index) => {
      record.providerRecordId = "provider-record-duplicate";
      record.raw.providerRecordId = "provider-record-duplicate";
      fixture.extractionBundle.observations.push({
        id: `provider-id-${index + 1}`,
        kind: "table_cell",
        row: index === 0 ? 2 : 5,
        column: 6,
        text: "provider-record-duplicate",
        engine: "synthetic-fixture",
        engineVersion: "1",
      });
    });

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [{ code: "duplicate_provider_record_id" }],
    });
  });

  it("returns a deterministic shape error when a raw row is missing a required field", async () => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    if (!record) {
      throw new Error("synthetic fixture is missing its first record");
    }
    delete record.raw.date;

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "raw_record_shape_invalid",
        },
      ],
    });
  });

  it("rejects extra full-document payload in a bounded raw row", async () => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    if (!record) {
      throw new Error("synthetic fixture is missing its first record");
    }
    record.raw.fullDocumentText = "not part of one source row";

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "raw_record_shape_invalid",
        },
      ],
    });
  });

  it("rejects an oversized value even when its field is allowed", async () => {
    const fixture = createSyntheticTransferFixture();
    const record = fixture.proposal.records[0];
    if (!record) {
      throw new Error("synthetic fixture is missing its first record");
    }
    record.raw.description = "x".repeat(16_385);

    const result = await validateStructuredProposal({
      ...fixture,
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [
        {
          proposalRecordId: "record-checking-out",
          code: "raw_record_shape_invalid",
        },
      ],
    });
  });
});

describe("semanticDocumentKey identity", () => {
  it("derives the host-canonical key with and without a provider root", async () => {
    expect(
      semanticDocumentKey({ providerKey: "synthetic-bank", documentType: "transfer_export" }),
    ).toBeUndefined();
    expect(
      semanticDocumentKey({
        providerKey: "synthetic-bank",
        documentType: "transfer_export",
        statementId: "transfer-2026-07",
      }),
    ).toBe("synthetic-bank:transfer-2026-07");
    expect(
      semanticDocumentKey({
        providerKey: "dbs",
        documentType: "bank_statement",
        providerRootId: "statements/2026",
        statementId: "dbs-bank_statement-2026-07",
      }),
    ).toBe("dbs:statements/2026:dbs-bank_statement-2026-07");
  });

  it("rejects an empty or overlong semantic document key", async () => {
    for (const candidate of ["", `synthetic-bank:${"x".repeat(256)}`]) {
      const fixture = createSyntheticTransferFixture();
      const result = await validateStructuredProposal({
        ...fixture,
        semanticDocumentKey: candidate,
        recordContract: syntheticBankRecordContract,
      });

      expect(result).toEqual({
        status: "invalid",
        errors: [{ code: "semantic_document_key_invalid" }],
      });
    }
  });

  it("rejects a well-formed key that does not match the canonical derivation", async () => {
    const fixture = createSyntheticTransferFixture();
    const result = await validateStructuredProposal({
      ...fixture,
      semanticDocumentKey: "synthetic-bank:other-statement",
      recordContract: syntheticBankRecordContract,
    });

    expect(result).toEqual({
      status: "invalid",
      errors: [{ code: "semantic_document_key_mismatch" }],
    });
  });

  it("hashes identity projections independent of key order with JSON undefined semantics", async () => {
    const contractFor = (projection: Record<string, unknown>) => ({
      inspect(raw: Record<string, unknown>) {
        const inspection = syntheticBankRecordContract.inspect(raw);
        return { ...inspection, identityProjection: projection };
      },
    });
    const fixture = createSyntheticTransferFixture();
    const probe = syntheticBankRecordContract.inspect(
      fixture.proposal.records[0]?.raw ?? {},
    ).identityProjection;
    const runs: Array<{
      semanticDocumentKey: string;
      extractionBundle: ExtractionBundle;
      proposal: StructuredParseProposal;
    }> = [fixture, createSyntheticTransferFixture(), createSyntheticTransferFixture()];
    const contracts = [
      contractFor({
        Z: "upper",
        ff: "letters",
        "ﬀ": "ligature",
        z: "lower",
        holes: [null, "kept"],
        ...probe,
      }),
      contractFor({
        z: "lower",
        "ﬀ": "ligature",
        ff: "letters",
        Z: "upper",
        maybe: undefined,
        holes: [null, "kept"],
        ...probe,
      }),
      contractFor({
        Z: "upper",
        ff: "letters",
        "ﬀ": "ligature",
        z: "lower",
        holes: [undefined, "kept"],
        ...probe,
      }),
    ];
    const keys: string[] = [];
    for (const [index, input] of runs.entries()) {
      const contract = contracts[index];
      if (!contract) {
        throw new Error("missing projection contract");
      }
      const result = await validateStructuredProposal({
        ...input,
        recordContract: contract,
      });
      if (result.status !== "valid") {
        throw new Error("synthetic fixture did not validate");
      }
      const key = result.records[0]?.stableRecordKey;
      if (!key) {
        throw new Error("synthetic fixture is missing its first record");
      }
      keys.push(key);
    }

    expect(keys[1]).toBe(keys[0]);
    expect(keys[2]).toBe(keys[0]);
  });
});
