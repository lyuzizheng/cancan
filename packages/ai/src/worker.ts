import { createInterface } from "node:readline";

import { normalizeWithMock } from "./mock-normalizer";
import { parseWorkerCommand } from "./worker-protocol";

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

createInterface({ input: process.stdin, crlfDelay: Infinity }).on("line", (line) => {
  const command = parseCommand(line);
  if (!command) {
    send({ type: "error", code: "invalid_command" });
    return;
  }
  if (command.type === "shutdown") {
    process.exit(0);
  }
  send({
    type: "result",
    requestId: command.requestId,
    result: normalizeWithMock(command),
  });
});

send({
  type: "ready",
  protocolVersion: 1,
  runtime: "single-pass-mock",
  environmentCleared: Object.keys(process.env).every(
    (key) => key === "__CF_USER_TEXT_ENCODING",
  ),
});
