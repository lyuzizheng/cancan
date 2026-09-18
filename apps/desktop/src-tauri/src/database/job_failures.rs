//! Failure and blocked-state transitions for claimed jobs: the two-layer
//! `error_json` payload and the operational log entry spec 0015 requires
//! for every failure. Split out of `mod.rs` under the source-file size
//! guardrail.

use super::*;

/// The operational-log entry for a failed job: the technical layer the runtime
/// carried with the failure when there is one, otherwise the static code alone.
fn job_failure_log_entry(
    component: &'static str,
    reason: &str,
    detail: Option<&JobFailureDetail>,
) -> OperationalLogEntry {
    match detail {
        Some(detail) => OperationalLogEntry::from_detail(component, reason, detail),
        None => OperationalLogEntry::failure_code(component, reason),
    }
}

impl ManualImportStore {
    pub(crate) fn fail_reconcile_document(
        &mut self,
        document_id: &str,
        reason: &'static str,
        detail: Option<&JobFailureDetail>,
    ) -> StoreResult<()> {
        // The context read is best-effort: a job that cannot be read back must
        // still be marked failed.
        let context = self
            .reconcile_job_failure_context(document_id)
            .ok()
            .flatten();
        let error_json = match (context.as_ref(), detail) {
            (Some(context), Some(detail)) => {
                serde_json::to_string(&context.error_json(reason, detail))?
            }
            _ => serde_json::json!({ "errorCode": reason }).to_string(),
        };
        let changed = self.connection.execute(
            "UPDATE jobs SET status = 'failed', error_json = ?1, blocked_reason = ?2, \
                     lease_owner = NULL, lease_until = NULL, finished_at = CURRENT_TIMESTAMP, \
                     updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?3 AND job_type = ?4 \
               AND status = 'running' AND lease_owner = ?5",
            params![
                error_json,
                reason,
                document_id,
                RECONCILE_DOCUMENT_JOB_TYPE,
                RECONCILE_DOCUMENT_LEASE_OWNER
            ],
        )?;
        if changed == 1
            && let Some(context) = context
        {
            let _ = self.record_operational_log(&context.log_entry(job_failure_log_entry(
                "job.reconcile_document",
                reason,
                detail,
            )));
        }
        Ok(())
    }

    pub(crate) fn block_parse_document_job(
        &mut self,
        claim: &ParseDocumentClaim,
        reason: &'static str,
    ) -> StoreResult<()> {
        let changed = self.connection.execute(
            "UPDATE jobs SET status = 'blocked', blocked_reason = ?1, lease_owner = NULL, \
                     lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?2 AND related_source_document_id = ?3 AND job_type = ?4 \
               AND status = 'running' AND lease_owner = ?5",
            params![
                reason,
                claim.job_id,
                claim.document_id,
                PARSE_DOCUMENT_JOB_TYPE,
                claim.claim_token,
            ],
        )?;
        parse_job_update(changed)
    }

    pub(crate) fn fail_parse_document_job(
        &mut self,
        claim: &ParseDocumentClaim,
        reason: &'static str,
        detail: Option<&JobFailureDetail>,
    ) -> StoreResult<()> {
        // The context read is best-effort: the job state write must not depend
        // on it.
        let context = self.job_failure_context(&claim.job_id).ok();
        let error_json = match (context.as_ref(), detail) {
            (Some(context), Some(detail)) => {
                serde_json::to_string(&context.error_json(reason, detail))?
            }
            _ => serde_json::json!({ "errorCode": reason }).to_string(),
        };
        let changed = self.connection.execute(
            "UPDATE jobs SET \
                 status = CASE WHEN attempts < max_attempts THEN 'queued' ELSE 'failed' END, \
                 result_json = NULL, \
                 error_json = CASE WHEN attempts < max_attempts THEN NULL ELSE ?1 END, \
                 blocked_reason = CASE WHEN attempts < max_attempts THEN NULL ELSE ?2 END, \
                 lease_owner = NULL, lease_until = NULL, \
                 finished_at = CASE WHEN attempts < max_attempts THEN NULL ELSE CURRENT_TIMESTAMP END, \
                 updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?3 AND related_source_document_id = ?4 AND job_type = ?5 \
               AND status = 'running' AND lease_owner = ?6",
            params![
                error_json,
                reason,
                claim.job_id,
                claim.document_id,
                PARSE_DOCUMENT_JOB_TYPE,
                claim.claim_token,
            ],
        )?;
        parse_job_update(changed)?;
        if let Some(context) = context {
            // An attempt that still has retries left only requeues the job; the
            // same SQL decides that, so the level mirrors it.
            let level = if context.attempt < context.max_attempts {
                OperationalLogLevel::Warning
            } else {
                OperationalLogLevel::Error
            };
            let _ = self.record_operational_log(
                &context
                    .log_entry(job_failure_log_entry("job.parse_document", reason, detail))
                    .with_level(level),
            );
        }
        Ok(())
    }

    pub(crate) fn fail_review_batch(
        &mut self,
        claimed: &ClaimedReviewBatch,
        error_code: &str,
        detail: Option<&JobFailureDetail>,
    ) -> StoreResult<ReviewJobSummary> {
        if error_code.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "review error code is required",
            )
            .into());
        }
        let context = self.job_failure_context(&claimed.job_id).ok();
        let error_json = match (context.as_ref(), detail) {
            (Some(context), Some(detail)) => {
                serde_json::to_string(&context.error_json(error_code, detail))?
            }
            _ => serde_json::to_string(&serde_json::json!({ "errorCode": error_code }))?,
        };
        let changed = self.connection.execute(
            "UPDATE jobs SET status = 'failed', error_json = ?1, lease_owner = NULL, \
                     lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?2 AND job_type = ?3 AND status = 'running' AND lease_owner = ?4",
            params![
                error_json,
                claimed.job_id,
                COMMIT_REVIEW_BATCH_JOB_TYPE,
                claimed.lease_owner
            ],
        )?;
        if changed != 1 {
            return Err(
                io::Error::new(io::ErrorKind::InvalidData, "review job lease changed").into(),
            );
        }
        let Some(summary) = self.review_job(&claimed.job_id)? else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "failed review job was not persisted",
            )
            .into());
        };
        if let Some(context) = context {
            let _ = self.record_operational_log(&context.log_entry(job_failure_log_entry(
                "job.commit_review_batch",
                error_code,
                detail,
            )));
        }
        Ok(summary)
    }
}
