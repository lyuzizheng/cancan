export const GMAIL_READONLY_SCOPE =
  "https://www.googleapis.com/auth/gmail.readonly";

const GOOGLE_AUTHORIZATION_ENDPOINT =
  "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token";
const GMAIL_PROFILE_ENDPOINT =
  "https://gmail.googleapis.com/gmail/v1/users/me/profile";

export interface GmailOAuthClientConfiguration {
  clientId: string;
  loopbackPort: number;
}

export interface GmailOAuthCrypto {
  randomBytes(size: number): Uint8Array;
  sha256(value: string): Promise<Uint8Array>;
}

export interface GmailOAuthSession {
  authorizationUrl: string;
  clientId: string;
  codeVerifier: string;
  redirectUri: string;
  state: string;
}

export interface GmailOAuthCallback {
  authorizationCode: string;
}

export interface GmailHttpRequest {
  url: string;
  init: {
    method: "GET" | "POST";
    headers: Record<string, string>;
    body?: string;
  };
}

export interface GmailOAuthTokens {
  accessToken: string;
  expiresInSeconds: number;
  refreshToken: string;
  scope: typeof GMAIL_READONLY_SCOPE;
  tokenType: "Bearer";
}

export interface GmailMailboxProfile {
  mailboxAddress: string;
}

export class GmailOAuthContractError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "GmailOAuthContractError";
  }
}

export async function createGmailOAuthSession(
  configuration: GmailOAuthClientConfiguration,
  crypto: GmailOAuthCrypto = defaultOAuthCrypto,
): Promise<GmailOAuthSession> {
  const clientId = requireNonEmptyString(
    configuration.clientId,
    "Gmail OAuth client ID is not configured.",
  );
  const loopbackPort = requireLoopbackPort(configuration.loopbackPort);
  const codeVerifier = encodeBase64Url(crypto.randomBytes(32));
  const state = encodeBase64Url(crypto.randomBytes(32));
  const codeChallenge = encodeBase64Url(await crypto.sha256(codeVerifier));
  const redirectUri = `http://127.0.0.1:${loopbackPort}`;
  const authorizationUrl = new URL(GOOGLE_AUTHORIZATION_ENDPOINT);

  authorizationUrl.searchParams.set("access_type", "offline");
  authorizationUrl.searchParams.set("client_id", clientId);
  authorizationUrl.searchParams.set("code_challenge", codeChallenge);
  authorizationUrl.searchParams.set("code_challenge_method", "S256");
  authorizationUrl.searchParams.set("prompt", "consent");
  authorizationUrl.searchParams.set("redirect_uri", redirectUri);
  authorizationUrl.searchParams.set("response_type", "code");
  authorizationUrl.searchParams.set("scope", GMAIL_READONLY_SCOPE);
  authorizationUrl.searchParams.set("state", state);

  return {
    authorizationUrl: authorizationUrl.toString(),
    clientId,
    codeVerifier,
    redirectUri,
    state,
  };
}

export function parseGmailOAuthCallback(
  session: GmailOAuthSession,
  callbackUrl: string,
): GmailOAuthCallback {
  let callback: URL;
  try {
    callback = new URL(callbackUrl);
  } catch {
    throw new GmailOAuthContractError(
      "Gmail authorization callback was invalid.",
    );
  }

  const expected = new URL(session.redirectUri);
  if (
    callback.origin !== expected.origin ||
    callback.pathname !== expected.pathname
  ) {
    throw new GmailOAuthContractError(
      "Gmail authorization callback was invalid.",
    );
  }

  const states = callback.searchParams.getAll("state");
  if (states.length !== 1 || states[0] !== session.state) {
    throw new GmailOAuthContractError(
      "Gmail authorization callback did not match this request.",
    );
  }

  if (callback.searchParams.has("error")) {
    throw new GmailOAuthContractError(
      "Gmail authorization was not completed.",
    );
  }

  const authorizationCodes = callback.searchParams.getAll("code");
  const authorizationCode = authorizationCodes[0];
  if (
    authorizationCodes.length !== 1 ||
    authorizationCode === undefined ||
    authorizationCode.trim().length === 0
  ) {
    throw new GmailOAuthContractError(
      "Gmail authorization callback did not contain a code.",
    );
  }

  return { authorizationCode };
}

