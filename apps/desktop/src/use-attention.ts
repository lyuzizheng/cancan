import { useCallback, useMemo, useState } from "react";

import type {
  AccountConfirmationPrompt,
  CandidateAccountDecisionInput,
  SourceConfirmationPrompt,
} from "./command-contracts";
import type { CommandWiring } from "./command-wiring";
import { createSourceConfirmationActions } from "./source-confirmation-actions";
import type { VaultApi } from "./vault-api";
import type { VaultSession } from "./use-vault-session";

export interface Attention {
  readonly accountPrompts: AccountConfirmationPrompt[] | null;
  readonly attentionBusyKey: string | null;
  confirmSourceCandidate(
    prompt: SourceConfirmationPrompt,
    displayName: string,
    sourceType: string,
  ): Promise<void>;
  decideAccounts(
    prompt: AccountConfirmationPrompt,
    decisions: CandidateAccountDecisionInput[],
  ): void;
  readonly focusedCandidateId: string | null;
  parkSourceCandidate(prompt: SourceConfirmationPrompt): Promise<void>;
  receivePrompts(
    accounts: AccountConfirmationPrompt[],
    sourceConfirmations: SourceConfirmationPrompt[],
  ): void;
  /** Drops every session-scoped attention state; the gate's reset half. */
  reset(): void;
  restoreAccount(accountId: string): void;
  setFocusedCandidateId: (candidateId: string | null) => void;
  readonly sourcePrompts: SourceConfirmationPrompt[] | null;
}

/**
 * Owns the evidence that needs a decision: account-confirmation proposals,
 * source-confirmation prompts, and the parked-evidence focus. The accepting and
 * parking commands live in `source-confirmation-actions.ts`.
 */
export function useAttention({
  api,
  loadDocuments,
  session,
  wiring,
}: {
  api: VaultApi;
  loadDocuments: () => Promise<void>;
  session: VaultSession;
  wiring: { current: CommandWiring };
}): Attention {
  const [accountPrompts, setAccountPrompts] = useState<
    AccountConfirmationPrompt[] | null
  >(null);
  const [sourcePrompts, setSourcePrompts] = useState<
    SourceConfirmationPrompt[] | null
  >(null);
  const [attentionBusyKey, setAttentionBusyKey] = useState<string | null>(null);
  const [focusedCandidateId, setFocusedCandidateId] = useState<string | null>(null);

  const {
    isCurrent,
    runGuarded,
    sessionId,
    setNotice,
  } = session;

  const reloadFinance = useCallback(
    () => wiring.current.loadFinanceData(),
    [wiring],
  );

  const receivePrompts = useCallback((
    accounts: AccountConfirmationPrompt[],
    sourceConfirmations: SourceConfirmationPrompt[],
  ) => {
    setAccountPrompts(accounts);
    setSourcePrompts(sourceConfirmations);
  }, []);

  const reset = useCallback(() => {
    setAccountPrompts(null);
    setSourcePrompts(null);
    setAttentionBusyKey(null);
    setFocusedCandidateId(null);
  }, []);

  const decideAccounts = useCallback((
    prompt: AccountConfirmationPrompt,
    decisions: CandidateAccountDecisionInput[],
  ) => {
    if (attentionBusyKey !== null) {
      return;
    }
    const startedAt = sessionId.current;
    setAttentionBusyKey(`account:${prompt.moneySourceId}`);
    void runGuarded(async () => {
      const outcome = await api.decideCandidateAccounts(
        prompt.moneySourceId,
        prompt.proposalVersion,
        decisions,
      );
      if (!isCurrent(startedAt)) {
        return;
      }
      if (outcome.status === "conflict") {
        setNotice({
          body: "CanCan reloaded the latest account list. Check it and save your choices again.",
          tone: "attention",
          title: "That account list changed",
        });
      } else {
        setNotice({
          body: outcome.status === "already_confirmed" ? "These account choices were already saved." : "Accepted accounts can enter review. Dismissed records remain in history.",
          tone: "success",
          title: outcome.status === "already_confirmed" ? "Account choices already saved" : "Account choices saved",
        });
      }
      await reloadFinance();
    }, {
      sessionId: startedAt,
      onError: async (message) => {
        setNotice({
          body: message,
          tone: "attention",
          title: "Couldn’t save those account choices",
        });
        await reloadFinance();
      },
      onSettled: () => setAttentionBusyKey(null),
    });
  }, [api, attentionBusyKey, isCurrent, reloadFinance, runGuarded, sessionId, setNotice]);

  const restoreAccount = useCallback((accountId: string) => {
    if (attentionBusyKey !== null) {
      return;
    }
    const startedAt = sessionId.current;
    setAttentionBusyKey(`restore:${accountId}`);
    void runGuarded(async () => {
      const outcome = await api.restoreDismissedCandidateAccount(accountId);
      if (!isCurrent(startedAt)) {
        return;
      }
      setNotice(outcome.status === "restored"
        ? {
            body: "The account is back in review with its latest records.",
            tone: "success",
            title: "Account restored",
          }
        : {
            body: "CanCan reloaded the latest account list.",
            tone: "attention",
            title: "That account changed",
          });
      await reloadFinance();
    }, {
      sessionId: startedAt,
      onError: (message) => setNotice({
        body: message,
        tone: "attention",
        title: "Couldn’t restore that account",
      }),
      onSettled: () => setAttentionBusyKey(null),
    });
  }, [api, attentionBusyKey, isCurrent, reloadFinance, runGuarded, sessionId, setNotice]);

  const sourceConfirmation = useMemo(
    () => createSourceConfirmationActions({
      api,
      attentionBusyKey,
      loadDocuments,
      loadFinanceData: reloadFinance,
      setAttentionBusyKey,
      setNotice,
      vaultSessionId: sessionId,
    }),
    [api, attentionBusyKey, loadDocuments, reloadFinance, sessionId, setNotice],
  );

  const updateFocusedCandidateId = useCallback(
    (candidateId: string | null) => setFocusedCandidateId(candidateId),
    [],
  );

  return {
    accountPrompts,
    attentionBusyKey,
    confirmSourceCandidate: sourceConfirmation.confirmSourceCandidate,
    decideAccounts,
    focusedCandidateId,
    parkSourceCandidate: sourceConfirmation.parkSourceCandidate,
    receivePrompts,
    reset,
    restoreAccount,
    setFocusedCandidateId: updateFocusedCandidateId,
    sourcePrompts,
  };
}
