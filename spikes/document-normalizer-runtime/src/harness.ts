import { createHash } from "node:crypto";

import {
  semanticDocumentKey,
  validateStructuredProposal,
  type ExtractionBundle,
  type SourceObservation,
  type StructuredParseProposal,
} from "@cancan/parsers";
import {
  createSyntheticTransferFixture,
  syntheticBankRecordContract,
} from "@cancan/parsers/testing";
import { z } from "zod";

export const toolNames = [
  "inspect_input",
  "extract_native_text",
  "extract_table",
  "ocr_pages",
  "read_page_region",
  "validate_proposal",
  "submit_structured_proposal",
] as const;

export type ToolName = (typeof toolNames)[number];
export type RuntimeName = "single-pass" | "tool-loop-agent" | "pi-agent-core";

const MAX_TOOL_OBSERVATIONS = 200;
const MAX_TOOL_TEXT_CHARACTERS = 4_096;
const MAX_VALIDATION_ERRORS = 20;

const exactMoneySchema = z
  .object({ value: z.string(), currency: z.string() })
  .strict();

const recordSchema = z
  .object({
    proposalRecordId: z.string(),
    providerRecordId: z.string().optional(),
    recordType: z.enum([
      "transaction",
      "balance",
      "position",
      "trade",
      "valuation",
      "fee",
      "interest",
    ]),
    eventType: z.string().optional(),
    proposalAccountId: z.string().optional(),
    instrumentSymbol: z.string().optional(),
    postedOn: z.string().optional(),
    transactionOn: z.string().optional(),
    postedAt: z.string().optional(),
    descriptionRaw: z.string().optional(),
    descriptionNormalized: z.string().optional(),
    amount: exactMoneySchema.optional(),
    quantity: z.string().optional(),
    statementEntrySide: z.enum(["debit", "credit"]).optional(),
    accountBalanceDelta: exactMoneySchema.optional(),
    balanceAfter: exactMoneySchema.optional(),
    valuation: exactMoneySchema.optional(),
    raw: z.record(z.string(), z.unknown()),
  })
  .strict();

export const proposalSchema = z
  .object({
    document: z
      .object({
        providerKey: z.string(),
        documentType: z.string(),
        statementId: z.string().optional(),
        statementPeriod: z
          .object({ from: z.string().optional(), to: z.string().optional() })
          .strict()
          .optional(),
      })
      .strict(),
    accounts: z.array(
      z
        .object({
          proposalAccountId: z.string(),
          accountType: z.enum([
            "deposit_account",
            "credit_card",
            "currency_balance",
            "brokerage_account",
            "cash_balance",
            "position_group",
            "insurance_policy",
            "manual_asset",
            "manual_liability",
          ]),
          providerAccountId: z.string().optional(),
          maskedIdentifier: z.string().optional(),
          currency: z.string().optional(),
        })
        .strict(),
    ),
    openingSnapshots: z.array(recordSchema),
    records: z.array(recordSchema),
    closingSnapshots: z.array(recordSchema),
  })
  .strict();

const regionSchema = z
  .object({
    x: z.number().min(0).max(1),
    y: z.number().min(0).max(1),
    width: z.number().positive().max(1),
    height: z.number().positive().max(1),
  })
  .strict();

const sourceObservationSchema = z
  .object({
    id: z.string(),
    kind: z.enum(["native_text", "ocr_text", "table_cell", "document_region"]),
    page: z.number().int().positive().optional(),
    row: z.number().int().positive().optional(),
    column: z.number().int().positive().optional(),
    text: z.string().max(MAX_TOOL_TEXT_CHARACTERS),
    textSpan: z
      .object({ start: z.number().int().nonnegative(), end: z.number().int().nonnegative() })
      .strict()
      .optional(),
    boundingBox: regionSchema.optional(),
    engine: z.string(),
    engineVersion: z.string(),
    confidence: z.number().min(0).max(1).optional(),
  })
  .strict();

