use super::{
    ManualImportStore, PARSE_DOCUMENT_JOB_TYPE, ParseDocumentClaim, SourceDocumentRoutingOutcome,
    SourceDocumentRoutingStatus, StoreResult, finish_parse_document_job_in_transaction,
    new_audit_id,
};
use crate::content_fingerprint::ContentFingerprint;
use rusqlite::{OptionalExtension, Transaction, params};
use std::io;

/// Terminal decision for the canonical content of one claimed parse job.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum CanonicalContentDecision {
    /// No prior success holds this content, so the claimed artifact is the
    /// statement's first evidence and normal parsing continues.
    Parsed,
    /// A trusted successful parse already holds this content; the claimed
    /// artifact is additional evidence under that statement.
    Matched { document_id: String },
}

/// Statement facts a matched document can lend to the artifact that reuses it.
struct ReusableStatement {
    document_id: String,
    document_type: Option<String>,
    money_source_id: String,
    semantic_document_key: String,
    statement_period_from: Option<String>,
    statement_period_to: Option<String>,
}

impl ManualImportStore {
    /// Records the canonical content fingerprint of one claimed parse job and,
    /// when a prior successful parse already holds that content, links this
    /// artifact to the matched statement and finishes the job as a reuse.
    ///
    /// The fingerprint is written on every parse attempt, whether or not a
    /// match exists: it is the durable record that this artifact's complete
    /// local extraction succeeded, and the artifact that follows it must be
    /// able to find it. A fingerprint that cannot be written therefore fails
    /// the caller instead of leaving the artifact permanently unmatched.
    ///
    /// A match requires a prior *successful* trusted parse of the same content
    /// that produced reusable records, so a blocked, failed, or
    /// unclassifiable attempt never becomes a match target and the artifact
    /// falls back to normal parsing. The matched document's statement identity
    /// is the one durable fact the link copies; nothing is regenerated.
    pub(crate) fn record_canonical_content_for_claimed_job(
        &mut self,
        claim: &ParseDocumentClaim,
        fingerprint: &ContentFingerprint,
    ) -> StoreResult<CanonicalContentDecision> {
        let transaction = self.connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE source_documents \
                SET canonical_content_fingerprint = ?1, \
                    canonical_content_fingerprint_version = ?2 \
              WHERE id = ?3",
            params![
                fingerprint.hex.as_str(),
                fingerprint.version,
                claim.document_id.as_str(),
            ],
        )?;
        if changed != 1 {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        }
        let Some(matched) = find_reusable_statement(&transaction, claim, fingerprint)? else {
            transaction.commit()?;
            return Ok(CanonicalContentDecision::Parsed);
        };
        link_artifact_to_matched_statement(&transaction, claim, &matched)?;
        finish_parse_document_job_in_transaction(
            &transaction,
            claim,
            &SourceDocumentRoutingOutcome {
                account_ids: Vec::new(),
                document_id: claim.document_id.clone(),
                money_source_id: Some(matched.money_source_id.clone()),
                reason: None,
                status: SourceDocumentRoutingStatus::SameContent,
            },
        )?;
        transaction.commit()?;
        Ok(CanonicalContentDecision::Matched {
            document_id: matched.document_id,
        })
    }
}

/// Finds the one document this artifact reuses: the most recent artifact of the
/// same canonical content whose trusted parse succeeded and whose records are
/// still stored.
///
/// Documents that are themselves reuses are excluded by construction: they
/// never ran a parse to success and never own records, so the two `EXISTS`
/// clauses already reject them. Ordering prefers an artifact whose file is
/// still stored, then the most recent receipt.
fn find_reusable_statement(
    transaction: &Transaction<'_>,
    claim: &ParseDocumentClaim,
    fingerprint: &ContentFingerprint,
) -> rusqlite::Result<Option<ReusableStatement>> {
    transaction
        .query_row(
            "SELECT source_documents.id, source_documents.document_type, \
                    source_documents.money_source_id, source_documents.semantic_document_key, \
                    source_documents.statement_period_from, source_documents.statement_period_to \
               FROM source_documents \
              WHERE source_documents.canonical_content_fingerprint = ?1 \
                AND source_documents.canonical_content_fingerprint_version = ?2 \
                AND source_documents.id <> ?3 \
                AND source_documents.money_source_id IS NOT NULL \
                AND source_documents.semantic_document_key IS NOT NULL \
                AND EXISTS( \
                  SELECT 1 FROM jobs \
                   WHERE jobs.related_source_document_id = source_documents.id \
                     AND jobs.job_type = ?4 AND jobs.status = 'succeeded' \
                ) \
                AND EXISTS( \
                  SELECT 1 FROM external_records \
                   WHERE external_records.source_document_id = source_documents.id \
                ) \
              ORDER BY (source_documents.file_state = 'available') DESC, \
                       source_documents.received_at DESC, source_documents.id DESC \
              LIMIT 1",
            params![
                fingerprint.hex.as_str(),
                fingerprint.version,
                claim.document_id.as_str(),
                PARSE_DOCUMENT_JOB_TYPE,
            ],
            |row| {
                Ok(ReusableStatement {
                    document_id: row.get(0)?,
                    document_type: row.get(1)?,
                    money_source_id: row.get(2)?,
                    semantic_document_key: row.get(3)?,
                    statement_period_from: row.get(4)?,
                    statement_period_to: row.get(5)?,
                })
            },
        )
        .optional()
}

/// Links the claimed artifact to the statement it duplicates: the artifact
/// keeps its own bytes, Vault location, and provenance while it adopts the
/// matched statement's identity, and it never absorbs the matched document's
/// records.
///
/// The link is the audit event, so it writes one row when it points the
/// artifact at a new document and none when the artifact already points there,
/// which keeps a repeated attempt idempotent.
fn link_artifact_to_matched_statement(
    transaction: &Transaction<'_>,
    claim: &ParseDocumentClaim,
    matched: &ReusableStatement,
) -> StoreResult<()> {
    let audit_id = new_audit_id();
    let changed = transaction.execute(
        "UPDATE source_documents \
            SET canonical_content_match_document_id = ?1, \
                money_source_id = COALESCE(money_source_id, ?2), \
                semantic_document_key = COALESCE(semantic_document_key, ?3), \
                document_type = COALESCE(document_type, ?4), \
                statement_period_from = COALESCE(statement_period_from, ?5), \
                statement_period_to = COALESCE(statement_period_to, ?6) \
          WHERE id = ?7 \
            AND (canonical_content_match_document_id IS NULL \
                 OR canonical_content_match_document_id <> ?1)",
        params![
            matched.document_id.as_str(),
            matched.money_source_id.as_str(),
            matched.semantic_document_key.as_str(),
            matched.document_type.as_deref(),
            matched.statement_period_from.as_deref(),
            matched.statement_period_to.as_deref(),
            claim.document_id.as_str(),
        ],
    )?;
    if changed == 0 {
        return Ok(());
    }
    if changed != 1 {
        return Err(io::Error::other("content match links exactly one artifact").into());
    }
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, 'source_document_content_matched', \
                   'system', 'canonical_content_matched', ?3, 'content-fingerprint-v1')",
        params![
            audit_id,
            claim.document_id.as_str(),
            matched.document_id.as_str(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
