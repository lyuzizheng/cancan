import { describe, expect, it } from "vitest";
import { selectProviderDocumentPackage } from "@cancan/parsers";
import {
  createSyntheticProviderStatementFixture,
  createSyntheticTransferFixture,
} from "@cancan/parsers/testing";

import { normalizeWithMock } from "./mock-normalizer";

function fixtureInput() {
  const fixture = createSyntheticTransferFixture();
  fixture.extractionBundle.sourceDocumentId = "document-fixture";
  fixture.extractionBundle.observations.push(
    {
      id: "csv-row-1-column-6",
      kind: "table_cell",
      row: 1,
      column: 6,
      text: "CANCAN_SYNTHETIC_STATEMENT_V1",
      engine: "rust-csv",
      engineVersion: "1.4.0",
    },
    {
      id: "csv-row-2-column-6",
      kind: "table_cell",
      row: 2,
      column: 6,
      text: "provider=synthetic-bank",
      engine: "rust-csv",
      engineVersion: "1.4.0",
    },
    {
      id: "csv-row-3-column-6",
      kind: "table_cell",
      row: 3,
      column: 6,
      text: "statement_id=transfer-2026-07",
      engine: "rust-csv",
      engineVersion: "1.4.0",
    },
  );
  fixture.extractionBundle.metadata.observationCount = fixture.extractionBundle.observations.length;
  return {
    documentId: "document-fixture",
    extractionBundle: fixture.extractionBundle,
  };
}

function providerFixtureInput(providerKey: string, documentType: string) {
  const providerPackage = selectProviderDocumentPackage({
    providerKey,
    documentType,
    mimeType: "application/pdf",
  });
  if (!providerPackage) {
    throw new Error(`missing provider fixture package ${providerKey}/${documentType}`);
  }
  const fixture = createSyntheticProviderStatementFixture(providerPackage);
  const documentId = `document-${providerPackage.packageId}`;
  fixture.extractionBundle.sourceDocumentId = documentId;
  return { documentId, extractionBundle: fixture.extractionBundle };
}

function providerPdfFixtureInput(providerKey: string, documentType: string) {
  const input = providerFixtureInput(providerKey, documentType);
  const rows = new Map<number, string[]>();
  const otherObservations: Array<{ id: string; text: string }> = [];

  for (const observation of input.extractionBundle.observations) {
    if (observation.kind === "table_cell" && observation.row !== undefined) {
      const line = rows.get(observation.row) ?? [];
      line.push(observation.text);
      rows.set(observation.row, line);
    } else {
      otherObservations.push({ id: observation.id, text: observation.text });
    }
  }

  const pdfObservations: Array<{
    id: string;
    kind: "native_text";
    page: number;
    row: number;
    textSpan: { start: 0; end: number };
    text: string;
    engine: string;
    engineVersion: string;
  }> = [];

  for (const [row, cellTexts] of rows.entries()) {
    const text = cellTexts.join(" ");
    pdfObservations.push({
      id: `pdf-page-1-native-text-${row}`,
      kind: "native_text",
      page: 1,
      row,
      textSpan: { start: 0, end: text.length },
      text,
      engine: "pdfkit",
      engineVersion: "macos-page-string-v2",
    });
  }

  let nextRow = rows.size + 1;
  for (const other of otherObservations) {
    pdfObservations.push({
      id: `pdf-page-1-native-text-${nextRow}`,
      kind: "native_text",
      page: 1,
      row: nextRow,
      textSpan: { start: 0, end: other.text.length },
      text: other.text,
      engine: "pdfkit",
      engineVersion: "macos-page-string-v2",
    });
    nextRow++;
  }

  input.extractionBundle.observations = pdfObservations;
  input.extractionBundle.metadata = {
    extractionVersion: "native-observations-v2",
    observationCount: pdfObservations.length,
  };
  return input;
}