export const toolInputSchemas = {
  inspect_input: z.object({}).strict(),
  extract_native_text: z
    .object({ maxCharacters: z.number().int().positive().max(4_096) })
    .strict(),
  extract_table: z.object({ maxRows: z.number().int().positive().max(100) }).strict(),
  ocr_pages: z
    .object({ pages: z.array(z.number().int().positive()).min(1).max(4) })
    .strict(),
  read_page_region: z
    .object({ page: z.number().int().positive(), region: regionSchema })
    .strict(),
  validate_proposal: z.object({ proposal: proposalSchema }).strict(),
  submit_structured_proposal: z.object({ proposal: proposalSchema }).strict(),
} satisfies Record<ToolName, z.ZodType>;

const rejectedToolResultSchema = z
  .object({
    status: z.literal("rejected"),
    code: z.string(),
    errors: z.array(z.string()).max(MAX_VALIDATION_ERRORS).optional(),
  })
  .strict();
const observationResultSchema = z.union([
  z
    .object({
      status: z.literal("ok"),
      data: z.array(sourceObservationSchema).max(MAX_TOOL_OBSERVATIONS),
    })
    .strict(),
  rejectedToolResultSchema,
]);

export const toolOutputSchemas = {
  inspect_input: z.union([
    z
      .object({
        status: z.literal("ok"),
        data: z
          .object({
            sourceDocumentId: z.string(),
            mimeType: z.string(),
            observationCount: z.number().int().nonnegative(),
          })
          .strict(),
      })
      .strict(),
    rejectedToolResultSchema,
  ]),
  extract_native_text: observationResultSchema,
  extract_table: observationResultSchema,
  ocr_pages: observationResultSchema,
  read_page_region: observationResultSchema,
  validate_proposal: z.union([
    z
      .object({
        status: z.literal("ok"),
        data: z.object({ validation: z.literal("passed") }).strict(),
      })
      .strict(),
    rejectedToolResultSchema,
  ]),
  submit_structured_proposal: z.union([
    z
      .object({
        status: z.literal("accepted"),
        recordCount: z.number().int().nonnegative(),
      })
      .strict(),
    rejectedToolResultSchema,
  ]),
} satisfies Record<ToolName, z.ZodType>;

export type RuntimeFixture = ReturnType<typeof createSyntheticTransferFixture>;

export interface RuntimeResult {
  runtime: RuntimeName;
  outcome: "accepted" | "rejected" | "cancelled";
  code?: string;
  modelSteps: number;
  toolCalls: ToolName[];
  submissions: number;
  proposalSha256?: string;
  recordCount?: number;
}

type ToolResult =
  | { status: "ok"; data: unknown }
  | { status: "accepted"; recordCount: number }
  | { status: "rejected"; code: string; errors?: string[] };

function sourceConflict(bundle: ExtractionBundle): boolean {
  const values = new Map<string, Set<string>>();
  for (const observation of bundle.observations) {
    if (
      observation.page === undefined ||
      observation.row === undefined ||
      observation.column === undefined ||
      (observation.kind !== "native_text" && observation.kind !== "ocr_text")
    ) {
      continue;
    }
    const key = `${observation.page}:${observation.row}:${observation.column}`;
    const existing = values.get(key) ?? new Set<string>();
    existing.add(observation.text.trim().replace(/\s+/g, " "));
    values.set(key, existing);
  }
  return [...values.values()].some((group) => group.size > 1);
}

function proposalHash(proposal: StructuredParseProposal): string {
  return createHash("sha256").update(JSON.stringify(proposal)).digest("hex");
}

function boundedObservations(
  observations: SourceObservation[],
  maxCharacters: number,
): SourceObservation[] {
  const result: SourceObservation[] = [];
  let remaining = maxCharacters;
  for (const observation of observations) {
    if (result.length === MAX_TOOL_OBSERVATIONS || remaining === 0) {
      break;
    }
    const text = observation.text.slice(0, remaining);
    if (text.length > 0) {
      result.push({ ...observation, text });
      remaining -= text.length;
    }
  }
  return result;
}

