import type { GmailAuthorizedMailbox } from "./gmail-oauth-adapter";

export interface GmailOAuthAuthorizeCommand {
  clientId: string;
  loopbackPort: number;
  requestId: string;
  type: "authorize";
}

export type GmailOAuthHostMessage =
  | { requestId: string; type: "callback"; callbackUrl: string }
  | { requestId: string; type: "closed" }
  | { requestId: string; type: "opened" }
  | { requestId: string; type: "persisted" };

export type GmailOAuthWorkerCommand =
  | GmailOAuthAuthorizeCommand
  | GmailOAuthHostMessage
  | { type: "shutdown" };

export type GmailOAuthWorkerMessage =
  | {
      environmentCleared: boolean;
      protocolVersion: 1;
      runtime: "gmail-oauth-v1";
      type: "ready";
    }
  | {
      authorizationUrl: string;
      requestId: string;
      type: "open_external_url";
    }
  | { requestId: string; type: "wait_for_callback" }
  | { requestId: string; type: "close_listener" }
  | {
      mailboxAddress: string;
      refreshToken: string;
      requestId: string;
      type: "persist";
    }
  | {
      mailboxAddress: string;
      requestId: string;
      type: "result";
    }
  | {
      code:
        | "gmail_authorization_failed"
        | "gmail_request_failed"
        | "invalid_command";
      type: "error";
    };

export function parseGmailOAuthWorkerCommand(
  value: unknown,
): GmailOAuthWorkerCommand | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  if (value.type === "shutdown" && hasExactKeys(value, ["type"])) {
    return { type: "shutdown" };
  }
  if (
    value.type === "authorize" &&
    hasExactKeys(value, ["clientId", "loopbackPort", "requestId", "type"]) &&
    isNonEmptyString(value.clientId) &&
    isLoopbackPort(value.loopbackPort) &&
    isNonEmptyString(value.requestId)
  ) {
    return value as unknown as GmailOAuthAuthorizeCommand;
  }
  if (
    value.type === "callback" &&
    hasExactKeys(value, ["callbackUrl", "requestId", "type"]) &&
    isNonEmptyString(value.callbackUrl) &&
    isNonEmptyString(value.requestId)
  ) {
    return value as unknown as GmailOAuthHostMessage;
  }
  if (
    (value.type === "closed" ||
      value.type === "opened" ||
      value.type === "persisted") &&
    hasExactKeys(value, ["requestId", "type"]) &&
    isNonEmptyString(value.requestId)
  ) {
    return value as unknown as GmailOAuthHostMessage;
  }
  return undefined;
}

export function persistenceMessage(
  requestId: string,
  mailbox: GmailAuthorizedMailbox,
): GmailOAuthWorkerMessage {
  return {
    type: "persist",
    requestId,
    mailboxAddress: mailbox.mailboxAddress,
    refreshToken: mailbox.tokens.refreshToken,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Record<string, unknown>,
  expectedKeys: string[],
): boolean {
  const keys = Object.keys(value).sort();
  return (
    keys.length === expectedKeys.length &&
    keys.every((key, index) => key === [...expectedKeys].sort()[index])
  );
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isLoopbackPort(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value >= 1 &&
    value <= 65_535
  );
}
