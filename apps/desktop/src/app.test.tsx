import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type {
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
} from "./command-contracts";
import {
  VaultManualImportView,
  importNotice,
  routingNotice,
  type VaultManualImportViewProps,
} from "./app";

const document: SourceDocumentSummary = {
  byteSize: 42,
  documentId: "document-1",
  fileState: "available",
  mimeType: "application/pdf",
  originalFilename: "June statement.pdf",
  receivedAt: "2026-07-19T00:00:00Z",
};

const baseProps: VaultManualImportViewProps = {
  busy: false,
  error: null,
  importing: false,
  loadingDocuments: false,
  normalizingDocumentId: null,
  notice: null,
  onImport: () => undefined,
  onLock: () => undefined,
  onNormalize: () => undefined,
  onPasswordChange: () => undefined,
  onRefresh: () => undefined,
  onSubmitPassword: () => undefined,
  password: "",
  unassignedDocuments: [],
  vaultStatus: "unlocked",
};

function render(props: Partial<VaultManualImportViewProps> = {}) {
  return renderToStaticMarkup(<VaultManualImportView {...baseProps} {...props} />);
}

describe("VaultManualImportView", () => {
  it("renders initial loading and locked Vault states", () => {
    expect(render({ busy: true, vaultStatus: "loading" })).toContain(
      "Checking your Vault",
    );
    const locked = render({ vaultStatus: "locked" });
    expect(locked).toContain("Unlock your Vault");
    expect(locked).not.toContain("Add file");
  });

  it("renders an unlocked import path and an unassigned document", () => {
    const markup = render({ unassignedDocuments: [document] });

    expect(markup).toContain("Add file");
    expect(markup).toContain("June statement.pdf");
    expect(markup).toContain("Check routing");
  });

  it("renders the active normalization state and friendly errors", () => {
    const normalizing = render({
      error: "Unlock your Vault to continue.",
      normalizingDocumentId: document.documentId,
      unassignedDocuments: [document],
    });

    expect(normalizing).toContain("Checking…");
    expect(normalizing).toContain("Unlock your Vault to continue.");
    expect(normalizing).toContain("Try again");
  });
});

describe("manual-import feedback", () => {
  it.each([
    ["cancelled", "No file was imported"],
    ["imported", "Added to your Vault"],
    ["already_present", "Already in CanCan"],
    ["restored", "Evidence restored"],
  ] as const)("maps %s import status to a clear outcome", (status, title) => {
    expect(importNotice(status).title).toBe(title);
  });

  it("confirms routing without inventing a Money Source name", () => {
    const routed: SourceDocumentRoutingOutcome = {
      accountIds: ["account-1"],
      documentId: document.documentId,
      moneySourceId: "source-1",
      reason: null,
      status: "routed",
    };
    const attention: SourceDocumentRoutingOutcome = {
      ...routed,
      moneySourceId: null,
      reason: "classification_uncertain",
      status: "needs_attention",
    };

    expect(routingNotice(routed).title).toBe("Evidence routed");
    expect(routingNotice(attention).title).toBe("Needs attention");
    expect(routingNotice(attention).body).not.toContain(attention.reason!);
  });
});