function firstRows(observations: SourceObservation[], maxRows: number): SourceObservation[] {
  const rows = new Set<number>();
  return observations
    .filter((observation) => {
      if (observation.kind !== "table_cell" || observation.row === undefined) {
        return false;
      }
      if (!rows.has(observation.row) && rows.size === maxRows) {
        return false;
      }
      rows.add(observation.row);
      return true;
    })
    .slice(0, MAX_TOOL_OBSERVATIONS);
}

function intersects(
  left: NonNullable<SourceObservation["boundingBox"]>,
  right: z.infer<typeof regionSchema>,
): boolean {
  return (
    left.x < right.x + right.width &&
    left.x + left.width > right.x &&
    left.y < right.y + right.height &&
    left.y + left.height > right.y
  );
}

export function createRuntimeFixture(): RuntimeFixture {
  return createSyntheticTransferFixture();
}

export class RuntimeSession {
  readonly fixture: RuntimeFixture;
  readonly toolCalls: ToolName[] = [];
  submissions = 0;
  acceptedProposal?: StructuredParseProposal;
  lastErrorCode?: string;

  constructor(fixture: RuntimeFixture = createRuntimeFixture()) {
    this.fixture = fixture;
  }

  reject(code: string): void {
    this.lastErrorCode = code;
  }

  private output(name: ToolName, result: ToolResult): ToolResult {
    const parsed = toolOutputSchemas[name].safeParse(result);
    if (!parsed.success) {
      this.reject("tool_output_invalid");
      return { status: "rejected", code: "tool_output_invalid" };
    }
    return parsed.data as ToolResult;
  }

  private async checkProposal(proposal: unknown): Promise<
    | { status: "valid"; proposal: StructuredParseProposal }
    | { status: "invalid"; code: string; errors: string[] }
  > {
    const parsed = proposalSchema.safeParse(proposal);
    if (!parsed.success) {
      return { status: "invalid", code: "structured_proposal_invalid", errors: ["schema_invalid"] };
    }
    if (sourceConflict(this.fixture.extractionBundle)) {
      return {
        status: "invalid",
        code: "evidence_grounding_failed",
        errors: ["native_ocr_conflict"],
      };
    }
    const proposalValue = parsed.data as StructuredParseProposal;
    const key = semanticDocumentKey(proposalValue.document);
    if (key === undefined) {
      return {
        status: "invalid",
        code: "evidence_grounding_failed",
        errors: ["statement_id_not_grounded"],
      };
    }
    const validation = await validateStructuredProposal({
      semanticDocumentKey: key,
      extractionBundle: this.fixture.extractionBundle,
      proposal: proposalValue,
      recordContract: syntheticBankRecordContract,
    });
    if (validation.status === "invalid") {
      return {
        status: "invalid",
        code: "evidence_grounding_failed",
        errors: validation.errors.map(({ code }) => code),
      };
    }
    return { status: "valid", proposal: proposalValue };
  }

  async submitSinglePass(proposal: unknown): Promise<void> {
    this.submissions += 1;
    const checked = await this.checkProposal(proposal);
    if (checked.status === "invalid") {
      this.reject(checked.code);
      return;
    }
    this.acceptedProposal = checked.proposal;
  }

