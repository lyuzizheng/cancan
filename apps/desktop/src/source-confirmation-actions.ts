import type { SourceConfirmationPrompt } from "./command-contracts";
import type { Notice } from "./feedback";
import { commandErrorMessage, type VaultApi } from "./vault-api";

export interface SourceConfirmationActions {
  confirmSourceCandidate(
    prompt: SourceConfirmationPrompt,
    displayName: string,
    sourceType: string,
  ): Promise<void>;
  parkSourceCandidate(prompt: SourceConfirmationPrompt): Promise<void>;
}

export interface SourceConfirmationActionDeps {
  api: VaultApi;
  attentionBusyKey: string | null;
  loadDocuments: () => Promise<void>;
  loadFinanceData: () => Promise<unknown>;
  setAttentionBusyKey: (key: string | null) => void;
  setNotice: (notice: Notice) => void;
  vaultSessionId: { current: number };
}

/**
 * Orchestration for the source-confirmation card actions. Every action goes
 * through the owning host command with the candidate's expected version; on
 * any outcome the prompts and documents reload so a stale version fails
 * closed into fresh host state.
 */
export function createSourceConfirmationActions(
  deps: SourceConfirmationActionDeps,
): SourceConfirmationActions {
  const {
    api,
    attentionBusyKey,
    loadDocuments,
    loadFinanceData,
    setAttentionBusyKey,
    setNotice,
    vaultSessionId,
  } = deps;
  const reload = () => Promise.all([loadFinanceData(), loadDocuments()]).then(() => undefined);

  const confirmSourceCandidate: SourceConfirmationActions["confirmSourceCandidate"] = async (
    prompt,
    displayName,
    sourceType,
  ) => {
    if (attentionBusyKey !== null) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setAttentionBusyKey(`source-confirm:${prompt.candidateId}`);
    try {
      await api.confirmSourceCandidate(
        prompt.candidateId,
        prompt.version,
        displayName,
        sourceType,
      );
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setNotice({
        body: `CanCan routed the waiting evidence to ${displayName} and will keep future ${displayName} statements together.`,
        tone: "success",
        title: `${displayName} is ready`,
      });
      await reload();
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setNotice({
          body: commandErrorMessage(nextError),
          tone: "attention",
          title: "Couldn’t confirm that source",
        });
        await reload();
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setAttentionBusyKey(null);
      }
    }
  };

  const parkSourceCandidate: SourceConfirmationActions["parkSourceCandidate"] = async (prompt) => {
    if (attentionBusyKey !== null) {
      return;
    }
    const sessionId = vaultSessionId.current;
    setAttentionBusyKey(`source-park:${prompt.candidateId}`);
    try {
      await api.parkSourceCandidate(prompt.candidateId, prompt.version);
      if (vaultSessionId.current !== sessionId) {
        return;
      }
      setNotice({
        body: "The evidence stays parked under Sources. You can create or choose a source whenever you are ready.",
        tone: "success",
        title: "Kept unassigned",
      });
      await reload();
    } catch (nextError) {
      if (vaultSessionId.current === sessionId) {
        setNotice({
          body: commandErrorMessage(nextError),
          tone: "attention",
          title: "Couldn’t keep that source unassigned",
        });
        await reload();
      }
    } finally {
      if (vaultSessionId.current === sessionId) {
        setAttentionBusyKey(null);
      }
    }
  };

  return { confirmSourceCandidate, parkSourceCandidate };
}
