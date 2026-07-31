import { describe, expect, it } from "vitest";

import { GMAIL_READONLY_SCOPE, type GmailOAuthCrypto } from "./gmail-oauth";
import { runGmailOAuthWorker } from "./gmail-oauth-worker";
import type { GmailOAuthWorkerMessage } from "./gmail-oauth-worker-protocol";

describe("Gmail OAuth worker", () => {
  it("bridges the frozen adapter to host-owned operations without returning tokens", async () => {
    const input = new AsyncLineQueue();
    const output: GmailOAuthWorkerMessage[] = [];
    const worker = runGmailOAuthWorker(
      input,
      (message) => {
        output.push(message);
        if (message.type === "open_external_url") {
          input.push(
            JSON.stringify({ type: "opened", requestId: message.requestId }),
          );
        } else if (message.type === "wait_for_callback") {
          const authorization = output.find(
            (candidate) => candidate.type === "open_external_url",
          );
          if (!authorization || authorization.type !== "open_external_url") {
            throw new Error("authorization URL missing");
          }
          const state = new URL(authorization.authorizationUrl).searchParams.get(
            "state",
          );
          input.push(
            JSON.stringify({
              type: "callback",
              requestId: message.requestId,
              callbackUrl: `http://127.0.0.1:49152?code=synthetic-code&state=${state}`,
            }),
          );
        } else if (message.type === "close_listener") {
          input.push(
            JSON.stringify({ type: "closed", requestId: message.requestId }),
          );
        } else if (message.type === "persist") {
          expect(message).toEqual({
            type: "persist",
            requestId: "request-1",
            mailboxAddress: "owner@example.com",
            refreshToken: "synthetic-refresh-token",
          });
          input.push(
            JSON.stringify({ type: "persisted", requestId: message.requestId }),
          );
        } else if (message.type === "result") {
          input.push(JSON.stringify({ type: "shutdown" }));
        }
      },
      {
        crypto: deterministicCrypto(),
        executeHttp: async (request) => {
          if (request.url === "https://oauth2.googleapis.com/token") {
            return {
              access_token: "synthetic-access-token",
              expires_in: 3600,
              refresh_token: "synthetic-refresh-token",
              scope: GMAIL_READONLY_SCOPE,
              token_type: "Bearer",
            };
          }
          return { emailAddress: "owner@example.com" };
        },
      },
    );

    input.push(
      JSON.stringify({
        type: "authorize",
        requestId: "request-1",
        clientId: "synthetic-client-id",
        loopbackPort: 49_152,
      }),
    );
    await worker;

    expect(output.at(-1)).toEqual({
      type: "result",
      requestId: "request-1",
      mailboxAddress: "owner@example.com",
    });
    expect(JSON.stringify(output.at(-1))).not.toContain("token");
    expect(output.map(({ type }) => type)).toEqual([
      "open_external_url",
      "wait_for_callback",
      "close_listener",
      "persist",
      "result",
    ]);
  });
});

class AsyncLineQueue implements AsyncIterable<string>, AsyncIterator<string> {
  private readonly pending: Array<(value: IteratorResult<string>) => void> = [];
  private readonly values: string[] = [];

  [Symbol.asyncIterator](): AsyncIterator<string> {
    return this;
  }

  next(): Promise<IteratorResult<string>> {
    const value = this.values.shift();
    if (value !== undefined) {
      return Promise.resolve({ done: false, value });
    }
    return new Promise((resolve) => this.pending.push(resolve));
  }

  push(value: string): void {
    const resolve = this.pending.shift();
    if (resolve) {
      resolve({ done: false, value });
    } else {
      this.values.push(value);
    }
  }
}

function deterministicCrypto(): GmailOAuthCrypto {
  let call = 0;
  return {
    randomBytes(size) {
      call += 1;
      return new Uint8Array(size).fill(call);
    },
    async sha256() {
      return new Uint8Array(32).fill(3);
    },
  };
}
