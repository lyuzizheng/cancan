import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { LocalInboxStatus } from "./command-contracts";
import { InboxPanel, type InboxPanelProps } from "./inbox";

const enabledStatus: LocalInboxStatus = {
  accessState: "enabled",
  backupsPrepared: false,
  enabled: true,
  inboxLabel: "Inbox",
  lastScan: {
    alreadyPresent: 12,
    deferred: 1,
    imported: 3,
    suppressed: 2,
  },
};

const baseProps: InboxPanelProps = {
  busy: false,
  confirmingDisable: false,
  error: null,
  onCancelDisable: () => undefined,
  onChoose: () => undefined,
  onConfirmDisable: () => undefined,
  onRequestDisable: () => undefined,
  onRescan: () => undefined,
  onRetry: () => undefined,
  status: enabledStatus,
};

function render(props: Partial<InboxPanelProps> = {}): string {
  return renderToStaticMarkup(<InboxPanel {...baseProps} {...props} />);
}

describe("InboxPanel", () => {
  it("shows a checking state while the host status loads", () => {
    const html = render({ status: null });
    expect(html).toContain("Checking CanCan Inbox…");
    expect(html).toContain("role=\"status\"");
  });

  it("offers folder selection when the inbox is disabled", () => {
    const html = render({
      status: {
        accessState: "disabled",
        backupsPrepared: false,
        enabled: false,
        inboxLabel: "Inbox",
        lastScan: null,
      },
    });
    expect(html).toContain("Add statements without opening CanCan");
    expect(html).toContain("Choose Cancan folder");
    expect(html).toContain("Backups is never read for statements");
    expect(html).not.toContain("Turn off");
  });

  it("shows the on state with the sanitized last-scan summary", () => {
    const html = render();
    expect(html).toContain("CanCan Inbox is on");
    expect(html).toContain(
      "Last check: 3 added; 12 already in CanCan; 1 to try again later; 2 kept deleted.",
    );
    expect(html).toContain("Check now");
    expect(html).toContain("Turn off");
    expect(html).not.toContain("Backups folder is prepared");
  });

  it("mentions the prepared Backups folder only when the host prepared it", () => {
    const html = render({ status: { ...enabledStatus, backupsPrepared: true } });
    expect(html).toContain("A Backups folder is prepared next to Inbox");
  });

  it("explains the paused state while the vault is locked", () => {
    const html = render({
      status: { ...enabledStatus, accessState: "paused" },
    });
    expect(html).toContain("CanCan Inbox is paused");
    expect(html).toContain("resumes when your Vault is unlocked");
  });

  it("asks for the folder again when access needs reauthorization", () => {
    const html = render({
      status: {
        ...enabledStatus,
        accessState: "needs_reauthorization",
        lastScan: null,
      },
    });
    expect(html).toContain("CanCan can’t reach your Cancan folder");
    expect(html).toContain("Nothing was moved or deleted");
    expect(html).toContain("Choose again");
  });

  it("explains the needs-attention state without blaming the user", () => {
    const html = render({
      status: { ...enabledStatus, accessState: "needs_attention" },
    });
    expect(html).toContain("CanCan Inbox needs attention");
    expect(html).toContain("CanCan never changes existing files");
  });

  it("requires an explicit second step before turning off", () => {
    const html = render({ confirmingDisable: true });
    expect(html).toContain("Turn off CanCan Inbox? Your folder and files stay untouched.");
    expect(html).toContain("Keep");
    expect(html).not.toContain("Check now");
  });

  it("shows busy labels and disables actions while a host call runs", () => {
    const html = render({ busy: true });
    expect(html).toContain("Checking…");
    expect((html.match(/disabled=""/g) ?? []).length).toBeGreaterThanOrEqual(2);
  });
});
