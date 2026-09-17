// @vitest-environment happy-dom

import { describe, expect, it, vi } from "vitest";

import {
  button,
  click,
  container,
  createApi,
  diagnosticsPreview,
  installAppHarness,
  mount,
} from "./test-support/app-harness";

installAppHarness();

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
