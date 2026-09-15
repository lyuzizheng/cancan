import type { ReviewItemSummary } from "./command-contracts";

/**
 * Cross-hook seam for the command center's read model.
 *
 * The vault session (declared first) and the domain hooks (declared after it)
 * need each other: the gate reloads documents and finance data when the Vault
 * unlocks, while the review, inbox, and attention actions reload finance data
 * after a mutation. `App` fills this object during render and every caller
 * reads it at call time, so no hook depends on a hook declared later in the
 * same component and no reloader becomes a render-changing dependency.
 */
export interface CommandWiring {
  /** Reloads Money Sources, the selected source's documents, and unassigned evidence. */
  loadDocuments(): Promise<void>;
  /** Reloads the whole finance read model and returns the review queue. */
  loadFinanceData(): Promise<ReviewItemSummary[] | null>;
  /** Drops every session-scoped domain — the gate's reset half. */
  resetSession(): void;
}

/** Placeholder wiring; every method is a no-op until `App` fills the real one. */
export const EMPTY_COMMAND_WIRING: CommandWiring = {
  loadDocuments: async () => {},
  loadFinanceData: async () => null,
  resetSession: () => {},
};
