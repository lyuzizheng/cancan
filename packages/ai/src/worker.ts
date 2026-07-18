import { createInterface } from "node:readline";

import { normalizeWithMock, type NormalizeDocumentInput } from "./mock-normalizer";

interface NormalizeCommand extends NormalizeDocumentInput {
  type: "normalize";
  requestId: string;
}

function send(message: unknown): void {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

function parseCommand(line: string): NormalizeCommand | { type: "shutdown" } | undefined {
  let value: unknown;
  try {
    value = JSON.parse(line);
  } catch {
    return undefined;
  }
  if (!value || Array.isArray(value) || typeof value !== "object") {
    return undefined;
  }
  const command = value as Record<string, unknown>;
  if (command.type === "shutdown" && Object.keys(command).length === 1) {
    return { type: "shutdown" };
  }
  const keys = Object.keys(command).sort().join(",");
  if (
    command.type !== "normalize" ||
    keys !== "content,documentId,mimeType,requestId,type" ||
    typeof command.requestId !== "string" ||
    typeof command.documentId !== "string" ||
    (command.mimeType !== "application/pdf" && command.mimeType !== "text/csv") ||
    typeof command.content !== "string"
  ) {
    return undefined;
  }
  return command as unknown as NormalizeCommand;
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
