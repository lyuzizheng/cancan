import type { ReactNode } from "react";

import { Badge } from "../primitives/badge";
import { StatusPoint, type StatusPointTone } from "./status-point";

export interface SectionHeaderProps {
  title: ReactNode;
  tone?: StatusPointTone;
  /** Optional count rendered as a pill badge in the tone's color. */
  count?: number;
  /** Right-aligned slot for a quiet action (View all, Refresh, …). */
  action?: ReactNode;
}

/**
 * Section heading — status point + title + optional count/action. The
 * Command Center's zone rhythm element; titles stay in Geologica medium
 * (the serif display role never touches UI chrome).
 */
export function SectionHeader({ title, tone = "healthy", count, action }: SectionHeaderProps) {
  const badgeTone = tone === "attention" ? "attention" : tone === "healthy" ? "healthy" : "neutral";
  return (
    <div className="flex items-center gap-2">
      <StatusPoint tone={tone} />
      <h2 className="text-lg font-medium text-ledger-ink">{title}</h2>
      {count !== undefined && <Badge tone={badgeTone}>{count}</Badge>}
      {action !== undefined && <div className="ml-auto">{action}</div>}
    </div>
  );
}
