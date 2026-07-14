import { describe, expect, it } from "vitest";

import { validateStructuredProposal } from "./index";
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