describe("mock document normalizer", () => {
  it("returns the validated full synthetic proposal", async () => {
    const result = await normalizeWithMock(fixtureInput());

    expect(result.status).toBe("classified");
    if (result.status !== "classified") {
      throw new Error("expected the validated synthetic proposal");
    }
    expect(result.proposal.status).toBe("valid");
    expect(result.proposal.accounts).toHaveLength(2);
    expect(result.proposal.openingSnapshots).toHaveLength(2);
    expect(result.proposal.records).toHaveLength(2);
    expect(result.proposal.closingSnapshots).toHaveLength(2);
    expect(result.profile).toMatchObject({
      id: "synthetic-bank-transfer-export-v1",
      providerKey: "synthetic-bank",
      documentType: "transfer_export",
      packageId: "synthetic/bank_transfer_export@1",
      normalizerRuntime: "single-pass-mock",
      inputStrategy: "native-observations-v1",
      modelProvider: "cancan-deterministic-mock",
      model: "fixture-v1",
      reviewOnly: true,
      extractionEngines: expect.arrayContaining([
        { kind: "table_cell", engine: "rust-csv", version: "1.4.0" },
        { kind: "table_cell", engine: "synthetic-fixture", version: "1" },
      ]),
      ocrEngines: [],
    });
    expect(
      [
        ...result.proposal.openingSnapshots,
        ...result.proposal.records,
        ...result.proposal.closingSnapshots,
      ].every(
        (record) =>
          record.stableRecordKey.length > 0 &&
          record.validation.schemaValid &&
          record.validation.rawGrounded &&
          record.validation.deterministicValidationPassed,
      ),
    ).toBe(true);
  });

  it.each([
    {
      providerKey: "dbs",
      documentType: "bank_statement",
      packageId: "dbs/bank_statement@1",
      eventType: undefined,
    },
    {
      providerKey: "dbs",
      documentType: "credit_card_statement",
      packageId: "dbs/credit_card_statement@1",
      eventType: "credit_card_repayment",
    },
    {
      providerKey: "hsbc",
      documentType: "bank_statement",
      packageId: "hsbc/bank_statement@1",
      eventType: "credit_card_repayment",
    },
  ])(
    "validates the $packageId synthetic provider fixture through its real package",
    async ({ providerKey, documentType, packageId, eventType }) => {
      const result = await normalizeWithMock(providerFixtureInput(providerKey, documentType));

      expect(result.status).toBe("classified");
      if (result.status !== "classified") {
        throw new Error("expected the provider fixture to classify");
      }
      expect(result.proposal.document).toMatchObject({ providerKey, documentType });
      expect(result.proposal.records[0]?.eventType).toBe(eventType);
      expect(result.profile).toMatchObject({
        providerKey,
        documentType,
        packageId,
        packageVersion: "1.0.0",
        parserVersion: "1.0.0",
        skillVersion: "1.0.0",
        promptVersion: "1.0.0",
        schemaVersion: "1.0.0",
        validatorVersion: "1.0.0",
        normalizerRuntime: "single-pass-mock",
        toolContractVersion: "1.0.0",
        inputStrategy: "native-observations-v1",
        modelProvider: "cancan-deterministic-mock",
        model: "fixture-v1",
        reviewOnly: true,
        extractionEngines: expect.arrayContaining([
          { kind: "native_text", engine: "synthetic-provider-fixture", version: "1" },
          { kind: "table_cell", engine: "synthetic-provider-fixture", version: "1" },
        ]),
        ocrEngines: [],
      });
      expect(result.profile.id).toBe(
        `mock:${packageId}:native-observations-v1:extract-native_text-synthetic-provider-fixture-1+extract-table_cell-synthetic-provider-fixture-1`,
      );
    },
  );

  it.each([
    {
      providerKey: "dbs",
      documentType: "bank_statement",
      packageId: "dbs/bank_statement@1",
    },
    {
      providerKey: "dbs",
      documentType: "credit_card_statement",
      packageId: "dbs/credit_card_statement@1",
    },
    {
      providerKey: "hsbc",
      documentType: "bank_statement",
      packageId: "hsbc/bank_statement@1",
    },
  ])(
    "validates a production-shaped $packageId PDF bundle",
    async ({ providerKey, documentType, packageId }) => {
      const result = await normalizeWithMock(providerPdfFixtureInput(providerKey, documentType));

      expect(result.status).toBe("classified");
      if (result.status !== "classified") {
        throw new Error("expected the native PDF fixture to classify");
      }
      expect(result.profile).toMatchObject({
        id: `mock:${packageId}:native-observations-v1:extract-native_text-pdfkit-macos-page-string-v2`,
        extractionEngines: [
          { kind: "native_text", engine: "pdfkit", version: "macos-page-string-v2" },
        ],
        ocrEngines: [],
      });
    },
  );

  it("returns needs attention when a provider fixture row is mutated", async () => {
    const input = providerFixtureInput("dbs", "bank_statement");
    const postingAmount = input.extractionBundle.observations.find(
      ({ row, text }) => row === 2 && text === "20.00",
    );
    if (!postingAmount) {
      throw new Error("missing synthetic provider posting amount");
    }
    postingAmount.text = "21.00";

    expect(await normalizeWithMock(input)).toEqual({
      status: "needs_attention",
      reason: "unsupported_document",
    });
  });

  it("returns needs attention when a provider marker names an unsupported package", async () => {
    const input = providerFixtureInput("dbs", "bank_statement");
    const marker = input.extractionBundle.observations.find(({ id }) => id === "fixture-marker");
    if (!marker) {
      throw new Error("missing synthetic provider marker");
    }
    marker.text = marker.text.replace("package_id=dbs/bank_statement@1", "package_id=unknown@1");

    expect(await normalizeWithMock(input)).toEqual({
      status: "needs_attention",
      reason: "unsupported_document",
    });
  });

  it("does not guess for unsupported evidence", async () => {
    expect(
      await normalizeWithMock({
        documentId: "document-unknown",
        extractionBundle: {
          sourceDocumentId: "document-unknown",
          fileSha256: "b".repeat(64),
          mimeType: "application/pdf",
          metadata: { extractionVersion: "native-observations-v2", observationCount: 1 },
          observations: [
            {
              id: "pdf-page-1-native-text-1",
              kind: "native_text",
              page: 1,
              row: 1,
              textSpan: { start: 0, end: 21 },
              text: "unknown provider text",
              engine: "pdfkit",
              engineVersion: "macos-page-string-v2",
            },
          ],
        },
      }),
    ).toEqual({ status: "needs_attention", reason: "unsupported_document" });
  });
});
