import { describe, expect, it } from "vitest";

import {
  parseGmailOAuthWorkerCommand,
  persistenceMessage,
} from "./gmail-oauth-worker-protocol";

describe("Gmail OAuth worker protocol", () => {
  it("accepts only the exact bounded authorization command", () => {
    expect(
      parseGmailOAuthWorkerCommand({
        type: "authorize",
        requestId: "request-1",
        clientId: "synthetic-client-id",
        loopbackPort: 49_152,
      }),
    ).toEqual({
      type: "authorize",
      requestId: "request-1",
      clientId: "synthetic-client-id",
      loopbackPort: 49_152,
    });
    expect(
      parseGmailOAuthWorkerCommand({
        type: "authorize",
        requestId: "request-1",
        clientId: "synthetic-client-id",
        loopbackPort: 0,
      }),
    ).toBeUndefined();
    expect(
      parseGmailOAuthWorkerCommand({
        type: "authorize",
        requestId: "request-1",
        clientId: "synthetic-client-id",
        loopbackPort: 49_152,
        refreshToken: "must-not-be-accepted",
      }),
    ).toBeUndefined();
  });

  it("accepts only request-scoped host responses", () => {
    expect(
      parseGmailOAuthWorkerCommand({
        type: "callback",
        requestId: "request-1",
        callbackUrl: "http://127.0.0.1:49152?code=code&state=state",
      }),
    ).toEqual({
      type: "callback",
      requestId: "request-1",
      callbackUrl: "http://127.0.0.1:49152?code=code&state=state",
    });
    expect(
      parseGmailOAuthWorkerCommand({
        type: "opened",
        requestId: "request-1",
        detail: "provider body",
      }),
    ).toBeUndefined();
  });

  it("sends only the refresh token required by the Rust persistence sink", () => {
    const message = persistenceMessage("request-1", {
      mailboxAddress: "owner@example.com",
      tokens: {
        accessToken: "access-token",
        expiresInSeconds: 3600,
        refreshToken: "refresh-token",
        scope: "https://www.googleapis.com/auth/gmail.readonly",
        tokenType: "Bearer",
      },
    });

    expect(message).toEqual({
      type: "persist",
      requestId: "request-1",
      mailboxAddress: "owner@example.com",
      refreshToken: "refresh-token",
    });
    expect(JSON.stringify(message)).not.toContain("access-token");
  });
});
