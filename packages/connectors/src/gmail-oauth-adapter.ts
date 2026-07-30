import {
  buildGmailProfileRequest,
  buildGmailTokenExchangeRequest,
  createGmailOAuthSession,
  GmailOAuthContractError,
  parseGmailOAuthCallback,
  parseGmailOAuthTokenResponse,
  parseGmailProfileResponse,
  type GmailHttpRequest,
  type GmailMailboxProfile,
  type GmailOAuthCrypto,
  type GmailOAuthTokens,
} from "./gmail-oauth";

export interface GmailOAuthLoopbackListener {
  port: number;
  close(): Promise<void>;
  waitForCallback(): Promise<string>;
}

export interface GmailAuthorizedMailbox {
  mailboxAddress: string;
  tokens: GmailOAuthTokens;
}

export interface GmailOAuthPrivilegedDependencies {
  crypto?: GmailOAuthCrypto;
  executeHttp(request: GmailHttpRequest): Promise<unknown>;
  openExternalUrl(authorizationUrl: string): Promise<void>;
  openLoopbackListener(): Promise<GmailOAuthLoopbackListener>;
  persistAuthorizedMailbox(mailbox: GmailAuthorizedMailbox): Promise<void>;
}

export class GmailOAuthExecutionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "GmailOAuthExecutionError";
  }
}

export async function runPrivilegedGmailOAuth(
  clientId: string,
  dependencies: GmailOAuthPrivilegedDependencies,
): Promise<GmailMailboxProfile> {
  let listener: GmailOAuthLoopbackListener;
  try {
    listener = await dependencies.openLoopbackListener();
  } catch {
    throw authorizationFailed();
  }

  let listenerClosed = false;
  try {
    const session = await createGmailOAuthSession(
      {
        clientId,
        loopbackPort: listener.port,
      },
      wrapInjectedCrypto(dependencies.crypto),
    );
    await runInjected(
      () => dependencies.openExternalUrl(session.authorizationUrl),
      authorizationFailed,
    );
    const callbackUrl = await runInjected(
      () => listener.waitForCallback(),
      authorizationFailed,
    );
    const callback = parseGmailOAuthCallback(session, callbackUrl);

    await runInjected(
      () => listener.close(),
      authorizationFailed,
    );
    listenerClosed = true;

    const tokens = parseGmailOAuthTokenResponse(
      await runInjected(
        () =>
          dependencies.executeHttp(
            buildGmailTokenExchangeRequest(
              session,
              callback.authorizationCode,
            ),
          ),
        requestFailed,
      ),
    );
    const mailbox = parseGmailProfileResponse(
      await runInjected(
        () =>
          dependencies.executeHttp(
            buildGmailProfileRequest(tokens.accessToken),
          ),
        requestFailed,
      ),
    );

    await runInjected(
      () =>
        dependencies.persistAuthorizedMailbox({
          mailboxAddress: mailbox.mailboxAddress,
          tokens,
        }),
      authorizationFailed,
    );
    return mailbox;
  } catch (error) {
    if (!listenerClosed) {
      try {
        await listener.close();
      } catch {
        // The original authorization failure remains the safe user-facing result.
      }
    }
    throw normalizeExecutionError(error);
  }
}

export async function executeGmailJsonRequest(
  request: GmailHttpRequest,
  fetchImpl: typeof fetch = globalThis.fetch,
): Promise<unknown> {
  let response: Response;
  try {
    response = await fetchImpl(request.url, request.init);
  } catch {
    throw requestFailed();
  }
  if (!response.ok) {
    throw requestFailed();
  }
  try {
    return await response.json();
  } catch {
    throw requestFailed();
  }
}

function normalizeExecutionError(error: unknown): Error {
  if (
    error instanceof GmailOAuthContractError ||
    error instanceof GmailOAuthExecutionError
  ) {
    return error;
  }
  return authorizationFailed();
}

function wrapInjectedCrypto(
  crypto: GmailOAuthCrypto | undefined,
): GmailOAuthCrypto | undefined {
  if (crypto === undefined) {
    return undefined;
  }
  return {
    randomBytes(size) {
      try {
        return crypto.randomBytes(size);
      } catch {
        throw authorizationFailed();
      }
    },
    async sha256(value) {
      return runInjected(() => crypto.sha256(value), authorizationFailed);
    },
  };
}

async function runInjected<T>(
  operation: () => Promise<T>,
  failure: () => GmailOAuthExecutionError,
): Promise<T> {
  try {
    return await operation();
  } catch {
    throw failure();
  }
}

function authorizationFailed(): GmailOAuthExecutionError {
  return new GmailOAuthExecutionError(
    "Gmail authorization could not be completed.",
  );
}

function requestFailed(): GmailOAuthExecutionError {
  return new GmailOAuthExecutionError("Gmail request failed.");
}
