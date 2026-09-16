// @vitest-environment happy-dom

import { act } from "react";
import { createRoot } from "react-dom/client";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type {
  MoneySourceSummary,
  SourceDocumentSummary,
} from "./command-contracts";
import { DocumentViewer } from "./document-modals";
import { parseReceivedAt } from "./evidence-documents";
import { importNotice } from "./notices";
import { SourcesView, type SourcesViewProps } from "./sources-view";
import { VaultGate } from "./vault-gate";

const document: SourceDocumentSummary = {
  attentionReason: null,
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

const baseProps: SourcesViewProps = {
  accountPrompts: [],
  attentionBusyKey: null,
  busy: false,
  existingSources: [],
  importing: false,
  inbox: null,
  inboxError: null,
  inboxBusy: false,
  inboxConfirmingDisable: false,
  loadingDocuments: false,
  normalizingDocumentId: null,
  notice: null,
  onConfirmSourceCandidate: () => undefined,
  onDecideAccounts: () => undefined,
  onImport: () => undefined,
  onInboxCancelDisable: () => undefined,
  onInboxChoose: () => undefined,
  onInboxConfirmDisable: () => undefined,
  onInboxRequestDisable: () => undefined,
  onInboxRescan: () => undefined,
  onInboxRetry: () => undefined,
  onKeepSourceCandidateUnassigned: () => undefined,
  onLock: () => undefined,
  onNormalize: () => undefined,
  onOpenUnlock: () => undefined,
  onRefresh: () => undefined,
  onRememberedChange: () => undefined,
  onRequestDelete: () => undefined,
  onRestoreAccount: () => undefined,
  onSelectMoneySource: () => undefined,
  onSaveRecoveryFile: () => undefined,
  onSaveSourceCopy: () => undefined,
  onView: () => undefined,
  onViewPromptDocument: () => undefined,
  recoveryConfigured: false,
  rememberedOnThisMac: false,
  savingCopyDocumentId: null,
  savingRecoveryFile: false,
  selectedMoneySourceId: null,
  sourceDocuments: [],
  sourcePrompts: [],
  unassignedDocuments: [],
  updatingRemembered: false,
};

function render(props: Partial<SourcesViewProps> = {}) {
  return renderToStaticMarkup(<SourcesView {...baseProps} {...props} />);
}

describe("VaultGate", () => {
  it("renders initial loading and locked Vault states", () => {
    expect(
      renderToStaticMarkup(
        <VaultGate busy title="Checking your Vault" body="Confirming the local Vault state before showing evidence." />,
      ),
    ).toContain("Checking your Vault");

    const locked = renderToStaticMarkup(
      <VaultGate
        busy={false}
        body="Unlock your local Vault to add a file or check its routing."
        onPasswordChange={() => undefined}
        onSubmit={() => undefined}
        password=""
        title="Unlock your Vault"
      />,
    );
    expect(locked).toContain("Unlock your Vault");
    expect(locked).not.toContain("Add file");

    const remembered = renderToStaticMarkup(
      <VaultGate
        busy={false}
        body="Unlock your local Vault to add a file or check its routing."
        onPasswordChange={() => undefined}
        onSubmit={() => undefined}
        onUnlockWithKeychain={() => undefined}
        password=""
        rememberedOnThisMac
        title="Unlock your Vault"
      />,
    );
    expect(remembered).toContain("Unlock with Touch ID");

    const unavailable = renderToStaticMarkup(
      <VaultGate
        busy={false}
        body="Unlock your local Vault to add a file or check its routing."
        onPasswordChange={() => undefined}
        onSubmit={() => undefined}
        password=""
        rememberedOnThisMac={null}
        title="Unlock your Vault"
      />,
    );
    expect(unavailable).toContain("Touch ID unlock is unavailable");
  });
});

describe("SourcesView", () => {
  it("renders an unlocked import path and an unassigned document", () => {
    const markup = render({ unassignedDocuments: [document] });

    expect(markup).toContain("Add file");
    expect(markup).toContain("Unlock with Touch ID");
    expect(markup).toContain("June statement.pdf");
    expect(markup).toContain("View document");
    expect(markup).toContain("Re-run parser");
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

  it("keeps available processing and failed evidence viewable", () => {
    const processing = {
      ...document,
      documentId: "processing-document",
      documentStatus: "processing" as const,
      originalFilename: "Processing.pdf",
    };
    const failed = {
      ...document,
      attentionReason: "classification_failed",
      documentId: "failed-document",
      documentStatus: "needs_attention" as const,
      originalFilename: "Failed.pdf",
    };
    const markup = render({ unassignedDocuments: [processing, failed] });

    expect(markup).toContain("Processing");
    expect(markup.match(/View document/g)).toHaveLength(2);
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
    expect(configured).toContain("No Money Sources yet");
  });

  it("renders the active normalization state", () => {
    const normalizing = render({
      normalizingDocumentId: document.documentId,
      unassignedDocuments: [document],
    });

    expect(normalizing).toContain("Re-running…");
  });
});

describe("DocumentViewer", () => {
  it("renders only page pixels and bounded viewer navigation", async () => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    const host = globalThis.document.createElement("div");
    globalThis.document.body.append(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(
        <DocumentViewer
          onClose={() => undefined}
          onPage={() => undefined}
          viewer={{
            documentId: document.documentId,
            documentTitle: document.originalFilename,
            page: {
              pageCount: 2,
              pageNumber: 1,
              pngBase64: "cmVuZGVyZWQtcGFnZQ==",
            },
          }}
          viewingPage={false}
        />,
      );
    });

    const dialog = globalThis.document.body.querySelector('[role="dialog"]');
    expect(dialog?.querySelector("img")?.getAttribute("src")).toBe(
      "data:image/png;base64,cmVuZGVyZWQtcGFnZQ==",
    );
    expect(dialog?.textContent).toContain("Page 1 of 2");
    expect(dialog?.textContent).not.toContain("%PDF");

    await act(async () => root.unmount());
    host.remove();
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
});