  async execute(name: ToolName, input: unknown): Promise<ToolResult> {
    const parsed = toolInputSchemas[name].safeParse(input);
    if (!parsed.success) {
      this.reject("tool_input_invalid");
      return this.output(name, { status: "rejected", code: "tool_input_invalid" });
    }
    this.toolCalls.push(name);

    switch (name) {
      case "inspect_input":
        return this.output(name, {
          status: "ok",
          data: {
            sourceDocumentId: this.fixture.extractionBundle.sourceDocumentId,
            mimeType: this.fixture.extractionBundle.mimeType,
            observationCount: this.fixture.extractionBundle.observations.length,
          },
        });
      case "extract_native_text": {
        const { maxCharacters } = parsed.data as { maxCharacters: number };
        return this.output(name, {
          status: "ok",
          data: boundedObservations(
            this.fixture.extractionBundle.observations.filter(
              ({ kind }) => kind === "native_text",
            ),
            maxCharacters,
          ),
        });
      }
      case "extract_table": {
        const { maxRows } = parsed.data as { maxRows: number };
        return this.output(name, {
          status: "ok",
          data: boundedObservations(
            firstRows(this.fixture.extractionBundle.observations, maxRows),
            MAX_TOOL_TEXT_CHARACTERS,
          ),
        });
      }
      case "ocr_pages": {
        const pages = new Set((parsed.data as { pages: number[] }).pages);
        return this.output(name, {
          status: "ok",
          data: boundedObservations(
            this.fixture.extractionBundle.observations.filter(
              ({ kind, page }) => kind === "ocr_text" && page !== undefined && pages.has(page),
            ),
            MAX_TOOL_TEXT_CHARACTERS,
          ),
        });
      }
      case "read_page_region": {
        const { page, region } = parsed.data as {
          page: number;
          region: z.infer<typeof regionSchema>;
        };
        return this.output(name, {
          status: "ok",
          data: boundedObservations(
            this.fixture.extractionBundle.observations.filter(
              (observation) =>
                observation.page === page &&
                observation.boundingBox !== undefined &&
                intersects(observation.boundingBox, region),
            ),
            MAX_TOOL_TEXT_CHARACTERS,
          ),
        });
      }
      case "validate_proposal": {
        const checked = await this.checkProposal(
          (parsed.data as { proposal: StructuredParseProposal }).proposal,
        );
        if (checked.status === "invalid") {
          this.reject(checked.code);
          return this.output(name, {
            status: "rejected",
            code: checked.code,
            errors: checked.errors.slice(0, MAX_VALIDATION_ERRORS),
          });
        }
        return this.output(name, { status: "ok", data: { validation: "passed" } });
      }
      case "submit_structured_proposal": {
        this.submissions += 1;
        if (this.submissions > 2) {
          this.reject("submission_budget_exhausted");
          return this.output(name, {
            status: "rejected",
            code: "submission_budget_exhausted",
          });
        }
        const checked = await this.checkProposal(
          (parsed.data as { proposal: StructuredParseProposal }).proposal,
        );
        if (checked.status === "invalid") {
          this.reject(checked.code);
          return this.output(name, {
            status: "rejected",
            code: checked.code,
            errors: checked.errors.slice(0, MAX_VALIDATION_ERRORS),
          });
        }
        this.acceptedProposal = checked.proposal;
        this.lastErrorCode = undefined;
        return this.output(name, {
          status: "accepted",
          recordCount:
            checked.proposal.openingSnapshots.length +
            checked.proposal.records.length +
            checked.proposal.closingSnapshots.length,
        });
      }
    }
  }

  result(runtime: RuntimeName, modelSteps: number, cancelled = false): RuntimeResult {
    if (cancelled) {
      return {
        runtime,
        outcome: "cancelled",
        code: "cancelled",
        modelSteps,
        toolCalls: [...this.toolCalls],
        submissions: this.submissions,
      };
    }
    if (!this.acceptedProposal) {
      return {
        runtime,
        outcome: "rejected",
        code: this.lastErrorCode ?? "free_text_completion",
        modelSteps,
        toolCalls: [...this.toolCalls],
        submissions: this.submissions,
      };
    }
    return {
      runtime,
      outcome: "accepted",
      modelSteps,
      toolCalls: [...this.toolCalls],
      submissions: this.submissions,
      proposalSha256: proposalHash(this.acceptedProposal),
      recordCount:
        this.acceptedProposal.openingSnapshots.length +
        this.acceptedProposal.records.length +
        this.acceptedProposal.closingSnapshots.length,
    };
  }
}
