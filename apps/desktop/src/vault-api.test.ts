import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import {
  commandErrorMessage,
  createVaultApi,
  type TauriInvoke,
  type TauriListen,
} from "./vault-api";

describe("Vault API", () => {
  it("allows only local and rendered in-memory images through the Tauri CSP", () => {
    const config = JSON.parse(
      readFileSync(
        new URL("../src-tauri/tauri.conf.json", import.meta.url),
        "utf8",
      ),
    ) as { app: { security: { csp: string } } };

    const imageSources = config.app.security.csp
      .split(";")
      .map((directive) => directive.trim())
      .find((directive) => directive.startsWith("img-src "));

    expect(imageSources).toBe("img-src 'self' data:");
  });

  it("uses only the established command names and safe request fields", async () => {
    const calls: Array<[string, Record<string, unknown> | undefined]> = [];
    const listCommands = new Set([
      "list_money_sources",
      "list_account_confirmation_prompts",
      "list_source_confirmation_prompts",
      "list_recent_activity",
      "audit_duplicate_committed_versions",
      "list_relationship_candidates",
      "list_review_items",
      "list_source_documents",
      "list_statement_password_sources",
      "list_unassigned_source_documents",
    ]);
    const invoke = (async (command, args) => {
      calls.push([command, args]);
      return listCommands.has(command) ? [] : null;
    }) as TauriInvoke;
    const listenedEvents: string[] = [];
    const subscribe = (async (event) => {
      listenedEvents.push(event);
      return () => undefined;
    }) as TauriListen;
    const api = createVaultApi(invoke, subscribe);

    await api.vaultStatus();
    await api.vaultAccessStatus();
    await api.listReviewItems();
    await api.getReviewDetail("review-1");
    await api.listRecentActivity();
    await api.auditDuplicateCommittedVersions();
    await api.getMoneyOverview();
    await api.listRelationshipCandidates("review-1", 3);
    await api.editReviewRecord("review-1", 3, {
      amountValue: "750.00",
    });
    await api.removeReviewRecord("review-1", 3);
    await api.acknowledgeReviewItem("review-1", 3);
    await api.acceptReviewRelationship("review-1", 3, "record-2", 1);
    await api.enqueueCommitReviewBatch(["review-1", "review-2"]);
    await api.getReviewJob("job-1");
    await api.undoCommittedEvent("event-1");
    await api.createVault("password");
    await api.chooseLocalInboxRoot();
    await api.localInboxStatus();
    await api.disableLocalInbox();
    await api.rescanLocalInbox();
    await api.unlockVault("password");
    await api.unlockVaultWithKeychain();
    await api.rememberVaultOnThisMac();
    await api.forgetVaultOnThisMac();
    await api.listStatementPasswordSources();
    await api.trySavedStatementPasswords("document-1");
    await api.unlockSourceDocument(
      "document-1",
      "source-dbs",
      "statement-password",
      true,
    );
    await api.removeStatementPassword("source-dbs");
    await api.lockVault();
    await api.operationalDiagnosticsPreview();
    await api.saveOperationalDiagnostics();
    await api.saveRecoveryFile();
    await api.saveSourceDocumentCopy("document-1");
    await api.deleteSourceDocument("document-1");
    await api.importSourceDocument();
    await api.listMoneySources();
    await api.listAccountConfirmationPrompts();
    await api.decideCandidateAccounts("source-dbs", "proposal-version-1", [
      { accountId: "account-1", action: "accept" },
      { accountId: "account-2", action: "dismiss" },
    ]);
    await api.restoreDismissedCandidateAccount("account-2");
    await api.listSourceConfirmationPrompts();
    await api.confirmSourceCandidate(
      "candidate-1",
      2,
      "DBS Bank",
      "bank_account",
    );
    await api.parkSourceCandidate("candidate-1", 3);
    await api.listSourceDocuments("source-dbs");
    await api.listUnassignedSourceDocuments();
    const removeVaultLockListener = await api.onVaultLocked(() => undefined);
    removeVaultLockListener();
    await api.reparseSourceDocument("document-1");
    await api.renderSourceDocumentPage("document-1", 2);
    await api.previewSourceDocument("document-1");
    await api.closeSourceDocumentView("document-1");

    expect(calls).toEqual([
      ["vault_status", undefined],
      ["vault_access_status", undefined],
      ["list_review_items", undefined],
      ["get_review_detail", { reviewItemId: "review-1" }],
      ["list_recent_activity", undefined],
      ["audit_duplicate_committed_versions", undefined],
      ["get_money_overview", undefined],
      [
        "list_relationship_candidates",
        { reviewItemId: "review-1", expectedRecordVersion: 3 },
      ],
      [
        "edit_review_record",
        {
          reviewItemId: "review-1",
          expectedRecordVersion: 3,
          amountValue: "750.00",
        },
      ],
      [
        "remove_review_record",
        { reviewItemId: "review-1", expectedRecordVersion: 3 },
      ],
      [
        "acknowledge_review_item",
        { reviewItemId: "review-1", expectedRecordVersion: 3 },
      ],
      [
        "accept_review_relationship",
        {
          reviewItemId: "review-1",
          expectedRecordVersion: 3,
          candidateRecordId: "record-2",
          expectedCandidateVersion: 1,
        },
      ],
      [
        "enqueue_commit_review_batch",
        { reviewItemIds: ["review-1", "review-2"] },
      ],
      ["get_review_job", { jobId: "job-1" }],
      ["undo_committed_event", { eventId: "event-1" }],
      ["create_vault", { password: "password" }],
      ["choose_local_inbox_root", undefined],
      ["local_inbox_status", undefined],
      ["disable_local_inbox", undefined],
      ["rescan_local_inbox", undefined],
      ["unlock_vault", { password: "password" }],
      ["unlock_vault_with_keychain", undefined],
      ["remember_vault_on_this_mac", undefined],
      ["forget_vault_on_this_mac", undefined],
      ["list_statement_password_sources", undefined],
      [
        "try_saved_statement_passwords",
        { documentId: "document-1" },
      ],
      [
        "unlock_source_document",
        {
          documentId: "document-1",
          moneySourceId: "source-dbs",
          password: "statement-password",
          updateSavedPassword: true,
        },
      ],
      ["remove_statement_password", { moneySourceId: "source-dbs" }],
      ["lock_vault", undefined],
      ["operational_diagnostics_preview", undefined],
      ["save_operational_diagnostics", undefined],
      ["save_recovery_file", undefined],
      ["save_source_document_copy", { documentId: "document-1" }],
      ["delete_source_document", { documentId: "document-1" }],
      ["import_source_document", undefined],
      ["list_money_sources", undefined],
      ["list_account_confirmation_prompts", undefined],
      [
        "decide_candidate_accounts",
        {
          request: {
            decisions: [
              { accountId: "account-1", action: "accept" },
              { accountId: "account-2", action: "dismiss" },
            ],
            moneySourceId: "source-dbs",
            proposalVersion: "proposal-version-1",
          },
        },
      ],
      ["restore_dismissed_candidate_account", { accountId: "account-2" }],
      ["list_source_confirmation_prompts", undefined],
      [
        "confirm_source_candidate",
        {
          request: {
            candidateId: "candidate-1",
            displayName: "DBS Bank",
            expectedVersion: 2,
            sourceType: "bank_account",
          },
        },
      ],
      [
        "park_source_candidate",
        { request: { candidateId: "candidate-1", expectedVersion: 3 } },
      ],
      ["list_source_documents", { moneySourceId: "source-dbs" }],
      ["list_unassigned_source_documents", undefined],
      ["reparse_source_document", { documentId: "document-1" }],
      ["render_source_document_page", { documentId: "document-1", pageNumber: 2 }],
      ["preview_source_document", { documentId: "document-1" }],
      ["close_source_document_view", { documentId: "document-1" }],
    ]);
    expect(listenedEvents).toEqual(["vault-locked"]);
  });

  it("maps command failures to safe user-facing copy", () => {
    expect(commandErrorMessage({ code: "vault_locked" })).toBe(
      "Unlock your Vault to continue.",
    );
    expect(commandErrorMessage({ code: "remembered_unlock_unavailable" })).toBe(
      "Touch ID unlock is no longer available. Use your Vault password instead.",
    );
    expect(commandErrorMessage({ code: "remember_failed" })).toBe(
      "CanCan couldn’t enable Touch ID unlock.",
    );
    expect(
      commandErrorMessage({ code: "statement_password_save_failed" }),
    ).toBe("CanCan couldn’t save that statement password in this Mac’s Keychain.");
    expect(
      commandErrorMessage({ code: "statement_password_remove_failed" }),
    ).toBe(
      "CanCan couldn’t remove that statement password from this Mac’s Keychain.",
    );
    expect(commandErrorMessage({ code: "recovery_status_failed" })).toBe(
      "The recovery file was saved, but CanCan couldn’t record setup. Keep the file private and try again.",
    );
    expect(commandErrorMessage({ code: "source_copy_location_invalid" })).toBe(
      "Save the copy somewhere outside your CanCan Vault.",
    );
    expect(commandErrorMessage({ code: "source_copy_save_failed" })).toBe(
      "CanCan couldn’t save a complete copy to that location.",
    );
    expect(commandErrorMessage({ code: "source_file_too_large" })).toBe(
      "Choose a file under 128 MB.",
    );
    expect(commandErrorMessage("private backend detail")).toBe(
      "Couldn’t complete that request. Try again.",
    );
    expect(commandErrorMessage({ code: "source_confirmation_unavailable" })).toBe(
      "CanCan couldn’t update that money source confirmation. Try again.",
    );
    expect(
      commandErrorMessage({ code: "invalid_source_confirmation_request" }),
    ).toBe("That money source confirmation isn’t valid.");
    expect(commandErrorMessage({ code: "diagnostics_unavailable" })).toBe(
      "CanCan couldn’t read the operational log. Try again.",
    );
    expect(commandErrorMessage({ code: "diagnostics_export_failed" })).toBe(
      "CanCan couldn’t write the diagnostics file to that location.",
    );
  });
});
