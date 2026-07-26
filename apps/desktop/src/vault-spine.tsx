import type { ReactNode } from "react";

import type { VaultStatus } from "./command-contracts";

export type VaultScreenStatus = VaultStatus | "loading";

export type AppView = "overview" | "sources" | "review";

export interface VaultSpineProps {
  activeView: AppView;
  inert: boolean;
  onNavigate: (view: AppView) => void;
  reviewCount: number | null;
  vaultStatus: VaultScreenStatus;
}

export function VaultSpine(props: VaultSpineProps) {
  return (
    <aside
      aria-hidden={props.inert ? true : undefined}
      className="vault-spine"
      inert={props.inert}
      aria-label="CanCan Vault"
    >
      <div className="vault-brand">
        <span className="vault-mark" aria-hidden="true">C</span>
        <span>CanCan</span>
      </div>
      <nav className="vault-nav" aria-label="Command Center">
        <p className="vault-nav-label">Command Center</p>
        <ul className="vault-nav-list">
          <NavEntry
            active={props.activeView === "overview"}
            icon="overview"
            label="Overview"
            onNavigate={() => props.onNavigate("overview")}
          />
          <NavEntry
            active={props.activeView === "sources"}
            icon="sources"
            label="Sources"
            onNavigate={() => props.onNavigate("sources")}
          />
          <li><span className="vault-nav-item vault-nav-upcoming"><NavIcon name="assets" />Assets</span></li>
          <li><span className="vault-nav-item vault-nav-upcoming"><NavIcon name="transactions" />Transactions</span></li>
          <NavEntry
            active={props.activeView === "review"}
            count={props.reviewCount}
            icon="review"
            label="Review"
            onNavigate={() => props.onNavigate("review")}
          />
          <li><span className="vault-nav-item vault-nav-upcoming"><NavIcon name="money-flow" />Money Flow</span></li>
          <li><span className="vault-nav-item vault-nav-upcoming"><NavIcon name="jobs" />Jobs</span></li>
          <li><span className="vault-nav-item vault-nav-upcoming"><NavIcon name="settings" />Settings</span></li>
        </ul>
      </nav>
      <div className="vault-assistant">
        <span className="vault-nav-item vault-nav-upcoming"><NavIcon name="assistant" />AI Assistant</span>
      </div>
      <div className="vault-spine-status">
        <p className="vault-spine-label">Local Vault</p>
        <p className="vault-spine-state">
          <span className={`vault-status-light vault-status-${props.vaultStatus}`} aria-hidden="true" />
          {vaultStatusLabel(props.vaultStatus)}
        </p>
        <p className="vault-spine-copy">Evidence stays encrypted on this Mac.</p>
      </div>
      <p className="vault-spine-footnote">Local-first finance</p>
    </aside>
  );
}

function NavEntry({
  active,
  count,
  icon,
  label,
  onNavigate,
}: {
  active: boolean;
  count?: number | null;
  icon: NavIconName;
  label: string;
  onNavigate: () => void;
}) {
  return (
    <li>
      <button
        aria-current={active ? "page" : undefined}
        className={`vault-nav-item${active ? " vault-nav-item-active" : ""}`}
        onClick={onNavigate}
        type="button"
      >
        <NavIcon name={icon} />
        {label}
        {count !== null && count !== undefined && count > 0 ? (
          <span className="vault-nav-count" aria-label={`${count} to review`}>{count}</span>
        ) : null}
        {active ? <span className="vault-nav-dot" aria-hidden="true" /> : null}
      </button>
    </li>
  );
}

function vaultStatusLabel(status: VaultScreenStatus) {
  return status === "unlocked"
    ? "Vault unlocked"
    : status === "locked"
    ? "Vault locked"
    : status === "not_created"
    ? "Vault setup needed"
    : "Checking Vault";
}

type NavIconName =
  | "overview"
  | "sources"
  | "assets"
  | "transactions"
  | "review"
  | "money-flow"
  | "jobs"
  | "settings"
  | "assistant";

export function NavIcon({ name }: { name: NavIconName }) {
  const shapes: Record<NavIconName, ReactNode> = {
    overview: (
      <>
        <rect x="3" y="3" width="5" height="5" rx="1" />
        <rect x="10" y="3" width="5" height="5" rx="1" />
        <rect x="3" y="10" width="5" height="5" rx="1" />
        <rect x="10" y="10" width="5" height="5" rx="1" />
      </>
    ),
    sources: (
      <>
        <rect x="3" y="3" width="12" height="12" rx="2" />
        <path d="M3 8h12" />
      </>
    ),
    assets: (
      <>
        <circle cx="9" cy="9" r="6" />
        <path d="M9 3v6l4.2 2.4" />
      </>
    ),
    transactions: (
      <>
        <path d="M3 6h10" />
        <path d="M10 3l3 3-3 3" />
        <path d="M15 12H5" />
        <path d="M8 9l-3 3 3 3" />
      </>
    ),
    review: (
      <>
        <rect x="3" y="3" width="12" height="12" rx="2" />
        <path d="M6 9.2l2.2 2.2 4-4.4" />
      </>
    ),
    "money-flow": <path d="M3 13.5l3.8-3.8 3 3 5.2-5.7" />,
    jobs: <path d="M5 4.5h8M5 9h8M5 13.5h5" />,
    settings: (
      <>
        <circle cx="9" cy="9" r="2.2" />
        <path d="M9 3v2.1M9 12.9V15M3 9h2.1M12.9 9H15M5.2 5.2l1.5 1.5M11.3 11.3l1.5 1.5M12.8 5.2l-1.5 1.5M6.7 11.3l-1.5 1.5" />
      </>
    ),
    assistant: (
      <path d="M4 3.5h10a1.5 1.5 0 0 1 1.5 1.5v6a1.5 1.5 0 0 1-1.5 1.5H8l-4.5 3v-12a1.5 1.5 0 0 1 .5-1z" />
    ),
  };
  return (
    <svg
      aria-hidden="true"
      className="vault-nav-icon"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.5"
      viewBox="0 0 18 18"
    >
      {shapes[name]}
    </svg>
  );
}
