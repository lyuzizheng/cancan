use super::{
    ManualImportStore, SourceDocumentImport, SourceDocumentImportOutcome,
    SourceDocumentRoutingOutcome, StoreResult, TrustedDocumentClassification,
};
use rusqlite::{OptionalExtension, params};
use zeroize::Zeroizing;

#[cfg(test)]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StructuredParseTestState {
    pub(crate) balance_snapshots_without_amount: i64,
    pub(crate) ledger_events: i64,
    pub(crate) open_review_items: i64,
    pub(crate) parse_runs: i64,
    pub(crate) reconcile_status: Option<String>,
    pub(crate) records: i64,
    pub(crate) staged_records: i64,
}

impl ManualImportStore {
    #[cfg(test)]
    pub fn register_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        super::imports::test_support_register_import(self, input, restore_deleted_document_id)
    }

    #[cfg(test)]
    pub(crate) fn register_captured_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        captured_bytes: Zeroizing<Vec<u8>>,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        super::imports::test_support_register_captured_import(
            self,
            input,
            captured_bytes,
            restore_deleted_document_id,
        )
    }

    #[cfg(test)]
    pub(crate) fn persist_captured_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        stored: &crate::vault::StoredFile,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        super::imports::test_support_persist_captured_import(
            &mut self.connection,
            input,
            stored,
            restore_deleted_document_id,
        )
    }

    #[cfg(test)]
    pub fn apply_trusted_classification(
        &mut self,
        input: &TrustedDocumentClassification<'_>,
    ) -> StoreResult<SourceDocumentRoutingOutcome> {
        self.apply_trusted_classification_with_parse_job(input, None)
    }

    #[cfg(test)]
    pub(crate) fn structured_parse_test_state(
        &self,
        document_id: &str,
    ) -> StoreResult<StructuredParseTestState> {
        Ok(StructuredParseTestState {
            parse_runs: self.connection.query_row(
                "SELECT count(*) FROM parse_runs WHERE source_document_id = ?1",
                [document_id],
                |row| row.get(0),
            )?,
            records: self.connection.query_row(
                "SELECT count(*) FROM external_records WHERE source_document_id = ?1",
                [document_id],
                |row| row.get(0),
            )?,
            staged_records: self.connection.query_row(
                "SELECT count(*) FROM external_records \
                 WHERE source_document_id = ?1 AND status = 'staged'",
                [document_id],
                |row| row.get(0),
            )?,
            balance_snapshots_without_amount: self.connection.query_row(
                "SELECT count(*) FROM external_records \
                 WHERE source_document_id = ?1 AND record_type = 'balance' \
                   AND amount_value IS NULL",
                [document_id],
                |row| row.get(0),
            )?,
            open_review_items: self.connection.query_row(
                "SELECT count(*) FROM review_items \
                 JOIN external_records ON external_records.id = review_items.external_record_id \
                 WHERE external_records.source_document_id = ?1 \
                   AND review_items.reason_code = 'normalization_profile_unqualified' \
                   AND review_items.status = 'open'",
                [document_id],
                |row| row.get(0),
            )?,
            reconcile_status: self
                .connection
                .query_row(
                    "SELECT status FROM jobs WHERE related_source_document_id = ?1 \
                     AND job_type = 'reconcile_document'",
                    [document_id],
                    |row| row.get(0),
                )
                .optional()?,
            ledger_events: self.connection.query_row(
                "SELECT count(*) FROM ledger_events",
                [],
                |row| row.get(0),
            )?,
        })
    }

    #[cfg(test)]
    pub(crate) fn structured_parse_posting_status(
        &self,
        document_id: &str,
        stable_record_key: &str,
    ) -> StoreResult<Option<String>> {
        self.connection
            .query_row(
                "SELECT posting_status FROM external_records \
                 WHERE source_document_id = ?1 AND stable_record_key = ?2",
                params![document_id, stable_record_key],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    #[cfg(test)]
    pub(crate) fn expire_reconcile_lease_for_test(&self, document_id: &str) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs SET lease_until = datetime('now', '-1 second') \
             WHERE related_source_document_id = ?1 AND job_type = 'reconcile_document'",
            [document_id],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn expire_parse_document_lease_for_test(&self, job_id: &str) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs SET lease_until = datetime('now', '-1 second') \
             WHERE id = ?1 AND job_type = 'parse_document'",
            [job_id],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn seed_money_source(
        &self,
        id: &str,
        provider_key: &str,
        display_name: &str,
        source_type: &str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES (?1, ?2, ?3, ?4)",
            params![id, provider_key, display_name, source_type],
        )?;
        Ok(())
    }
}

impl ManualImportStore {
    #[cfg(test)]
    pub(crate) fn confirm_candidate_accounts(
        &mut self,
        money_source_id: &str,
        expected_candidate_account_ids: &[String],
        audit_id: &str,
    ) -> StoreResult<super::accounts::AccountConfirmationOutcome> {
        let proposal_version = super::accounts::candidate_proposal_version_for_test(
            &self.connection,
            money_source_id,
        )?;
        self.decide_candidate_accounts(
            money_source_id,
            &proposal_version,
            &expected_candidate_account_ids
                .iter()
                .cloned()
                .map(
                    |account_id| super::accounts::CandidateAccountDecisionInput {
                        account_id,
                        action: super::accounts::CandidateAccountDecision::Accept,
                    },
                )
                .collect::<Vec<_>>(),
            audit_id,
        )
    }
}
