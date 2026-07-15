import { describe, expect, it } from "vitest";
import { z } from "zod";

import {
  RuntimeSession,
  createRuntimeFixture,
  toolInputSchemas,
  toolNames,
  toolOutputSchemas,
  type RuntimeFixture,
} from "./harness";
import {
  happyPathTranscript,
  piToolParameters,
  runPiAgentCore,
  runSinglePass,
  runToolLoopAgent,
  type TranscriptEntry,
} from "./runtimes";

function invalidProposal() {
  const proposal = structuredClone(createRuntimeFixture().proposal);
  const record = proposal.records[0];
  if (record?.amount) {
    record.amount.value = "999.00";
  }
  return proposal;
}

function observationTexts(result: unknown): string[] {
  if (
    typeof result !== "object" ||
    result === null ||
    !("status" in result) ||
    result.status !== "ok" ||
    !("data" in result) ||
    !Array.isArray(result.data)
  ) {
    return [];
  }
  return result.data.flatMap((value) =>
    typeof value === "object" &&
    value !== null &&
    "text" in value &&
    typeof value.text === "string"
      ? [value.text]
      : [],
  );
}

function conflictingFixture(): RuntimeFixture {
  const fixture = createRuntimeFixture();
  fixture.extractionBundle.observations.push(
    {
      id: "native-conflict",
      kind: "native_text",
      page: 1,
      row: 1,
      column: 1,
      text: "250.00",
      engine: "native",
      engineVersion: "1",
    },
    {
      id: "ocr-conflict",
      kind: "ocr_text",
      page: 1,
      row: 1,
      column: 1,
      text: "280.00",
      engine: "ocr",
      engineVersion: "1",
    },
  );
  return fixture;
}

