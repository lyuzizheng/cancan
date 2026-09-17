import {
  cx,
  Icon,
  StatusPoint,
  type IconName,
  type StatusPointTone,
} from "@cancan/ui";

import type { VaultStatus } from "./command-contracts";

export type VaultScreenStatus = VaultStatus | "loading";

export type AppView = "overview" | "sources" | "review" | "tasks" | "settings";

export interface VaultSpineProps {
  activeView: AppView;
  inert: boolean;
  onNavigate: (view: AppView) => void;
  reviewCount: number | null;
  tasksCount: number | null;
  vaultStatus: VaultScreenStatus;
}

/**
 * Vault Spine — the obsidian source index rail. Chrome stays in Geologica
 * (the serif display role never touches the spine); go-bright is reserved
 * for the brand mark and the active dot, the two dark-surface accent moments.
 * Below md it collapses to a top bar with a horizontal nav strip so
 * navigation is never lost.
 */
export function VaultSpine(props: VaultSpineProps) {
  return (
    <aside
      aria-hidden={props.inert ? true : undefined}
      className="flex shrink-0 flex-col border-vault-seam bg-vault-obsidian px-3.5 pt-5 pb-4 text-vault-text max-md:w-full max-md:border-b max-md:pt-4 md:min-h-screen md:w-60 md:border-r"
      inert={props.inert}
      aria-label="CanCan Vault"
    >
      <div className="flex items-center gap-2.5 px-2.5 max-md:px-0">
        <span
          aria-hidden="true"
          className="grid size-7 shrink-0 place-items-center rounded-sm border border-vault-seam bg-vault-graphite font-mono text-sm font-medium text-accent-go-bright"
        >
          C
        </span>
        <span className="text-lg font-medium tracking-tight">CanCan</span>
      </div>

      <nav
        aria-label="Command Center"
        className="max-md:-mx-3.5 max-md:mt-3 max-md:overflow-x-auto max-md:px-3.5 max-md:pb-1 md:mt-7"
      >
        <p className="mb-2 px-2.5 font-mono text-xs uppercase tracking-mono-label text-vault-text-muted max-md:hidden">
          Command Center
        </p>
        <ul className="m-0 flex list-none gap-1 p-0 md:grid md:gap-0.5">
          <NavEntry
            active={props.activeView === "overview"}
            icon="overview"
            label="Overview"
            onNavigate={() => props.onNavigate("overview")}
          />
          <NavEntry
            active={props.activeView === "tasks"}
            count={props.tasksCount}
            countLabel={(count) => `${count} ${count === 1 ? "needs" : "need"} action`}
            icon="jobs"
            label="Tasks"
            onNavigate={() => props.onNavigate("tasks")}
          />
          <NavEntry
            active={props.activeView === "sources"}
            icon="sources"
            label="Sources"
            onNavigate={() => props.onNavigate("sources")}
          />
          <UpcomingEntry icon="assets" label="Assets" />
          <UpcomingEntry icon="transactions" label="Transactions" />
          <NavEntry
            active={props.activeView === "review"}
            count={props.reviewCount}
            countLabel={(count) => `${count} to review`}
            icon="review"
            label="Review"
            onNavigate={() => props.onNavigate("review")}
          />
          <UpcomingEntry icon="money-flow" label="Money Flow" />
          <NavEntry
            active={props.activeView === "settings"}
            icon="settings"
            label="Settings"
            onNavigate={() => props.onNavigate("settings")}
          />
        </ul>
      </nav>

      <div className="mt-auto border-t border-vault-seam pt-3 max-md:hidden">
        <span className="flex h-9 items-center gap-2.5 rounded-sm px-2.5 text-base font-medium text-vault-text-muted opacity-50">
          <Icon name="assistant" size={16} />
          AI Assistant
        </span>
      </div>

      <div className="border-t border-vault-seam max-md:mt-2 max-md:flex max-md:items-center max-md:gap-2.5 max-md:pt-2.5 md:mt-3.5 md:px-2.5 md:pt-3.5">
        <p className="font-mono text-xs uppercase tracking-mono-label text-vault-text-muted">
          Local Vault
        </p>
        <p className="flex items-center gap-2 text-sm font-medium text-vault-text md:mt-2">
          <StatusPoint
            className={props.vaultStatus === "loading" ? "animate-pulse-bounded motion-reduce:animate-none" : undefined}
            tone={statusTone(props.vaultStatus)}
          />
          {vaultStatusLabel(props.vaultStatus)}
        </p>
        <p className="mt-1.5 text-xs text-vault-text-muted max-md:hidden">
          Evidence stays encrypted on this Mac.
        </p>
      </div>

      <p className="mt-3.5 border-t border-vault-seam px-2.5 pt-3 text-xs text-vault-text-muted max-md:hidden">
        Local-first finance
      </p>
    </aside>
  );
}

function NavEntry({
  active,
  count,
  countLabel,
  icon,
  label,
  onNavigate,
}: {
  active: boolean;
  count?: number | null;
  /** Screen-reader phrase for the count pill (the visible digit is aria-hidden). */
  countLabel?: (count: number) => string;
  icon: IconName;
  label: string;
  onNavigate: () => void;
}) {
  const hasCount = count !== null && count !== undefined && count > 0;
  return (
    <li className="max-md:flex-none">
      <button
        aria-current={active ? "page" : undefined}
        className={cx(
          "flex h-9 items-center gap-2.5 whitespace-nowrap rounded-sm border bg-transparent px-2.5 text-base font-medium transition-colors duration-120 ease-mech md:w-full",
          "focus-visible:outline-2 focus-visible:outline-accent-go-bright focus-visible:outline-offset-2",
          active
            ? "border-vault-seam bg-vault-graphite text-vault-text"
            : "border-transparent text-vault-text-muted hover:text-vault-text",
        )}
        onClick={onNavigate}
        type="button"
      >
        <Icon name={icon} size={16} />
        {label}
        {hasCount || active ? (
          <span className="ml-2 flex items-center gap-2 md:ml-auto">
            {hasCount ? (
              <span
                className={cx(
                  "grid h-5 min-w-5 place-items-center rounded-pill border bg-vault-graphite px-1.5 font-mono text-xs text-signal-amber",
                  active ? "border-signal-amber" : "border-vault-seam",
                )}
              >
                <span aria-hidden="true">{count}</span>
                <span className="sr-only">{countLabel ? countLabel(count) : `${count} items`}</span>
              </span>
            ) : null}
            {active ? (
              <span aria-hidden="true" className="size-1.5 rounded-pill bg-accent-go-bright" />
            ) : null}
          </span>
        ) : null}
      </button>
    </li>
  );
}

function UpcomingEntry({ icon, label }: { icon: IconName; label: string }) {
  return (
    <li className="max-md:flex-none">
      <span className="flex h-9 items-center gap-2.5 whitespace-nowrap rounded-sm px-2.5 text-base font-medium text-vault-text-muted opacity-50 md:w-full">
        <Icon name={icon} size={16} />
        {label}
      </span>
    </li>
  );
}

function statusTone(status: VaultScreenStatus): StatusPointTone {
  return status === "unlocked" ? "healthy" : "attention";
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
