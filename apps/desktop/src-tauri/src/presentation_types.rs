#[cfg(test)]
fn generated_presentation_types() -> String {
    use ts_rs::{Config, TS};

    let config = Config::default().with_large_int("number");
    let declarations = [
        crate::runtime::SourceDocumentStatus::decl(&config),
        crate::runtime::SourceDocumentSummary::decl(&config),
        crate::runtime::TaskFilter::decl(&config),
        crate::runtime::TaskGroup::decl(&config),
        crate::runtime::TaskConsequence::decl(&config),
        crate::runtime::TaskDestination::decl(&config),
        crate::runtime::TaskRow::decl(&config),
        crate::runtime::Tasks::decl(&config),
        crate::database::AccountConfirmationCandidate::decl(&config),
        crate::database::AccountConfirmationPrompt::decl(&config),
        crate::database::CandidateAccountDecision::decl(&config),
        crate::database::CandidateAccountDecisionInput::decl(&config),
        crate::database::AccountConfirmationStatus::decl(&config),
        crate::database::AccountConfirmationOutcome::decl(&config),
    ]
    .map(|declaration| format!("export {declaration}"));
    format!(
        "// This file is generated from Rust presentation types. Do not edit.\n\n{}\n",
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
