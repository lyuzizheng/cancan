import { useCallback, useEffect, useRef, useState } from "react";

import type { IntakeNotificationSettings, TaskRow } from "./command-contracts";
import type { VaultApi } from "./vault-api";
import type { VaultSession } from "./use-vault-session";

export interface IntakeNotifications {
  readonly busy: boolean;
  readonly error: string | null;
  /** Re-reads the setting and the permission it currently stands on. */
  load(): void;
  /** Opens the macOS Notifications pane, where a refusal can be undone. */
  openSystemSettings(): void;
  /** Drops every session-scoped state; the gate's reset half. */
  reset(): void;
  /**
   * Bumped by every host click that reopened this renderer, so the window that
   * is already on screen pulls the pending route instead of waiting for an
   * unlock that will not come.
   */
  readonly routeRequest: number;
  /** Re-runs the command that produced `error`. */
  retry(): void;
  /** The user's toggle. Off is the absence of the status file, not a read failure. */
  setEnabled(enabled: boolean): void;
  readonly settings: IntakeNotificationSettings | null;
  /**
   * The Tasks row a menu-bar or notification click asked to open, or `null`.
   * Pulling consumes the route host-side, so one click opens one row once.
   */
  takeRoute(): Promise<TaskRow | null>;
}

/**
 * Owns the opt-in background intake notifications: the setting the user flips
 * in Settings, the system pane a refusal points at, and the route a
 * notification click left behind. The setting is durable host state, so this
 * hook only mirrors it; turning it on is the single moment macOS is asked for
 * permission, and a refused permission stays a truthful `denied` instead of an
 * error.
 */
export function useIntakeNotifications({
  api,
  session,
}: {
  api: VaultApi;
  session: VaultSession;
}): IntakeNotifications {
  const [settings, setSettings] = useState<IntakeNotificationSettings | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [routeRequest, setRouteRequest] = useState(0);
  // The read runs from an effect on unlock, so `load` must keep one identity
  // for the whole session: a `busy` state in its dependencies would re-trigger
  // the effect that called it. Re-entry is refused on the ref instead.
  const loading = useRef(false);
  // The command that produced the current error, so its own prompt can re-run
  // it rather than always re-reading the setting.
  const retrying = useRef<() => void>(() => undefined);

  const {
    isCurrent,
    runGuarded,
    sessionId: currentSessionId,
    status,
  } = session;

  const reset = useCallback(() => {
    loading.current = false;
    retrying.current = () => undefined;
    setSettings(null);
    setError(null);
    setBusy(false);
  }, []);

  const load = useCallback(() => {
    if (loading.current) {
      return;
    }
    loading.current = true;
    const sessionId = currentSessionId.current;
    setBusy(true);
    void runGuarded(async () => {
      const nextSettings = await api.intakeNotificationSettings();
      if (!isCurrent(sessionId)) {
        return;
      }
      setSettings(nextSettings);
      setError(null);
    }, {
      sessionId,
      onError: (message) => {
        retrying.current = load;
        setError(message);
      },
      onSettled: () => setBusy(false),
    }).then(() => {
      loading.current = false;
    });
  }, [api, currentSessionId, isCurrent, runGuarded]);

  const setEnabled = useCallback((enabled: boolean) => {
    if (busy) {
      return;
    }
    const sessionId = currentSessionId.current;
    setBusy(true);
    void runGuarded(async () => {
      const nextSettings = await api.setIntakeNotificationsEnabled(enabled);
      if (!isCurrent(sessionId)) {
        return;
      }
      setSettings(nextSettings);
      setError(null);
    }, {
      sessionId,
      onError: (message) => {
        retrying.current = () => setEnabled(enabled);
        setError(message);
      },
      onSettled: () => setBusy(false),
    });
  }, [api, busy, currentSessionId, isCurrent, runGuarded]);

  const openSystemSettings = useCallback(() => {
    const sessionId = currentSessionId.current;
    setBusy(true);
    void runGuarded(async () => {
      await api.openNotificationSettings();
      if (!isCurrent(sessionId)) {
        return;
      }
      setError(null);
    }, {
      sessionId,
      onError: (message) => {
        retrying.current = openSystemSettings;
        setError(message);
      },
      onSettled: () => setBusy(false),
    });
  }, [api, currentSessionId, isCurrent, runGuarded]);

  const takeRoute = useCallback(async () => {
    try {
      return await api.takeBackgroundIntakeRoute();
    } catch {
      // A pull that cannot read the projection yet (the Vault is still locked)
      // leaves the route pending host-side; a later pull opens it.
      return null;
    }
  }, [api]);

  const retry = useCallback(() => {
    retrying.current();
  }, []);

  // A click that finds this renderer already live (the window was never
  // destroyed, or the menu-bar item opened it first) has no unlock to pull the
  // route on, so the host's click event re-runs that pull.
  useEffect(() => {
    let active = true;
    let removeListener: (() => void) | undefined;
    void api
      .onBackgroundIntakeRoute(() => {
        if (active) {
          setRouteRequest((request) => request + 1);
        }
      })
      .then((remove) => {
        if (active) {
          removeListener = remove;
        } else {
          remove();
        }
      })
      .catch(() => {
        // The mount pull remains the fail-closed fallback.
      });

    return () => {
      active = false;
      removeListener?.();
    };
  }, [api]);

  // The setting lives outside the Vault's schema, so it is only readable once a
  // session can answer; the mount after every unlock refreshes it.
  useEffect(() => {
    if (status === "unlocked") {
      load();
    }
  }, [load, status]);

  return {
    busy,
    error,
    load,
    openSystemSettings,
    reset,
    retry,
    routeRequest,
    setEnabled,
    settings,
    takeRoute,
  };
}
