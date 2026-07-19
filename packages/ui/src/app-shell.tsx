import type { ReactNode } from "react";

export interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  return <main className="vault-app">{children}</main>;
}
