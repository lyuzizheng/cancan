import { createInterface } from "node:readline";

import { runGmailOAuthWorker } from "./gmail-oauth-worker";
import type { GmailOAuthWorkerMessage } from "./gmail-oauth-worker-protocol";

function send(message: GmailOAuthWorkerMessage): void {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
void runGmailOAuthWorker(lines, send).then(
  () => lines.close(),
  () => {
    lines.close();
    process.exitCode = 1;
  },
);

send({
  type: "ready",
  protocolVersion: 1,
  runtime: "gmail-oauth-v1",
  environmentCleared: Object.keys(process.env).every(
    (key) => key === "__CF_USER_TEXT_ENCODING",
  ),
});
