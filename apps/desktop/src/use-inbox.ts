import { useCallback, useState } from "react";

import type { LocalInboxStatus } from "./command-contracts";
import type { CommandWiring } from "./command-wiring";
import { localInboxScanSummaryText } from "./format";
import type { VaultApi } from "./vault-api";
import type { VaultSession } from "./use-vault-session";

export interface Inbox {
  readonly busy: boolean;
  cancelDisable(): void;
  choose(): void;
  readonly confirmingDisable: boolean;
  confirmDisable(): void;
  readonly error: string | null;
  receiveFailure(message: string): void;
  receiveStatus(status: LocalInboxStatus): void;
  requestDisable(): void;
  rescan(): void;
  /** Drops every session-scoped Inbox state; the gate's reset half. */
  reset(): void;
  readonly status: LocalInboxStatus | null;
}

/**
 * Owns the CanCan Inbox domain: the watched folder's status, its scan state,
 * and the choose, re-scan, and disable commands. The shared finance load
 * reports what it reads through `receiveStatus` / `receiveFailure`.
 */
export function useInbox({
  api,
  session,
  wiring,
}: {
  api: VaultApi;
  session: VaultSession;
  wiring: { current: CommandWiring };
}): Inbox {
  const [status, setStatus] = useState<LocalInboxStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmingDisable, setConfirmingDisable] = useState(false);

  const {
    isCurrent,
    runGuarded,
    sessionId: currentSessionId,
    setNotice,
  } = session;

  const receiveStatus = useCallback((nextStatus: LocalInboxStatus) => {
    setStatus(nextStatus);
    setError(null);
  }, []);

  const receiveFailure = useCallback((message: string) => {
    setStatus(null);
    setError(message);
  }, []);

  const reset = useCallback(() => {
    setStatus(null);
    setError(null);
    setBusy(false);
    setConfirmingDisable(false);
  }, []);

  const requestDisable = useCallback(() => setConfirmingDisable(true), []);
  const cancelDisable = useCallback(() => setConfirmingDisable(false), []);

  const choose = useCallback(() => {
    if (busy) {
      return;
    }
    const sessionId = currentSessionId.current;
    setBusy(true);
    void runGuarded(async () => {
      const nextStatus = await api.chooseLocalInboxRoot();
      if (!isCurrent(sessionId) || nextStatus === null) {
        return;
      }
      receiveStatus(nextStatus);
      setConfirmingDisable(false);
      setNotice({
        body: "New statements you save to Inbox are added for you. The folder stays outside your encrypted Vault.",
        tone: "success",
        title: "CanCan Inbox is on",
      });
      await wiring.current.loadFinanceData();
    }, { sessionId, onSettled: () => setBusy(false) });
  }, [api, busy, currentSessionId, isCurrent, receiveStatus, runGuarded, setNotice, wiring]);

  const rescan = useCallback(() => {
    if (busy) {
      return;
    }
    const sessionId = currentSessionId.current;
    setBusy(true);
    void runGuarded(async () => {
      const summary = await api.rescanLocalInbox();
      if (!isCurrent(sessionId)) {
        return;
      }
      setStatus((current) => current === null
        ? current
        : { ...current, lastScan: summary });
      setError(null);
      setNotice({
        body: localInboxScanSummaryText(summary),
        tone: "success",
        title: "Inbox checked",
      });
      await wiring.current.loadFinanceData();
    }, { sessionId, onSettled: () => setBusy(false) });
  }, [api, busy, currentSessionId, isCurrent, runGuarded, setNotice, wiring]);

  const confirmDisable = useCallback(() => {
    if (busy) {
      return;
    }
    const sessionId = currentSessionId.current;
    setBusy(true);
    void runGuarded(async () => {
      const nextStatus = await api.disableLocalInbox();
      if (!isCurrent(sessionId)) {
        return;
      }
      receiveStatus(nextStatus);
      setConfirmingDisable(false);
      setNotice({
        body: "Your Cancan folder and its files stay untouched. You can choose it again anytime.",
        tone: "success",
        title: "CanCan Inbox is off",
      });
    }, { sessionId, onSettled: () => setBusy(false) });
  }, [api, busy, currentSessionId, isCurrent, receiveStatus, runGuarded, setNotice]);

  return {
    busy,
    cancelDisable,
    choose,
    confirmDisable,
    confirmingDisable,
    error,
    receiveFailure,
    receiveStatus,
    requestDisable,
    rescan,
    reset,
    status,
  };
}
