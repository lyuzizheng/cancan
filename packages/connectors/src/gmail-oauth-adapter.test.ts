import { describe, expect, it, vi } from "vitest";

import {
  executeGmailJsonRequest,
  GmailOAuthContractError,
  GmailOAuthExecutionError,
  runPrivilegedGmailOAuth,
  type GmailOAuthCrypto,
  type GmailOAuthLoopbackListener,
} from "./index";

const clientId = "synthetic-client.apps.googleusercontent.com";
const accessToken = "synthetic-access-token";
const refreshToken = "synthetic-refresh-token";

describe("privileged Gmail OAuth adapter", () => {
  it("closes the callback listener before exchanging tokens and returns only mailbox identity", async () => {
    const events: string[] = [];
    const persisted: unknown[] = [];
    const listener = fakeListener(events);

    const result = await runPrivilegedGmailOAuth(clientId, {
      crypto: deterministicCrypto(),
      async executeHttp(request) {
        if (request.url === "https://oauth2.googleapis.com/token") {
          events.push("http:token");
          return {
            access_token: accessToken,
            expires_in: 3600,
            refresh_token: refreshToken,
            scope: "https://www.googleapis.com/auth/gmail.readonly",
            token_type: "Bearer",
          };
        }
        events.push("http:profile");
        expect(request).toEqual({
          url: "https://gmail.googleapis.com/gmail/v1/users/me/profile",
          init: {
            method: "GET",
            headers: { Authorization: `Bearer ${accessToken}` },
          },
        });
        return { emailAddress: " Owner+Statements@Example.COM " };
      },
      async openExternalUrl(authorizationUrl) {
        events.push("browser:open");
        const url = new URL(authorizationUrl);
        expect(url.searchParams.get("redirect_uri")).toBe(
          "http://127.0.0.1:49152",
        );
      },
      async openLoopbackListener() {
        events.push("listener:open");
        return listener;
      },
      async persistAuthorizedMailbox(mailbox) {
        events.push("persist");
        persisted.push(mailbox);
      },
    });

    expect(events).toEqual([
      "listener:open",
      "browser:open",
      "callback:wait",
      "listener:close",
      "http:token",
      "http:profile",
      "persist",
    ]);
    expect(persisted).toEqual([
      {
        mailboxAddress: "owner+statements@example.com",
        tokens: {
          accessToken,
          expiresInSeconds: 3600,
          refreshToken,
          scope: "https://www.googleapis.com/auth/gmail.readonly",
          tokenType: "Bearer",
        },
      },
    ]);
    expect(result).toEqual({
      mailboxAddress: "owner+statements@example.com",
    });
    expect(JSON.stringify(result)).not.toContain(accessToken);
    expect(JSON.stringify(result)).not.toContain(refreshToken);
  });

  it("closes the listener and redacts dependency failures", async () => {
    const events: string[] = [];
    const executeHttp = vi.fn();
    const persistAuthorizedMailbox = vi.fn();

    await expect(
      runPrivilegedGmailOAuth(clientId, {
        crypto: deterministicCrypto(),
        executeHttp,
        async openExternalUrl() {
          events.push("browser:open");
          throw new GmailOAuthContractError(
            `browser failed with ${refreshToken}`,
          );
        },
        async openLoopbackListener() {
          events.push("listener:open");
          return fakeListener(events);
        },
        persistAuthorizedMailbox,
      }),
    ).rejects.toThrow("Gmail authorization could not be completed.");

    expect(events).toEqual([
      "listener:open",
      "browser:open",
      "listener:close",
    ]);
    expect(executeHttp).not.toHaveBeenCalled();
    expect(persistAuthorizedMailbox).not.toHaveBeenCalled();
  });

  it("validates callback state before any token exchange", async () => {
    const events: string[] = [];
    const executeHttp = vi.fn();
    const persistAuthorizedMailbox = vi.fn();

    await expect(
      runPrivilegedGmailOAuth(clientId, {
        crypto: deterministicCrypto(),
        executeHttp,
        async openExternalUrl() {
          events.push("browser:open");
        },
        async openLoopbackListener() {
          events.push("listener:open");
          return fakeListener(
            events,
            "http://127.0.0.1:49152?state=wrong&error=access_denied",
          );
        },
        persistAuthorizedMailbox,
      }),
    ).rejects.toThrow(
      "Gmail authorization callback did not match this request.",
    );

    expect(events).toEqual([
      "listener:open",
      "browser:open",
      "callback:wait",
      "listener:close",
    ]);
    expect(executeHttp).not.toHaveBeenCalled();
    expect(persistAuthorizedMailbox).not.toHaveBeenCalled();
  });

  it("does not exchange the authorization code while the loopback listener remains open", async () => {
    const executeHttp = vi.fn();
    const persistAuthorizedMailbox = vi.fn();
    const close = vi.fn(async () => {
      if (close.mock.calls.length === 1) {
        throw new Error("listener close failed");
      }
    });

    await expect(
      runPrivilegedGmailOAuth(clientId, {
        crypto: deterministicCrypto(),
        executeHttp,
        async openExternalUrl() {},
        async openLoopbackListener() {
          return {
            port: 49152,
            close,
            async waitForCallback() {
              return "http://127.0.0.1:49152?state=IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI&code=synthetic-code";
            },
          };
        },
        persistAuthorizedMailbox,
      }),
    ).rejects.toThrow("Gmail authorization could not be completed.");

    expect(close).toHaveBeenCalledTimes(2);
    expect(executeHttp).not.toHaveBeenCalled();
    expect(persistAuthorizedMailbox).not.toHaveBeenCalled();
  });

  it("does not echo tokens when privileged persistence fails", async () => {
    const events: string[] = [];

    let thrown: unknown;
    try {
      await runPrivilegedGmailOAuth(clientId, {
        crypto: deterministicCrypto(),
        async executeHttp(request) {
          return request.init.method === "POST"
            ? {
                access_token: accessToken,
                expires_in: 3600,
                refresh_token: refreshToken,
                scope: "https://www.googleapis.com/auth/gmail.readonly",
                token_type: "Bearer",
              }
            : { emailAddress: "owner@example.com" };
        },
        async openExternalUrl() {},
        async openLoopbackListener() {
          return fakeListener(events);
        },
        async persistAuthorizedMailbox() {
          throw new GmailOAuthExecutionError(
            `save failed for ${refreshToken}`,
            "authorization_failed",
          );
        },
      });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(Error);
    expect((thrown as Error).message).toBe(
      "Gmail authorization could not be completed.",
    );
    expect((thrown as Error).message).not.toContain(accessToken);
    expect((thrown as Error).message).not.toContain(refreshToken);
    expect(events).toEqual(["callback:wait", "listener:close"]);
  });
});

describe("Gmail JSON transport", () => {
  it("executes the contract request through an injected fetch boundary", async () => {
    const fetchImpl = vi.fn(async () =>
      new Response(JSON.stringify({ emailAddress: "owner@example.com" }), {
        status: 200,
      }),
    );

    await expect(
      executeGmailJsonRequest(
        {
          url: "https://gmail.googleapis.com/gmail/v1/users/me/profile",
          init: {
            method: "GET",
            headers: { Authorization: `Bearer ${accessToken}` },
          },
        },
        fetchImpl,
      ),
    ).resolves.toEqual({ emailAddress: "owner@example.com" });
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it("redacts network, provider, and malformed-response failures", async () => {
    const providerBody = `provider rejected ${accessToken}`;
    const cases = [
      vi.fn(async () => {
        throw new Error(`network leaked ${refreshToken}`);
      }),
      vi.fn(async () => new Response(providerBody, { status: 400 })),
      vi.fn(async () => new Response("not-json", { status: 200 })),
    ];

    for (const fetchImpl of cases) {
      let thrown: unknown;
      try {
        await executeGmailJsonRequest(
          {
            url: "https://oauth2.googleapis.com/token",
            init: {
              method: "POST",
              headers: {
                "Content-Type": "application/x-www-form-urlencoded",
              },
              body: `code=synthetic&access_token=${accessToken}`,
            },
          },
          fetchImpl,
        );
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(Error);
      expect((thrown as Error).message).toBe("Gmail request failed.");
      expect((thrown as Error).message).not.toContain(accessToken);
      expect((thrown as Error).message).not.toContain(refreshToken);
      expect((thrown as Error).message).not.toContain(providerBody);
    }
  });
});

function fakeListener(
  events: string[],
  callbackUrl = "http://127.0.0.1:49152?state=IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI&code=synthetic-code",
): GmailOAuthLoopbackListener {
  return {
    port: 49152,
    async close() {
      events.push("listener:close");
    },
    async waitForCallback() {
      events.push("callback:wait");
      return callbackUrl;
    },
  };
}

function deterministicCrypto(): GmailOAuthCrypto {
  const randomValues = [
    new Uint8Array(32).fill(0x11),
    new Uint8Array(32).fill(0x22),
  ];
  return {
    randomBytes(size) {
      const value = randomValues.shift();
      if (value === undefined || value.length !== size) {
        throw new Error("Unexpected deterministic random byte request.");
      }
      return value;
    },
    async sha256(value) {
      return new Uint8Array(
        await globalThis.crypto.subtle.digest(
          "SHA-256",
          new TextEncoder().encode(value),
        ),
      );
    },
  };
}
