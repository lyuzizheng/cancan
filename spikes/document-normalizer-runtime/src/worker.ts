import { createInterface } from "node:readline";

import { z } from "zod";

import { createRuntimeFixture } from "./harness";
import { runSinglePass } from "./runtimes";

const commandSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("ping"), requestId: z.string() }).strict(),
  z
    .object({
      type: z.literal("run_fixture"),
      requestId: z.string(),
      delayMs: z.number().int().min(0).max(5_000).optional(),
    })
    .strict(),
  z.object({ type: z.literal("cancel"), requestId: z.string() }).strict(),
  z.object({ type: z.literal("shutdown") }).strict(),
  z.object({ type: z.literal("crash") }).strict(),
]);

const active = new Map<string, AbortController>();

function send(message: unknown): void {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

function delay(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(resolve, ms);
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(timeout);
        reject(new DOMException("Aborted", "AbortError"));
      },
      { once: true },
    );
  });
}

async function handle(line: string): Promise<void> {
  let raw: unknown;
  try {
    raw = JSON.parse(line);
  } catch {
    send({ type: "error", code: "invalid_command" });
    return;
  }
  const parsed = commandSchema.safeParse(raw);
  if (!parsed.success) {
    send({ type: "error", code: "invalid_command" });
    return;
  }

  const command = parsed.data;
  switch (command.type) {
    case "ping":
      send({ type: "pong", requestId: command.requestId });
      return;
    case "run_fixture": {
      const controller = new AbortController();
      active.set(command.requestId, controller);
      try {
        await delay(command.delayMs ?? 0, controller.signal);
        const fixture = createRuntimeFixture();
        const result = await runSinglePass({
          proposal: fixture.proposal,
          fixture,
          signal: controller.signal,
        });
        send({ type: "result", requestId: command.requestId, result });
      } catch {
        send({
          type: "result",
          requestId: command.requestId,
          result: {
            runtime: "single-pass",
            outcome: "cancelled",
            code: "cancelled",
            modelSteps: 0,
            toolCalls: [],
            submissions: 0,
          },
        });
      } finally {
        active.delete(command.requestId);
      }
      return;
    }
    case "cancel":
      active.get(command.requestId)?.abort();
      return;
    case "shutdown":
      for (const controller of active.values()) {
        controller.abort();
      }
      process.exit(0);
    case "crash":
      process.exit(70);
  }
}

process.on("SIGTERM", () => {
  for (const controller of active.values()) {
    controller.abort();
  }
  process.exit(0);
});

createInterface({ input: process.stdin, crlfDelay: Infinity }).on("line", (line) => {
  void handle(line);
});

send({
  type: "ready",
  protocolVersion: 1,
  runtime: "single-pass",
  aiSdkApi: "generateObject",
  environmentEvidencePresent: "CANCAN_EVIDENCE_PROVIDER_KEY" in process.env,
});
