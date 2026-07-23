import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type {
  MoneySourceSummary,
  SourceDocumentRoutingOutcome,
  SourceDocumentSummary,
} from "./command-contracts";
import {
  VaultManualImportView,
  importNotice,
  parseReceivedAt,
  routingNotice,
  type VaultManualImportViewProps,
} from "./app";

const document: SourceDocumentSummary = {
  byteSize: 42,
  documentId: "document-1",
  documentStatus: "ready",
  fileState: "available",
  mimeType: "application/pdf",
  originalFilename: "June statement.pdf",
  receivedAt: "2026-07-19T00:00:00Z",
};

const source: MoneySourceSummary = {
  displayName: "Synthetic Bank",
  moneySourceId: "source-synthetic",
  sourceType: "bank",
};

const baseProps: VaultManualImportViewProps = {
  busy: false,
  deletingDocumentId: null,
  error: null,
  importing: false,
  loadingDocuments: false,
  normalizingDocumentId: null,
  notice: null,
  onCloseUnlock: () => undefined,
  onClosePreview: () => undefined,
  onCloseViewer: () => undefined,
  onDelete: () => undefined,
  onImport: () => undefined,
  onLock: () => undefined,
  onNormalize: () => undefined,
  onOpenUnlock: () => undefined,
  onRetryUnlockSources: () => undefined,
  onPasswordChange: () => undefined,
  onRefresh: () => undefined,
  onRememberedChange: () => undefined,
  onSelectMoneySource: () => undefined,
  onSaveRecoveryFile: () => undefined,
  onSaveSourceCopy: () => undefined,
  onSubmitPassword: () => undefined,
  onUnlockWithKeychain: () => undefined,
  onUnlockPasswordChange: () => undefined,
  onUnlockSourceChange: () => undefined,
  onUnlockSubmit: () => undefined,
  onView: () => undefined,
  onViewerPage: () => undefined,
  password: "",
  preview: null,
  rememberedOnThisMac: false,
  recoveryConfigured: false,
  savingRecoveryFile: false,
  savingCopyDocumentId: null,
  selectedMoneySourceId: null,
  sourceDocuments: [],
  unassignedDocuments: [],
  unlockingDocument: null,
  updatingRemembered: false,
  vaultStatus: "unlocked",
  viewer: null,
  viewingPage: false,
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

    const remembered = render({
      rememberedOnThisMac: true,
      vaultStatus: "locked",
    });
    expect(remembered).toContain("Unlock with this Mac");

    const unavailable = render({
      rememberedOnThisMac: null,
      vaultStatus: "locked",
    });
    expect(unavailable).toContain("Keychain unlock is unavailable");
  });

  it("renders an unlocked import path and an unassigned document", () => {
    const markup = render({ unassignedDocuments: [document] });

    expect(markup).toContain("Add file");
    expect(markup).toContain("Remember on this Mac");
    expect(markup).toContain("June statement.pdf");
    expect(markup).toContain("View document");
    expect(markup).toContain("Check routing");
    expect(markup).toContain("Delete source file");
    expect(markup).toContain("Save a copy");
  });

  it("shows routed documents under their safe Money Source display name", () => {
    const markup = render({
      sourceDocuments: [{
        documents: [{ ...document, documentId: "routed-document" }],
        source,
      }],
      selectedMoneySourceId: source.moneySourceId,
    });

    expect(markup).toContain("Synthetic Bank");
    expect(markup).toContain("Bank");
    expect(markup).toContain("1 document");
    expect(markup).toContain("June statement.pdf");
    expect(markup).not.toContain("Check routing");
  });

  it("groups unassigned evidence by received month, newest first", () => {
    const older: SourceDocumentSummary = {
      ...document,
      documentId: "document-2",
      originalFilename: "May statement.pdf",
      receivedAt: "2026-05-03T09:18:00Z",
    };
    const markup = render({ unassignedDocuments: [document, older] });

    expect(markup).toContain("July 2026");
    expect(markup).toContain("May 2026");
    expect(markup.indexOf("July 2026")).toBeLessThan(markup.indexOf("May 2026"));
    expect(markup).toContain("1 document");
  });

  it("treats SQLite import timestamps as UTC", () => {
    const receivedAt = "2026-07-19 00:00:00";
    const markup = render({
      unassignedDocuments: [{ ...document, receivedAt }],
    });

    expect(parseReceivedAt(receivedAt).toISOString()).toBe(
      "2026-07-19T00:00:00.000Z",
    );
    expect(markup).toContain("Added 19 Jul 2026");
  });

  it("keeps recovery as a non-blocking task until the file is saved", () => {
    const pending = render();
    expect(pending).toContain("To do");
    expect(pending).toContain("Save your recovery file");
    expect(pending).toContain("Save recovery file");

    const configured = render({ recoveryConfigured: true });
    expect(configured).not.toContain("Save your recovery file");
    expect(configured).toContain("Add a statement or export");
  });

  it("renders only page pixels and bounded viewer navigation", () => {
    const markup = render({
      viewer: {
        documentId: document.documentId,
        documentTitle: document.originalFilename,
        page: {
          pageCount: 2,
          pageNumber: 1,
          pngBase64: "cmVuZGVyZWQtcGFnZQ==",
        },
      },
    });

    expect(markup).toContain("data:image/png;base64,cmVuZGVyZWQtcGFnZQ==");
    expect(markup).toContain("Page 1 of 2");
    expect(markup).toContain("inert=\"\"");
    expect(markup).toContain("aria-hidden=\"true\"");
    expect(markup).not.toContain("%PDF");
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

    expect(routingNotice(routed, source)).toEqual({
      body: "CanCan matched this evidence to Synthetic Bank.",
      title: "Evidence routed",
      tone: "success",
    });
    expect(routingNotice(attention).title).toBe("Needs attention");
    expect(routingNotice(attention).body).not.toContain(attention.reason!);
  });
});