export function buildGmailTokenExchangeRequest(
  session: GmailOAuthSession,
  authorizationCode: string,
): GmailHttpRequest {
  const code = requireNonEmptyString(
    authorizationCode,
    "Gmail authorization code was missing.",
  );
  const body = new URLSearchParams({
    client_id: session.clientId,
    code,
    code_verifier: session.codeVerifier,
    grant_type: "authorization_code",
    redirect_uri: session.redirectUri,
  });

  return {
    url: GOOGLE_TOKEN_ENDPOINT,
    init: {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: body.toString(),
    },
  };
}

export function parseGmailOAuthTokenResponse(
  value: unknown,
): GmailOAuthTokens {
  if (!isRecord(value)) {
    throw invalidTokenResponse();
  }

  const accessToken = value.access_token;
  const expiresInSeconds = value.expires_in;
  const refreshToken = value.refresh_token;
  const scope = value.scope;
  const tokenType = value.token_type;

  if (
    !isNonEmptyString(accessToken) ||
    !Number.isInteger(expiresInSeconds) ||
    (expiresInSeconds as number) <= 0 ||
    !isNonEmptyString(refreshToken) ||
    !hasOnlyReadOnlyScope(scope) ||
    tokenType !== "Bearer"
  ) {
    throw invalidTokenResponse();
  }

  return {
    accessToken,
    expiresInSeconds: expiresInSeconds as number,
    refreshToken,
    scope: GMAIL_READONLY_SCOPE,
    tokenType,
  };
}

export function buildGmailProfileRequest(
  accessToken: string,
): GmailHttpRequest {
  const token = requireNonEmptyString(
    accessToken,
    "Gmail access token was missing.",
  );

  return {
    url: GMAIL_PROFILE_ENDPOINT,
    init: {
      method: "GET",
      headers: {
        Authorization: `Bearer ${token}`,
      },
    },
  };
}

export function parseGmailProfileResponse(
  value: unknown,
): GmailMailboxProfile {
  if (!isRecord(value) || !isNonEmptyString(value.emailAddress)) {
    throw new GmailOAuthContractError(
      "Gmail profile response did not contain a mailbox address.",
    );
  }

  const mailboxAddress = value.emailAddress.trim().toLowerCase();
  if (
    mailboxAddress.startsWith("@") ||
    mailboxAddress.endsWith("@") ||
    mailboxAddress.split("@").length !== 2
  ) {
    throw new GmailOAuthContractError(
      "Gmail profile response did not contain a mailbox address.",
    );
  }

  return { mailboxAddress };
}

const defaultOAuthCrypto: GmailOAuthCrypto = {
  randomBytes(size) {
    return globalThis.crypto.getRandomValues(new Uint8Array(size));
  },
  async sha256(value) {
    const digest = await globalThis.crypto.subtle.digest(
      "SHA-256",
      new TextEncoder().encode(value),
    );
    return new Uint8Array(digest);
  },
};

function requireLoopbackPort(value: number): number {
  if (!Number.isInteger(value) || value < 1 || value > 65_535) {
    throw new GmailOAuthContractError(
      "Gmail OAuth requires a valid loopback port.",
    );
  }
  return value;
}

function requireNonEmptyString(value: unknown, message: string): string {
  if (!isNonEmptyString(value)) {
    throw new GmailOAuthContractError(message);
  }
  return value.trim();
}

function hasOnlyReadOnlyScope(value: unknown): boolean {
  if (!isNonEmptyString(value)) {
    return false;
  }
  const scopes = value.trim().split(/\s+/u);
  return scopes.length === 1 && scopes[0] === GMAIL_READONLY_SCOPE;
}

function invalidTokenResponse(): GmailOAuthContractError {
  return new GmailOAuthContractError(
    "Gmail token response did not satisfy the read-only OAuth contract.",
  );
}

function encodeBase64Url(value: Uint8Array): string {
  const binary = String.fromCharCode(...value);
  return btoa(binary)
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/u, "");
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && !Array.isArray(value) && typeof value === "object";
}
