import { createInterface } from "node:readline";

import { normalizeWithMock } from "./mock-normalizer";
import { parseWorkerCommand, runCoreCommand } from "./worker-protocol";

function send(message: unknown): void {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

function parseCommand(line: string) {
  let value: unknown;
  try {
    value = JSON.parse(line);
  } catch {
    return undefined;
  }
  return parseWorkerCommand(value);
}

async function run(): Promise<void> {
  for await (const line of createInterface({ input: process.stdin, crlfDelay: Infinity })) {
    const command = parseCommand(line);
    if (!command) {
      send({ type: "error", code: "invalid_command" });
      continue;
    }
    if (command.type === "shutdown") {
      return;
    }
    send({
      type: "result",
      requestId: command.requestId,
      result:
        command.type === "core" ? runCoreCommand(command) : await normalizeWithMock(command),
    });
  }
}

void run();

send({
  type: "ready",
  protocolVersion: 1,
  runtime: "single-pass-mock",
  environmentCleared: Object.keys(process.env).every(
    (key) => key === "__CF_USER_TEXT_ENCODING",
  ),
});
