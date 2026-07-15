import { Agent, type AgentTool } from "@earendil-works/pi-agent-core";
import {
  createAssistantMessageEventStream,
  type AssistantMessage,
  type Model,
} from "@earendil-works/pi-ai";
import { ToolLoopAgent, generateObject, stepCountIs, tool } from "ai";
import { MockLanguageModelV3 } from "ai/test";
import { z } from "zod";

import {
  RuntimeSession,
  proposalSchema,
  toolInputSchemas,
  toolNames,
  type RuntimeFixture,
  type RuntimeResult,
  type ToolName,
} from "./harness";

export type TranscriptEntry =
  | { toolName: string; input: unknown }
  | { text: string };

const usage = {
  inputTokens: { total: 10, noCache: 10, cacheRead: undefined, cacheWrite: undefined },
  outputTokens: { total: 5, text: 5, reasoning: undefined },
};

function isAbort(error: unknown): boolean {
  return error instanceof Error && error.name === "AbortError";
}

function delay(ms: number, signal?: AbortSignal): Promise<void> {
  if (ms === 0) {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(resolve, ms);
    signal?.addEventListener(
      "abort",
      () => {
        clearTimeout(timeout);
        reject(new DOMException("Aborted", "AbortError"));
      },
      { once: true },
    );
  });
}

function aiModel(transcript: TranscriptEntry[], delayMs: number): MockLanguageModelV3 {
  let index = 0;
  return new MockLanguageModelV3({
    doGenerate: async ({ abortSignal }) => {
      await delay(delayMs, abortSignal);
      const entry = transcript[index++] ?? { text: "done" };
      if ("toolName" in entry) {
        return {
          content: [
            {
              type: "tool-call" as const,
              toolCallId: `call-${index}`,
              toolName: entry.toolName,
              input: JSON.stringify(entry.input),
            },
          ],
          finishReason: { unified: "tool-calls" as const, raw: undefined },
          usage,
          warnings: [],
        };
      }
      return {
        content: [{ type: "text" as const, text: entry.text }],
        finishReason: { unified: "stop" as const, raw: undefined },
        usage,
        warnings: [],
      };
    },
  });
}

function aiTools(session: RuntimeSession) {
  return {
    inspect_input: tool({
      description: "Inspect the current job-scoped input",
      inputSchema: toolInputSchemas.inspect_input,
      execute: (input) => session.execute("inspect_input", input),
    }),
    extract_native_text: tool({
      description: "Read bounded native text observations",
      inputSchema: toolInputSchemas.extract_native_text,
      execute: (input) => session.execute("extract_native_text", input),
    }),
    extract_table: tool({
      description: "Read bounded table observations",
      inputSchema: toolInputSchemas.extract_table,
      execute: (input) => session.execute("extract_table", input),
    }),
    ocr_pages: tool({
      description: "Read bounded OCR observations",
      inputSchema: toolInputSchemas.ocr_pages,
      execute: (input) => session.execute("ocr_pages", input),
    }),
    read_page_region: tool({
      description: "Read one bounded region in the current document",
      inputSchema: toolInputSchemas.read_page_region,
      execute: (input) => session.execute("read_page_region", input),
    }),
    validate_proposal: tool({
      description: "Validate a structured proposal",
      inputSchema: toolInputSchemas.validate_proposal,
      execute: (input) => session.execute("validate_proposal", input),
    }),
    submit_structured_proposal: tool({
      description: "Submit the only accepted completion shape",
      inputSchema: toolInputSchemas.submit_structured_proposal,
      execute: (input) => session.execute("submit_structured_proposal", input),
    }),
  };
}

export async function runToolLoopAgent(input: {
  transcript: TranscriptEntry[];
  fixture?: RuntimeFixture;
  signal?: AbortSignal;
  delayMs?: number;
}): Promise<RuntimeResult> {
  const session = new RuntimeSession(input.fixture);
  let steps = 0;
  const agent = new ToolLoopAgent({
    model: aiModel(input.transcript, input.delayMs ?? 0),
    instructions: "Normalize only the current parse job through the fixed tools.",
    tools: aiTools(session),
    stopWhen: stepCountIs(8),
    onStepFinish: () => {
      steps += 1;
    },
  });

  try {
    const generated = await agent.generate({
      prompt: "Normalize the current document.",
      abortSignal: input.signal,
    });
    if (
      generated.steps.some((step) =>
        step.toolCalls.some(({ toolName }) => !toolNames.includes(toolName as ToolName)),
      )
    ) {
      session.reject("forbidden_tool");
    }
  } catch (error) {
    if (isAbort(error) || input.signal?.aborted) {
      return session.result("tool-loop-agent", steps, true);
    }
    session.reject("forbidden_tool");
    return session.result("tool-loop-agent", steps);
  }

  if (!session.acceptedProposal && steps >= 8) {
    session.reject("normalizer_budget_exhausted");
  }
  return session.result("tool-loop-agent", steps);
}

const piModel: Model<"mock"> = {
  id: "cancan-mock",
  name: "CanCan deterministic mock",
  api: "mock",
  provider: "cancan",
  baseUrl: "local://mock",
  reasoning: false,
  input: ["text"],
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  contextWindow: 10_000,
  maxTokens: 1_000,
};