describe("document normalizer runtime evidence", () => {
  it("exposes exactly the seven job-scoped tools with bounded strict inputs", () => {
    expect(toolNames).toEqual([
      "inspect_input",
      "extract_native_text",
      "extract_table",
      "ocr_pages",
      "read_page_region",
      "validate_proposal",
      "submit_structured_proposal",
    ]);
    expect(
      toolInputSchemas.read_page_region.safeParse({
        page: 1,
        region: { x: 0, y: 0, width: 1, height: 1 },
        path: "/tmp/other-document.pdf",
      }).success,
    ).toBe(false);
    expect(toolInputSchemas.ocr_pages.safeParse({ pages: [1, 2, 3, 4, 5] }).success).toBe(
      false,
    );
    for (const name of toolNames) {
      expect(piToolParameters[name]).toEqual(
        z.toJSONSchema(toolInputSchemas[name], { target: "draft-7" }),
      );
    }
    const submitSchema = piToolParameters.submit_structured_proposal as unknown as {
      properties?: { proposal?: unknown };
    };
    expect(submitSchema.properties?.proposal).toMatchObject({ type: "object" });
  });

  it("runtime-validates and caps tool results, including page-region intersection", async () => {
    const fixture = createRuntimeFixture();
    fixture.extractionBundle.observations = Array.from({ length: 205 }, (_, index) => ({
      id: `ocr-${index}`,
      kind: "ocr_text" as const,
      page: 1,
      text: "abcd",
      boundingBox: { x: 0.1, y: 0.1, width: 0.1, height: 0.1 },
      engine: "mock",
      engineVersion: "1",
    }));
    fixture.extractionBundle.observations.push({
      id: "outside",
      kind: "document_region",
      page: 1,
      text: "outside",
      boundingBox: { x: 0.8, y: 0.8, width: 0.1, height: 0.1 },
      engine: "mock",
      engineVersion: "1",
    });
    const session = new RuntimeSession(fixture);
    const ocr = await session.execute("ocr_pages", { pages: [1] });
    const region = await session.execute("read_page_region", {
      page: 1,
      region: { x: 0, y: 0, width: 0.5, height: 0.5 },
    });
    fixture.extractionBundle.observations = [
      {
        id: "oversized-ocr",
        kind: "ocr_text",
        page: 1,
        text: "x".repeat(5_000),
        engine: "mock",
        engineVersion: "1",
      },
    ];
    const oversizedOcr = await session.execute("ocr_pages", { pages: [1] });
    fixture.extractionBundle.observations = [
      {
        id: "oversized-region",
        kind: "document_region",
        page: 1,
        text: "x".repeat(5_000),
        boundingBox: { x: 0.1, y: 0.1, width: 0.1, height: 0.1 },
        engine: "mock",
        engineVersion: "1",
      },
    ];
    const oversizedRegion = await session.execute("read_page_region", {
      page: 1,
      region: { x: 0, y: 0, width: 0.5, height: 0.5 },
    });
    fixture.extractionBundle.observations = Array.from({ length: 205 }, (_, index) => ({
      id: `native-${index}`,
      kind: "native_text" as const,
      text: "abcd",
      engine: "mock",
      engineVersion: "1",
    }));
    const native = await session.execute("extract_native_text", { maxCharacters: 3 });
    fixture.extractionBundle.observations = Array.from({ length: 205 }, (_, index) => ({
      id: `cell-${index}`,
      kind: "table_cell" as const,
      row: index + 1,
      column: 1,
      text: "cell",
      engine: "mock",
      engineVersion: "1",
    }));
    const table = await session.execute("extract_table", { maxRows: 2 });
    fixture.extractionBundle.observations = [
      {
        id: "oversized-cell",
        kind: "table_cell",
        row: 1,
        column: 1,
        text: "x".repeat(5_000),
        engine: "mock",
        engineVersion: "1",
      },
    ];
    const oversizedTable = await session.execute("extract_table", { maxRows: 1 });

    expect(toolOutputSchemas.ocr_pages.safeParse(ocr).success).toBe(true);
    expect(observationTexts(ocr)).toHaveLength(200);
    expect(toolOutputSchemas.read_page_region.safeParse(region).success).toBe(true);
    expect(
      region.status === "ok" && Array.isArray(region.data)
        ? region.data.map(({ id }) => id)
        : [],
    ).not.toContain("outside");
    expect(
      native.status === "ok" && Array.isArray(native.data)
        ? native.data.reduce((total, { text }) => total + text.length, 0)
        : Number.POSITIVE_INFINITY,
    ).toBe(3);
    expect(table.status === "ok" && Array.isArray(table.data) ? table.data : []).toHaveLength(2);
    for (const result of [oversizedOcr, oversizedRegion, oversizedTable]) {
      const texts = observationTexts(result);
      expect(texts.reduce((total, text) => total + text.length, 0)).toBeLessThanOrEqual(4_096);
      expect(texts.every((text) => text.length <= 4_096)).toBe(true);
    }
    expect(
      toolOutputSchemas.ocr_pages.safeParse({
        status: "ok",
        data: Array.from({ length: 201 }, () => fixture.extractionBundle.observations[0]),
      }).success,
    ).toBe(false);
  });

  it("runs both frameworks and single-pass through one proposal and validator", async () => {
    const fixture = createRuntimeFixture();
    const transcript = happyPathTranscript(fixture.proposal);
    const [singlePass, toolLoop, pi] = await Promise.all([
      runSinglePass({ proposal: fixture.proposal }),
      runToolLoopAgent({ transcript }),
      runPiAgentCore({ transcript }),
    ]);

    expect(singlePass.outcome).toBe("accepted");
    expect(toolLoop.outcome).toBe("accepted");
    expect(pi.outcome).toBe("accepted");
    expect(new Set([singlePass.proposalSha256, toolLoop.proposalSha256, pi.proposalSha256]).size).toBe(
      1,
    );
    expect(singlePass.modelSteps).toBe(1);
    expect(toolLoop.toolCalls).toEqual([
      "inspect_input",
      "extract_table",
      "validate_proposal",
      "submit_structured_proposal",
    ]);
    expect(pi.toolCalls).toEqual(toolLoop.toolCalls);
  });

  it("rejects malformed canonical tool inputs equivalently in both frameworks", async () => {
    const transcript: TranscriptEntry[] = [
      { toolName: "ocr_pages", input: { pages: [1.5] } },
      { text: "done" },
    ];
    const [toolLoop, pi] = await Promise.all([
      runToolLoopAgent({ transcript }),
      runPiAgentCore({ transcript }),
    ]);

    expect(toolLoop.outcome).toBe("rejected");
    expect(pi.outcome).toBe("rejected");
    expect(toolLoop.toolCalls).toEqual([]);
    expect(pi.toolCalls).toEqual([]);
    expect(pi.code).toBe(toolLoop.code);
  });

  it("revalidates every submission and enforces the two-submission budget", async () => {
    const proposal = invalidProposal();
    const transcript: TranscriptEntry[] = [
      { toolName: "submit_structured_proposal", input: { proposal } },
      { toolName: "submit_structured_proposal", input: { proposal } },
      { toolName: "submit_structured_proposal", input: { proposal } },
      { text: "accept it anyway" },
    ];
    const [toolLoop, pi] = await Promise.all([
      runToolLoopAgent({ transcript }),
      runPiAgentCore({ transcript }),
    ]);
    expect(toolLoop.outcome).toBe("rejected");
    expect(toolLoop.code).toBe("submission_budget_exhausted");
    expect(pi.outcome).toBe("rejected");
    expect(pi.code).toBe("submission_budget_exhausted");
  });

  it("rejects free text and forbidden capability attempts without echoing secrets", async () => {
    const secret = "cancan-evidence-secret-value";
    const freeText: TranscriptEntry[] = [{ text: "{\"records\":[]}" }];
    const forbidden: TranscriptEntry[] = [
      { toolName: "read_secret", input: { name: secret } },
      { text: "done" },
    ];
    const results = await Promise.all([
      runToolLoopAgent({ transcript: freeText }),
      runPiAgentCore({ transcript: freeText }),
      runToolLoopAgent({ transcript: forbidden }),
      runPiAgentCore({ transcript: forbidden }),
    ]);

    expect(results[0]?.code).toBe("free_text_completion");
    expect(results[1]?.code).toBe("free_text_completion");
    expect(results[2]?.code).toBe("forbidden_tool");
    expect(results[3]?.code).toBe("forbidden_tool");
    expect(JSON.stringify(results)).not.toContain(secret);
    expect(results.flatMap(({ toolCalls }) => toolCalls)).not.toContain("read_secret");
  });

  it("turns source conflicts and ungrounded values into deterministic rejection", async () => {
    const conflict = conflictingFixture();
    const conflictResult = await runSinglePass({
      proposal: conflict.proposal,
      fixture: conflict,
    });
    const groundingResult = await runSinglePass({ proposal: invalidProposal() });
    expect(conflictResult).toMatchObject({
      outcome: "rejected",
      code: "evidence_grounding_failed",
    });
    expect(groundingResult).toMatchObject({
      outcome: "rejected",
      code: "evidence_grounding_failed",
    });
  });

  it("enforces eight model steps and supports cancellation in both frameworks", async () => {
    const budgetTranscript = Array.from({ length: 9 }, () => ({
      toolName: "inspect_input",
      input: {},
    }));
    const toolLoopAbort = new AbortController();
    const piAbort = new AbortController();
    setTimeout(() => toolLoopAbort.abort(), 10);
    setTimeout(() => piAbort.abort(), 10);

    const [toolLoopBudget, piBudget, toolLoopCancelled, piCancelled] = await Promise.all([
      runToolLoopAgent({ transcript: budgetTranscript }),
      runPiAgentCore({ transcript: budgetTranscript }),
      runToolLoopAgent({
        transcript: happyPathTranscript(createRuntimeFixture().proposal),
        signal: toolLoopAbort.signal,
        delayMs: 100,
      }),
      runPiAgentCore({
        transcript: happyPathTranscript(createRuntimeFixture().proposal),
        signal: piAbort.signal,
        delayMs: 100,
      }),
    ]);

    expect(toolLoopBudget).toMatchObject({
      outcome: "rejected",
      code: "normalizer_budget_exhausted",
      modelSteps: 8,
    });
    expect(piBudget).toMatchObject({
      outcome: "rejected",
      code: "normalizer_budget_exhausted",
      modelSteps: 8,
    });
    expect(toolLoopCancelled.outcome).toBe("cancelled");
    expect(piCancelled.outcome).toBe("cancelled");
  });
});
