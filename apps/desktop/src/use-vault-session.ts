import { useCallback, useEffect, useRef, useState } from "react";

import { commandErrorMessage, type VaultApi } from "./vault-api";
import type { Notice } from "./feedback";
import type { VaultScreenStatus } from "./vault-spine";
import type { CommandWiring } from "./command-wiring";

/** How a single command reports the vault session it belongs to. */
export interface CommandRunOptions {
  /**
   * Runs as a command-center command: raises the shared busy flag and clears
   * the error banner before the command starts, and releases the flag when it
   * finishes. Omit for a domain command that carries its own in-flight flag.
   */
  busy?: boolean;
  /**
   * Vault-session epoch captured before the command started. When the session
   * changed meanwhile, the failure and the settle are dropped so a locked Vault
   * never sees a stale command's outcome.
   */
  sessionId?: number;
  /** Receives the user-facing failure message instead of the shared banner. */
  onError?: (message: string) => Promise<void> | void;
  /** Releases the caller's own in-flight flag once the command has settled. */
  onSettled?: () => void;
}

export interface VaultSession {
  readonly busy: boolean;
  /** False while the Vault is locked; gates every document and finance load. */
  readonly documentsAllowed: { current: boolean };
  readonly error: string | null;
  /** Bumps on every gate transition; work from an older epoch must not write state. */
  readonly sessionId: { current: number };
  readonly notice: Notice | null;
  readonly password: string;
  readonly recoveryConfigured: boolean;
  readonly rememberedOnThisMac: boolean | null;
  readonly status: VaultScreenStatus;
  /** True after a password unlock while Touch ID is off; the offer is dismissible. */
  readonly touchIdOffer: boolean;
  isCurrent(sessionId: number): boolean;
  unlockWithKeychain(): void;
  lockVault(): Promise<boolean | undefined>;
  refresh(): Promise<void>;
  run(action: () => Promise<void>, sessionId?: number): Promise<void>;
  runGuarded<T>(
    action: () => Promise<T>,
    options?: CommandRunOptions,
  ): Promise<T | null>;
  setError(message: string | null): void;
  setNotice(notice: Notice | null): void;
  setPassword(password: string): void;
  setRecoveryConfigured(configured: boolean): void;
  setRememberedOnThisMac(remembered: boolean | null): void;
  /** Dismisses the post-password Touch ID offer without enabling it. */
  dismissTouchIdOffer(): void;
  /** Closes the session for a gate screen and returns the new session epoch. */
  showGate(nextStatus: VaultScreenStatus): number;
  unlock(): void;
}

/**
 * Owns the Vault session: the gate screen, the session epoch every other hook
 * guards against, the shared busy flag and error banner, and the lock, unlock,
 * and refresh flows. `App` owns the navigation state itself.
 */
