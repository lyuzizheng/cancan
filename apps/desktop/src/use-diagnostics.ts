import { useCallback, useState } from "react";

import type { OperationalDiagnosticsPreview } from "./command-contracts";
import type { VaultApi } from "./vault-api";
import type { VaultSession } from "./use-vault-session";

export interface Diagnostics {
  readonly busy: boolean;
  dismissPreview(): void;
  readonly error: string | null;
  exportDiagnostics(): void;
  loadPreview(): void;
  readonly pending: "preview" | "export" | null;
  readonly preview: OperationalDiagnosticsPreview | null;
  /** Re-runs whichever command produced `error`. */
  retry(): void;
  /** Drops every session-scoped diagnostics state; the gate's reset half. */
  reset(): void;
}

/**
 * Owns the Settings diagnostics flow: the read-only preview of what an export
 * would contain, then the explicit user-confirmed save. Both commands run
 * under the session guard so a Vault lock mid-flight drops the outcome, and
 * failures stay inside the panel instead of the shared banner.
 */
export function useDiagnostics({
  api,
  session,
}: {
  api: VaultApi;
  session: VaultSession;
}): Diagnostics {
  const [preview, setPreview] = useState<OperationalDiagnosticsPreview | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<"preview" | "export" | null>(null);

  const { isCurrent, runGuarded, sessionId: currentSessionId, setNotice } =
    session;

  const reset = useCallback(() => {
    setPreview(null);
    setError(null);
    setPending(null);
  }, []);

  const dismissPreview = useCallback(() => {
    setPreview(null);
    setError(null);
  }, []);

  const loadPreview = useCallback(() => {
    if (pending !== null) {
      return;
    }
    const sessionId = currentSessionId.current;
    setPending("preview");
    void runGuarded(async () => {
      const nextPreview = await api.operationalDiagnosticsPreview();
      if (!isCurrent(sessionId)) {
        return;
      }
      setPreview(nextPreview);
      setError(null);
    }, {
      sessionId,
      onError: setError,
      onSettled: () => setPending(null),
    });
  }, [api, currentSessionId, isCurrent, pending, runGuarded]);

  const exportDiagnostics = useCallback(() => {
    if (pending !== null) {
      return;
    }
    const sessionId = currentSessionId.current;
    setPending("export");
    void runGuarded(async () => {
      const saved = await api.saveOperationalDiagnostics();
      if (!isCurrent(sessionId)) {
        return;
      }
      setError(null);
      if (saved) {
        setPreview(null);
        setNotice({
          body: "The file holds only redacted operational entries — no document content, amounts, filenames, or paths.",
          tone: "success",
          title: "Diagnostics exported",
        });
      } else {
        setNotice({
          body: "No file was written. Your operational log stays in the Vault.",
          tone: "attention",
          title: "Export cancelled",
        });
      }
    }, {
      sessionId,
      onError: setError,
      onSettled: () => setPending(null),
    });
  }, [api, currentSessionId, isCurrent, pending, runGuarded, setNotice]);

  const retry = useCallback(() => {
    // A failure with a preview on screen came from the export; without one it
    // came from the preview load itself.
    if (preview === null) {
      loadPreview();
    } else {
      exportDiagnostics();
    }
  }, [exportDiagnostics, loadPreview, preview]);

  return {
    busy: pending !== null,
    dismissPreview,
    error,
    exportDiagnostics,
    loadPreview,
    pending,
    preview,
    retry,
    reset,
  };
}