function piMessage(entry: TranscriptEntry, index: number): AssistantMessage {
  return {
    role: "assistant",
    content:
      "toolName" in entry
        ? [
            {
              type: "toolCall",
              id: `call-${index}`,
              name: entry.toolName,
              arguments: entry.input as Record<string, unknown>,
            },
          ]
        : [{ type: "text", text: entry.text }],
    api: "mock",
    provider: "cancan",
    model: "cancan-mock",
    usage: {
      input: 10,
      output: 5,
      cacheRead: 0,
      cacheWrite: 0,
      totalTokens: 15,
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
    },
    stopReason: "toolName" in entry ? "toolUse" : "stop",
    timestamp: index,
  };
}

function piStream(transcript: TranscriptEntry[], delayMs: number) {
  let index = 0;
  return (_model: Model<string>, _context: unknown, options?: { signal?: AbortSignal }) => {
    const stream = createAssistantMessageEventStream();
    void (async () => {
      try {
        await delay(delayMs, options?.signal);
        const entry = transcript[index++] ?? { text: "done" };
        const message = piMessage(entry, index);
        stream.push({ type: "start", partial: message });
        if ("toolName" in entry) {
          const toolCall = message.content[0];
          if (toolCall?.type === "toolCall") {
            stream.push({ type: "toolcall_start", contentIndex: 0, partial: message });
            stream.push({
              type: "toolcall_end",
              contentIndex: 0,
              toolCall,
              partial: message,
            });
          }
        }
        stream.push({
          type: "done",
          reason: "toolName" in entry ? "toolUse" : "stop",
          message,
        });
      } catch (error) {
        const message = piMessage({ text: "cancelled" }, index + 1);
        message.stopReason = "aborted";
        message.errorMessage = error instanceof Error ? error.message : "aborted";
        stream.push({ type: "error", reason: "aborted", error: message });
      }
    })();
    return stream;
  };
}

export const piToolParameters = Object.fromEntries(
  toolNames.map((name) => [
    name,
    z.toJSONSchema(toolInputSchemas[name], { target: "draft-7" }),
  ]),
) as unknown as Record<ToolName, AgentTool["parameters"]>;

function piTools(session: RuntimeSession): AgentTool[] {
  return toolNames.map((name) => ({
    name,
    label: name,
    description: name,
    parameters: piToolParameters[name],
    executionMode: "sequential",
    execute: async (_toolCallId, params) => {
      const result = await session.execute(name, params);
      return {
        content: [{ type: "text", text: JSON.stringify(result) }],
        details: result,
      };
    },
  }));
}

export async function runPiAgentCore(input: {
  transcript: TranscriptEntry[];
  fixture?: RuntimeFixture;
  signal?: AbortSignal;
  delayMs?: number;
}): Promise<RuntimeResult> {
  const session = new RuntimeSession(input.fixture);
  let steps = 0;
  const agent = new Agent({
    initialState: {
      systemPrompt: "Normalize only the current parse job through the fixed tools.",
      model: piModel,
      thinkingLevel: "off",
      tools: piTools(session),
      messages: [],
    },
    streamFn: piStream(input.transcript, input.delayMs ?? 0),
    toolExecution: "sequential",
    afterToolCall: async () => {
      if (session.acceptedProposal || session.toolCalls.length >= 8) {
        return { terminate: true };
      }
      return undefined;
    },
  });
  const unsubscribe = agent.subscribe((event) => {
    if (event.type === "turn_start") {
      steps += 1;
    }
    if (
      event.type === "tool_execution_end" &&
      !toolNames.includes(event.toolName as ToolName)
    ) {
      session.reject("forbidden_tool");
    }
  });
  const abort = () => agent.abort();
  input.signal?.addEventListener("abort", abort, { once: true });

  try {
    if (input.signal?.aborted) {
      return session.result("pi-agent-core", steps, true);
    }
    await agent.prompt("Normalize the current document.");
  } catch (error) {
    if (isAbort(error) || input.signal?.aborted) {
      return session.result("pi-agent-core", steps, true);
    }
    session.reject("runtime_error");
  } finally {
    unsubscribe();
    input.signal?.removeEventListener("abort", abort);
  }

  if (input.signal?.aborted && !session.acceptedProposal) {
    return session.result("pi-agent-core", steps, true);
  }
  if (!session.acceptedProposal && session.toolCalls.length >= 8) {
    session.reject("normalizer_budget_exhausted");
  }
  return session.result("pi-agent-core", steps);
}

export async function runSinglePass(input: {
  proposal: unknown;
  fixture?: RuntimeFixture;
  signal?: AbortSignal;
}): Promise<RuntimeResult> {
  const session = new RuntimeSession(input.fixture);
  if (input.signal?.aborted) {
    return session.result("single-pass", 0, true);
  }
  try {
    const generated = await generateObject({
      model: aiModel([{ text: JSON.stringify(input.proposal) }], 0),
      schema: proposalSchema,
      prompt: "Normalize the current document into the structured proposal schema.",
      abortSignal: input.signal,
    });
    await session.submitSinglePass(generated.object);
  } catch (error) {
    if (isAbort(error) || input.signal?.aborted) {
      return session.result("single-pass", 1, true);
    }
    session.reject("structured_proposal_invalid");
  }
  return session.result("single-pass", 1);
}

export function happyPathTranscript(proposal: unknown): TranscriptEntry[] {
  return [
    { toolName: "inspect_input", input: {} },
    { toolName: "extract_table", input: { maxRows: 100 } },
    { toolName: "validate_proposal", input: { proposal } },
    { toolName: "submit_structured_proposal", input: { proposal } },
    { text: "submitted" },
  ];
}
