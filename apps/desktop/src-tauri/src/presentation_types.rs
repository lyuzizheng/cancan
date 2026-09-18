//! Test-only harness that emits TypeScript for every type crossing the Tauri
//! command bridge.
//!
//! The renderer previously hand-wrote mirrors of these types in
//! `src/command-contracts.ts`, which had no drift protection. Every wire type
//! now derives `ts_rs::TS` under `cfg(test)` and is listed here, so the checked
//! in `src/generated/presentation-types.ts` is the single source of truth and
//! [`generated_presentation_types_are_current`] fails when it goes stale.
//!
//! Adding a wire type means adding its `#[cfg_attr(test, derive(ts_rs::TS))]`
//! and its `decl` entry below; regenerate with
//! `pnpm --filter @cancan/desktop generate:presentation-types`.

#[cfg(test)]
fn generated_presentation_types() -> String {
    use ts_rs::{Config, TS};

    let config = Config::default().with_large_int("number");
    let declarations = [
        // Vault lifecycle.
        crate::runtime::VaultAccessStatus::decl(&config),
        crate::runtime::VaultStatus::decl(&config),
        crate::runtime::SavedStatementPasswordResult::decl(&config),
        // Local Inbox.
        crate::runtime::LocalInboxAccessState::decl(&config),
        crate::runtime::LocalInboxScanSummary::decl(&config),
        crate::runtime::LocalInboxStatus::decl(&config),
        // Source documents and statements.
        crate::database::SourceDocumentImportOutcome::decl(&config),
        crate::database::SourceDocumentImportStatus::decl(&config),
        crate::runtime::MoneySourceSummary::decl(&config),
        crate::runtime::SourceDocumentPreview::decl(&config),
        crate::runtime::SourceDocumentStatus::decl(&config),
        crate::runtime::SourceDocumentSummary::decl(&config),
        crate::runtime::StatementPasswordSourceSummary::decl(&config),
        crate::viewer::RenderedDocumentPage::decl(&config),
        // Operational diagnostics.
        crate::diagnostics::OperationalDiagnosticsCategory::decl(&config),
        crate::diagnostics::OperationalDiagnosticsPreview::decl(&config),
        // Background intake notifications.
        crate::runtime::IntakeNotificationPermission::decl(&config),
        crate::runtime::IntakeNotificationSettings::decl(&config),
        // Phone capture.
        crate::phone_shortcut::PhoneShortcutInboxCheck::decl(&config),
        crate::phone_shortcut::PhoneShortcutInboxCheckState::decl(&config),
        crate::runtime::PhoneShortcutStatus::decl(&config),
        // Tasks.
        crate::runtime::TaskConsequence::decl(&config),
        crate::runtime::TaskDestination::decl(&config),
        crate::runtime::TaskFilter::decl(&config),
        crate::runtime::TaskGroup::decl(&config),
        crate::runtime::TaskRow::decl(&config),
        crate::runtime::Tasks::decl(&config),
        // Account confirmation.
        crate::database::AccountConfirmationCandidate::decl(&config),
        crate::database::AccountConfirmationOutcome::decl(&config),
        crate::database::AccountConfirmationPrompt::decl(&config),
        crate::database::AccountConfirmationStatus::decl(&config),
        crate::database::CandidateAccountDecision::decl(&config),
        crate::database::CandidateAccountDecisionInput::decl(&config),
        crate::runtime::DecideCandidateAccountsRequest::decl(&config),
        // Money source confirmation.
        crate::database::intake::ConfirmedMoneySourceCandidate::decl(&config),
        crate::database::intake::MoneySourceCandidateState::decl(&config),
        crate::database::intake::MoneySourceCandidateStatus::decl(&config),
        crate::database::intake::SourceConfirmationPrompt::decl(&config),
        crate::database::intake::SourceConfirmationPromptStatus::decl(&config),
        crate::database::intake::SourceConfirmationScopeKind::decl(&config),
        crate::runtime::ConfirmSourceCandidateRequest::decl(&config),
        crate::runtime::ParkSourceCandidateRequest::decl(&config),
        // Review queue and ledger evidence.
        crate::database::DuplicateCommittedVersionAuditRow::decl(&config),
        crate::database::MoneyOverview::decl(&config),
        crate::database::MoneyOverviewAmount::decl(&config),
        crate::database::RecentActivitySummary::decl(&config),
        crate::database::RelationshipCandidateSummary::decl(&config),
        crate::database::ReviewBatchGroupOutcome::decl(&config),
        crate::database::ReviewBatchGroupStatus::decl(&config),
        crate::database::ReviewItemDetail::decl(&config),
        crate::database::ReviewItemSummary::decl(&config),
        crate::database::ReviewJobStatus::decl(&config),
        crate::database::ReviewJobSummary::decl(&config),
        crate::database::ReviewMutationOutcome::decl(&config),
        crate::database::ReviewMutationStatus::decl(&config),
        // Undo.
        crate::database::UndoOutcome::decl(&config),
        crate::database::UndoStatus::decl(&config),
    ]
    .map(|declaration| {
        // ts_rs wraps long object types with a trailing space at the break;
        // the checked-in file carries no trailing whitespace.
        let declaration = declaration
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
        format!("export {declaration}")
    });
    format!(
        "// This file is generated from Rust wire types. Do not edit.\n\n{}\n",
        declarations.join("\n\n")
    )
}

#[cfg(test)]
#[test]
fn generated_presentation_types_are_current() {
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../src/generated/presentation-types.ts");
    let generated = generated_presentation_types();
    if std::env::var_os("CANCAN_WRITE_PRESENTATION_TYPES").is_some() {
        std::fs::write(&output, generated).expect("write presentation types");
        return;
    }
    let checked_in = std::fs::read_to_string(&output).expect("read presentation types");
    assert_eq!(checked_in, generated, "presentation types are stale");
}
