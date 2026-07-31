import {
  executeGmailJsonRequest,
  GmailOAuthExecutionError,
  runPrivilegedGmailOAuth,
  type GmailOAuthLoopbackListener,
} from "./gmail-oauth-adapter";
import type { GmailOAuthCrypto } from "./gmail-oauth";
import {
  parseGmailOAuthWorkerCommand,
  persistenceMessage,
  type GmailOAuthAuthorizeCommand,
  type GmailOAuthHostMessage,
  type GmailOAuthWorkerMessage,
} from "./gmail-oauth-worker-protocol";

interface GmailOAuthWorkerOptions {
  crypto?: GmailOAuthCrypto;
  executeHttp?: typeof executeGmailJsonRequest;
}

type SendWorkerMessage = (message: GmailOAuthWorkerMessage) => void;

export async function runGmailOAuthWorker(
  lines: AsyncIterable<string>,
  send: SendWorkerMessage,
  options: GmailOAuthWorkerOptions = {},
): Promise<void> {
  const iterator = lines[Symbol.asyncIterator]();
  for (;;) {
    const next = await iterator.next();
    if (next.done) {
      return;
    }
    const command = parseLine(next.value);
    if (!command) {
      send({ type: "error", code: "invalid_command" });
      continue;
    }
    if (command.type === "shutdown") {
      return;
    }
    if (command.type !== "authorize") {
      send({ type: "error", code: "invalid_command" });
      continue;
    }
    await authorize(command, iterator, send, options);
  }
}

async function authorize(
  command: GmailOAuthAuthorizeCommand,
  iterator: AsyncIterator<string>,
  send: SendWorkerMessage,
  options: GmailOAuthWorkerOptions,
): Promise<void> {
  const bridge = new GmailOAuthHostBridge(command, iterator, send);
  try {
    const mailbox = await runPrivilegedGmailOAuth(command.clientId, {
      crypto: options.crypto,
      executeHttp: options.executeHttp ?? executeGmailJsonRequest,
      openExternalUrl: (authorizationUrl) =>
        bridge.openExternalUrl(authorizationUrl),
      openLoopbackListener: async () => bridge.loopbackListener(),
      persistAuthorizedMailbox: (authorizedMailbox) =>
        bridge.persistAuthorizedMailbox(authorizedMailbox),
    });
    send({
      type: "result",
      requestId: command.requestId,
      mailboxAddress: mailbox.mailboxAddress,
    });
  } catch (error) {
    send({
      type: "error",
      code:
        error instanceof GmailOAuthExecutionError &&
        error.message === "Gmail request failed."
          ? "gmail_request_failed"
          : "gmail_authorization_failed",
    });
  }
}

class GmailOAuthHostBridge {
  constructor(
    private readonly command: GmailOAuthAuthorizeCommand,
    private readonly iterator: AsyncIterator<string>,
    private readonly send: SendWorkerMessage,
  ) {}

  loopbackListener(): GmailOAuthLoopbackListener {
    return {
      port: this.command.loopbackPort,
      close: async () => {
        this.send({
          type: "close_listener",
          requestId: this.command.requestId,
        });
        await this.expect("closed");
      },
      waitForCallback: async () => {
        this.send({
          type: "wait_for_callback",
          requestId: this.command.requestId,
        });
        const response = await this.expect("callback");
        return response.callbackUrl;
      },
    };
  }

  async openExternalUrl(authorizationUrl: string): Promise<void> {
    this.send({
      type: "open_external_url",
      requestId: this.command.requestId,
      authorizationUrl,
    });
    await this.expect("opened");
  }

  async persistAuthorizedMailbox(
    mailbox: Parameters<typeof persistenceMessage>[1],
  ): Promise<void> {
    this.send(persistenceMessage(this.command.requestId, mailbox));
    await this.expect("persisted");
  }

  private async expect<T extends GmailOAuthHostMessage["type"]>(
    type: T,
  ): Promise<Extract<GmailOAuthHostMessage, { type: T }>> {
    const next = await this.iterator.next();
    if (next.done) {
      throw new Error("host disconnected");
    }
    const response = parseLine(next.value);
    if (
      !response ||
      response.type !== type ||
      response.requestId !== this.command.requestId
    ) {
      throw new Error("invalid host response");
    }
    return response as Extract<GmailOAuthHostMessage, { type: T }>;
  }
}

function parseLine(line: string) {
  try {
    return parseGmailOAuthWorkerCommand(JSON.parse(line));
  } catch {
    return undefined;
  }
}
