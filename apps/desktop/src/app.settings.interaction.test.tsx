// @vitest-environment happy-dom

import { act } from "react";
import { describe, expect, it, vi } from "vitest";

import type { IntakeNotificationSettings } from "./command-contracts";
import {
  button,
  click,
  container,
  createApi,
  diagnosticsPreview,
  installAppHarness,
  mount,
  settle,
} from "./test-support/app-harness";

installAppHarness();

describe("App settings background intake notifications", () => {
  function notificationToggle(): HTMLInputElement {
    const labels = [...container.querySelectorAll("label")].filter((element) =>
      element.textContent?.includes("Background intake notifications")
    );
    expect(labels).toHaveLength(1);
    const input = labels[0]!.querySelector<HTMLInputElement>(
      'input[type="checkbox"]',
    );
    expect(input).not.toBeNull();
    return input!;
  }

  async function toggleNotifications() {
    await act(async () => {
      notificationToggle().click();
      await settle();
    });
  }

  it("starts off, asks the host once, and shows the opted-in state", async () => {
    const api = createApi();
    await mount(api, "settings");

    expect(api.intakeNotificationSettings).toHaveBeenCalledTimes(1);
    expect(notificationToggle().checked).toBe(false);
    expect(api.setIntakeNotificationsEnabled).not.toHaveBeenCalled();

    await toggleNotifications();

    expect(api.setIntakeNotificationsEnabled).toHaveBeenCalledWith(true);
    expect(notificationToggle().checked).toBe(true);
    expect(container.textContent).not.toContain("isn’t allowing notifications");
  });

  it("keeps the switch on when macOS refuses permission, and points at the pane", async () => {
    const api = createApi({
      setIntakeNotificationsEnabled: vi.fn(async (): Promise<IntakeNotificationSettings> => ({
        enabled: true,
        permission: "denied",
      })),
    });
    await mount(api, "settings");
    await toggleNotifications();

    // The user's intent stands; the guidance names the system setting instead of
    // reporting a failure, because nothing failed.
    expect(notificationToggle().checked).toBe(true);
    expect(container.textContent).toContain("Notifications blocked by macOS");

    await click("Open System Settings");

    expect(api.openNotificationSettings).toHaveBeenCalledTimes(1);
    expect(container.textContent).not.toContain("Try again");
  });

  it("reports a rejected write and re-applies the toggle on retry", async () => {
    const api = createApi({
      setIntakeNotificationsEnabled: vi
        .fn()
        .mockRejectedValueOnce({ code: "intake_notification_setting_failed" })
        .mockResolvedValue({ enabled: true, permission: "authorized" }),
    });
    await mount(api, "settings");
    await toggleNotifications();

    expect(container.textContent).toContain(
      "CanCan couldn’t save that notification setting.",
    );
    expect(notificationToggle().checked).toBe(false);

    await click("Try again");

    expect(api.setIntakeNotificationsEnabled).toHaveBeenCalledTimes(2);
    expect(notificationToggle().checked).toBe(true);
    expect(container.textContent).not.toContain("couldn’t save");
  });
});

describe("App settings diagnostics export", () => {
  it("previews the export content before saving it", async () => {
    const api = createApi();
    await mount(api, "settings");

    expect(api.operationalDiagnosticsPreview).not.toHaveBeenCalled();
    await click("Preview export");

    expect(api.operationalDiagnosticsPreview).toHaveBeenCalledTimes(1);
    expect(api.saveOperationalDiagnostics).not.toHaveBeenCalled();
    expect(container.textContent).toContain("2");
    expect(container.textContent).toContain("runtime.documents");
    expect(container.textContent).toContain("import_failed");
    // Sample lines render verbatim — the export's own redacted content.
    expect(container.textContent).toContain(
      "2026-09-16T08:00:00Z ERROR component=runtime.documents code=import_failed",
    );

    await click("Export diagnostics");

    expect(api.saveOperationalDiagnostics).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("Diagnostics exported");
  });

  it("keeps the preview open when the save dialog is dismissed", async () => {
    const api = createApi({
      saveOperationalDiagnostics: vi.fn(async () => false),
    });
    await mount(api, "settings");
    await click("Preview export");
    await click("Export diagnostics");

    expect(container.textContent).toContain("Export cancelled");
    expect(container.textContent).toContain(
      "2026-09-16T08:00:00Z ERROR component=runtime.documents code=import_failed",
    );
    expect(button("Export diagnostics")).toBeDefined();
  });

  it("shows the mapped error and retries the preview", async () => {
    const api = createApi({
      operationalDiagnosticsPreview: vi
        .fn()
        .mockRejectedValueOnce({ code: "diagnostics_unavailable" })
        .mockResolvedValue(diagnosticsPreview),
    });
    await mount(api, "settings");
    await click("Preview export");

    expect(container.textContent).toContain(
      "CanCan couldn’t read the operational log. Try again.",
    );

    await click("Try again");

    expect(api.operationalDiagnosticsPreview).toHaveBeenCalledTimes(2);
    expect(container.textContent).toContain("Export diagnostics");
  });

  it("keeps the preview and retries the export after a write failure", async () => {
    const api = createApi({
      saveOperationalDiagnostics: vi
        .fn()
        .mockRejectedValueOnce({ code: "diagnostics_export_failed" })
        .mockResolvedValue(true),
    });
    await mount(api, "settings");
    await click("Preview export");
    await click("Export diagnostics");

    expect(container.textContent).toContain(
      "CanCan couldn’t write the diagnostics file to that location.",
    );
    expect(container.textContent).toContain("runtime.documents");

    await click("Try again");

    expect(api.saveOperationalDiagnostics).toHaveBeenCalledTimes(2);
    expect(api.operationalDiagnosticsPreview).toHaveBeenCalledTimes(1);
    expect(container.textContent).toContain("Diagnostics exported");
  });
});
