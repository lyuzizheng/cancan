import { describe, expect, it } from "vitest";

import {
  buildGmailProfileRequest,
  buildGmailTokenExchangeRequest,
  createGmailOAuthSession,
  GMAIL_READONLY_SCOPE,
  GmailOAuthContractError,
  parseGmailOAuthCallback,
  parseGmailOAuthTokenResponse,
  parseGmailProfileResponse,
  type GmailOAuthCrypto,
} from "./gmail-oauth";

const clientId = "synthetic-client.apps.googleusercontent.com";
const accessToken = "synthetic-access-token";
const refreshToken = "synthetic-refresh-token";

describe("Gmail OAuth contract", () => {
  it("builds one Desktop OAuth PKCE request for gmail.readonly", async () => {
    const crypto = deterministicCrypto();
    const session = await createGmailOAuthSession(
      { clientId, loopbackPort: 49152 },
      crypto,
    );
    const authorizationUrl = new URL(session.authorizationUrl);

    expect(authorizationUrl.origin).toBe("https://accounts.google.com");
    expect(authorizationUrl.pathname).toBe("/o/oauth2/v2/auth");
    expect(Object.fromEntries(authorizationUrl.searchParams)).toEqual({
      access_type: "offline",
      client_id: clientId,
      code_challenge: "tZ5lxAKmKebFsoNVx2KKEAImK1W0OlL5QcAnQOO9-vw",
      code_challenge_method: "S256",
      prompt: "consent",
      redirect_uri: "http://127.0.0.1:49152",
      response_type: "code",
      scope: GMAIL_READONLY_SCOPE,
      state: "IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI",
    });
    expect(session.codeVerifier).toBe(
      "ERERERERERERERERERERERERERERERERERERERERERE",
    );
    expect(crypto.sha256Inputs).toEqual([
      "ERERERERERERERERERERERERERERERERERERERERERE",
    ]);
  });

  it("rejects missing credentials and invalid loopback ports", async () => {
    await expect(
      createGmailOAuthSession(
        { clientId: "", loopbackPort: 49152 },
        deterministicCrypto(),
      ),
    ).rejects.toThrow(GmailOAuthContractError);
    await expect(
      createGmailOAuthSession(
        { clientId, loopbackPort: 0 },
        deterministicCrypto(),
      ),
    ).rejects.toThrow(GmailOAuthContractError);
  });

  it("accepts only the matching loopback callback and state", async () => {
    const session = await createGmailOAuthSession(
      { clientId, loopbackPort: 49152 },
      deterministicCrypto(),
    );

    expect(
      parseGmailOAuthCallback(
        session,
        `${session.redirectUri}?state=${session.state}&code=synthetic-code`,
      ),
    ).toEqual({ authorizationCode: "synthetic-code" });

    expect(() =>
      parseGmailOAuthCallback(
        session,
        `${session.redirectUri}?state=wrong-state&code=synthetic-code`,
      ),
    ).toThrow(GmailOAuthContractError);
    expect(() =>
      parseGmailOAuthCallback(
        session,
        `http://localhost:49152?state=${session.state}&code=synthetic-code`,
      ),
    ).toThrow(GmailOAuthContractError);
  });

  it("turns provider callback failures into stable redacted errors", async () => {
    const session = await createGmailOAuthSession(
      { clientId, loopbackPort: 49152 },
      deterministicCrypto(),
    );
    const providerDescription = "sensitive-provider-description";

    expect(() =>
      parseGmailOAuthCallback(
        session,
        `${session.redirectUri}?error=access_denied`,
      ),
    ).toThrow("Gmail authorization callback did not match this request.");
    expect(() =>
      parseGmailOAuthCallback(
        session,
        `${session.redirectUri}?state=wrong-state&error=access_denied`,
      ),
    ).toThrow("Gmail authorization callback did not match this request.");

    try {
      parseGmailOAuthCallback(
        session,
        `${session.redirectUri}?state=${session.state}&error=access_denied&error_description=${providerDescription}`,
      );
      throw new Error("Expected the provider callback to fail.");
    } catch (error) {
      expect(error).toBeInstanceOf(GmailOAuthContractError);
      expect((error as Error).message).toBe("Gmail authorization was not completed.");
      expect((error as Error).message).not.toContain(providerDescription);
    }
  });

  it("builds a token exchange with PKCE and no client secret", async () => {
    const session = await createGmailOAuthSession(
      { clientId, loopbackPort: 49152 },
      deterministicCrypto(),
    );
    const request = buildGmailTokenExchangeRequest(
      session,
      "synthetic-authorization-code",
    );
    const body = new URLSearchParams(request.init.body);

    expect(request.url).toBe("https://oauth2.googleapis.com/token");
    expect(request.init).toMatchObject({
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
    });
    expect(Object.fromEntries(body)).toEqual({
      client_id: clientId,
      code: "synthetic-authorization-code",
      code_verifier: session.codeVerifier,
      grant_type: "authorization_code",
      redirect_uri: session.redirectUri,
    });
    expect(request.init.body).not.toContain("client_secret");
  });

  it("parses only a complete read-only token response", () => {
    expect(
      parseGmailOAuthTokenResponse({
        access_token: accessToken,
        expires_in: 3600,
        refresh_token: refreshToken,
        scope: GMAIL_READONLY_SCOPE,
        token_type: "Bearer",
      }),
    ).toEqual({
      accessToken,
      expiresInSeconds: 3600,
      refreshToken,
      scope: GMAIL_READONLY_SCOPE,
      tokenType: "Bearer",
    });

    expect(() =>
      parseGmailOAuthTokenResponse({
        access_token: accessToken,
        expires_in: 3600,
        scope: GMAIL_READONLY_SCOPE,
        token_type: "Bearer",
      }),
    ).toThrow(GmailOAuthContractError);
    expect(() =>
      parseGmailOAuthTokenResponse({
        access_token: accessToken,
        expires_in: 3600,
        refresh_token: refreshToken,
        scope: `${GMAIL_READONLY_SCOPE} https://www.googleapis.com/auth/gmail.modify`,
        token_type: "Bearer",
      }),
    ).toThrow(GmailOAuthContractError);
  });

  it("builds the profile request and normalizes the mailbox identity", () => {
    expect(buildGmailProfileRequest(accessToken)).toEqual({
      url: "https://gmail.googleapis.com/gmail/v1/users/me/profile",
      init: {
        method: "GET",
        headers: { Authorization: `Bearer ${accessToken}` },
      },
    });
    expect(
      parseGmailProfileResponse({
        emailAddress: "  Owner+Statements@Example.COM ",
        historyId: "12345",
        messagesTotal: 10,
        threadsTotal: 8,
      }),
    ).toEqual({
      mailboxAddress: "owner+statements@example.com",
    });
  });

  it("does not echo tokens or provider payloads in contract errors", () => {
    const providerPayload = JSON.stringify({
      access_token: accessToken,
      refresh_token: refreshToken,
    });

    try {
      parseGmailOAuthTokenResponse(JSON.parse(providerPayload));
      throw new Error("Expected the incomplete response to fail.");
    } catch (error) {
      expect(error).toBeInstanceOf(GmailOAuthContractError);
      expect((error as Error).message).not.toContain(accessToken);
      expect((error as Error).message).not.toContain(refreshToken);
      expect((error as Error).message).not.toContain(providerPayload);
    }
  });
});

function deterministicCrypto(): GmailOAuthCrypto & {
  sha256Inputs: string[];
} {
  const randomValues = [
    new Uint8Array(32).fill(0x11),
    new Uint8Array(32).fill(0x22),
  ];
  const sha256Inputs: string[] = [];

  return {
    sha256Inputs,
    randomBytes(size) {
      const value = randomValues.shift();
      if (value === undefined || value.length !== size) {
        throw new Error("Unexpected deterministic random byte request.");
      }
      return value;
    },
    async sha256(value) {
      sha256Inputs.push(value);
      return new Uint8Array(
        await globalThis.crypto.subtle.digest(
          "SHA-256",
          new TextEncoder().encode(value),
        ),
      );
    },
  };
}