export function useVaultSession({
  api,
  wiring,
}: {
  api: VaultApi;
  wiring: { current: CommandWiring };
}): VaultSession {
  const [status, setStatus] = useState<VaultScreenStatus>("loading");
  const [busy, setBusy] = useState(true);
  const [password, setPassword] = useState("");
  const [rememberedOnThisMac, setRememberedOnThisMac] = useState<boolean | null>(
    false,
  );
  const [touchIdOffer, setTouchIdOffer] = useState(false);
  const [recoveryConfigured, setRecoveryConfigured] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [error, setError] = useState<string | null>(null);
  const sessionId = useRef(0);
  const documentsAllowed = useRef(false);

  useEffect(() => () => {
    sessionId.current += 1;
  }, []);

  const isCurrent = useCallback(
    (candidate: number) => sessionId.current === candidate,
    [],
  );

  /**
   * Runs one command under the session guard: the failure and the settle are
   * reported only while the session that started the command is still live.
   */
  const runGuarded = useCallback(async <T,>(
    action: () => Promise<T>,
    options: CommandRunOptions = {},
  ): Promise<T | null> => {
    const { busy: showsBusy = false, onError, onSettled, sessionId: startedAt } = options;
    const settles = () => startedAt === undefined || sessionId.current === startedAt;
    if (showsBusy) {
      setBusy(true);
      setError(null);
    }
    try {
      return await action();
    } catch (nextError) {
      if (settles()) {
        // A handler that reloads (the attention cards do) is awaited; a plain
        // setter stays synchronous, so no failure path gains an extra hop.
        const reported = (onError ?? setError)(commandErrorMessage(nextError));
        if (reported) {
          await reported;
        }
      }
      return null;
    } finally {
      if (settles()) {
        onSettled?.();
        if (showsBusy) {
          setBusy(false);
        }
      }
    }
  }, []);

  const run = useCallback(
    async (action: () => Promise<void>, startedAt?: number) => {
      await runGuarded(action, { busy: true, sessionId: startedAt });
    },
    [runGuarded],
  );

  const showGate = useCallback((nextStatus: VaultScreenStatus) => {
    const nextSessionId = sessionId.current + 1;
    sessionId.current = nextSessionId;
    documentsAllowed.current = false;
    setStatus(nextStatus);
    setError(null);
    setNotice(null);
    setPassword("");
    setTouchIdOffer(false);
    wiring.current.resetSession();
    setBusy(false);
    return nextSessionId;
  }, [wiring]);

  const dismissTouchIdOffer = useCallback(() => setTouchIdOffer(false), []);

  const loadAllData = useCallback(() => Promise.all([
    wiring.current.loadDocuments(),
    wiring.current.loadFinanceData(),
  ]), [wiring]);

  const requestVaultLock = useCallback(async () => {
    const startedAt = showGate("loading");
    try {
      const nextStatus = await api.lockVault();
      if (sessionId.current === startedAt) {
        documentsAllowed.current = nextStatus === "unlocked";
        setStatus(nextStatus);
        return nextStatus === "unlocked";
      }
    } catch (nextError) {
      if (sessionId.current !== startedAt) {
        return false;
      }
      try {
        const nextStatus = await api.vaultStatus();
        if (sessionId.current !== startedAt) {
          return false;
        }
        documentsAllowed.current = nextStatus === "unlocked";
        setStatus(nextStatus);
        if (nextStatus === "unlocked") {
          setError(commandErrorMessage(nextError));
          await loadAllData();
          return sessionId.current === startedAt;
        }
      } catch {
        if (sessionId.current === startedAt) {
          setError(commandErrorMessage(nextError));
        }
      }
    }
    return false;
  }, [api, loadAllData, showGate]);

  useEffect(() => {
    let active = true;
    let removeListener: (() => void) | undefined;
    void api.onVaultLocked(() => {
      if (active) {
        showGate("locked");
      }
    }).then((remove) => {
      if (active) {
        removeListener = remove;
      } else {
        remove();
      }
    }).catch(() => {
      // Focus and visibility reconciliation remain the fail-closed fallback.
    });

    return () => {
      active = false;
      removeListener?.();
    };
  }, [api, showGate]);

  const refresh = useCallback(async () => {
    const startedAt = sessionId.current;
    setBusy(true);
    try {
      const access = await api.vaultAccessStatus();
      if (sessionId.current !== startedAt) {
        return;
      }
      const nextStatus = access.status;
      documentsAllowed.current = nextStatus === "unlocked";
      setRememberedOnThisMac(access.rememberedOnThisMac);
      setRecoveryConfigured(access.recoveryConfigured);
      setStatus(nextStatus);
      if (nextStatus === "unlocked") {
        await loadAllData();
      } else {
        showGate(nextStatus);
      }
    } catch (nextError) {
      if (sessionId.current === startedAt) {
        setError(commandErrorMessage(nextError));
      }
    } finally {
      if (sessionId.current === startedAt) {
        setBusy(false);
      }
    }
  }, [api, loadAllData, showGate]);

  useEffect(() => {
    const reconcileAfterFocus = () => {
      if (document.visibilityState === "visible") {
        void refresh();
      }
    };
    window.addEventListener("focus", reconcileAfterFocus);
    document.addEventListener("visibilitychange", reconcileAfterFocus);
    return () => {
      window.removeEventListener("focus", reconcileAfterFocus);
      document.removeEventListener("visibilitychange", reconcileAfterFocus);
    };
  }, [refresh]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const finalizeUnlock = useCallback(async (nextStatus: VaultScreenStatus) => {
    documentsAllowed.current = nextStatus === "unlocked";
    setStatus(nextStatus);
    setPassword("");
    if (nextStatus === "unlocked") {
      await loadAllData();
    }
  }, [loadAllData]);

  const submitPassword = useCallback(() => {
    if (!password) {
      setError("Enter a password to continue.");
      return;
    }

    const startedAt = sessionId.current;
    void run(async () => {
      const nextStatus =
        status === "not_created"
          ? await api.createVault(password)
          : await api.unlockVault(password);
      if (sessionId.current === startedAt) {
        await finalizeUnlock(nextStatus);
        // A password unlock while Touch ID is off earns one dismissible
        // re-enable offer (spec 0009); `null` means the Keychain read failed,
        // so there is nothing reliable to offer.
        if (
          status === "locked"
          && nextStatus === "unlocked"
          && rememberedOnThisMac === false
        ) {
          setTouchIdOffer(true);
        }
      }
    }, startedAt);
  }, [api, finalizeUnlock, password, rememberedOnThisMac, run, status]);

  const unlockWithKeychain = useCallback(() => {
    const startedAt = sessionId.current;
    void run(async () => {
      try {
        const nextStatus = await api.unlockVaultWithKeychain();
        if (sessionId.current === startedAt) {
          await finalizeUnlock(nextStatus);
        }
      } catch (nextError) {
        if (sessionId.current !== startedAt) {
          return;
        }
        try {
          const access = await api.vaultAccessStatus();
          if (sessionId.current === startedAt) {
            setRememberedOnThisMac(access.rememberedOnThisMac);
          }
        } catch {
          // Keep the original Keychain error as the user-facing outcome.
        }
        throw nextError;
      }
    }, startedAt);
  }, [api, finalizeUnlock, run]);

  return {
    busy,
    documentsAllowed,
    error,
    isCurrent,
    lockVault: requestVaultLock,
    notice,
    password,
    recoveryConfigured,
    refresh,
    rememberedOnThisMac,
    run,
    runGuarded,
    sessionId,
    setError,
    setNotice,
    setPassword,
    setRecoveryConfigured,
    setRememberedOnThisMac,
    dismissTouchIdOffer,
    touchIdOffer,
    showGate,
    status,
    unlock: submitPassword,
    unlockWithKeychain,
  };
}
