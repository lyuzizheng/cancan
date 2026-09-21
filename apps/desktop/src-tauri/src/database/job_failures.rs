//! Failure and blocked-state transitions for claimed jobs: the two-layer
//! `error_json` payload and the operational log entry spec 0015 requires
//! for every failure. Split out of `mod.rs` under the source-file size
//! guardrail.

use super::*;
use crate::database::operational_logs::{JOB_FAILURE_COLUMNS, job_failure_context_from_row};

/// The static code a job carries when a claim refused it because its attempts
/// were already spent.
///
/// Spec 0015 puts `error_json` and the operational log entry inside the
/// failure, not beside it, so this transition writes both: a dead letter
/// nobody can diagnose is the defect the code exists to prevent.
pub(super) const RETRY_LIMIT_REACHED: &str = "retry_limit_reached";

/// The claim predicate that identifies one exhausted queued job. It is the
/// same predicate the refused claim ran on, so the rows read here are exactly
/// the rows that claim declined.
const EXHAUSTED_JOB_BY_ID: &str =
    "id = ?1 AND job_type = ?2 AND status = 'queued' AND attempts >= max_attempts";

/// [`EXHAUSTED_JOB_BY_ID`] keyed by the document claim instead of one row.
const EXHAUSTED_JOBS_BY_DOCUMENT: &str = "related_source_document_id = ?1 AND job_type = ?2 \
     AND status = 'queued' AND attempts >= max_attempts";

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

/// The technical layer of a refused claim: the store's own decision, with no
/// error object behind it, so the static code is the whole message.
fn retry_limit_detail() -> JobFailureDetail {
    JobFailureDetail::from_code(None, RETRY_LIMIT_REACHED)
}

impl ManualImportStore {
    /// Fails the queued jobs a refused claim covers, with the two-layer
    /// `error_json` and the operational log entry spec 0015 requires.
    ///
    /// `filter` is the predicate the claim used, so the rows that transition
    /// are the rows whose attempts are spent. Returns how many rows failed.
    fn fail_exhausted_jobs(
        &mut self,
        filter: &'static str,
        key: &str,
        job_type: &'static str,
        component: &'static str,
    ) -> StoreResult<usize> {
        let detail = retry_limit_detail();
        let transaction = self.connection.transaction()?;
        let exhausted = {
            let mut statement = transaction.prepare(&format!(
                "SELECT {JOB_FAILURE_COLUMNS}, id FROM jobs WHERE {filter} ORDER BY id"
            ))?;
            statement
                .query_map(params![key, job_type], |row| {
                    Ok((job_failure_context_from_row(row)?, row.get::<_, String>(8)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut failed = Vec::with_capacity(exhausted.len());
        for (context, job_id) in exhausted {
            let changed = transaction.execute(
                "UPDATE jobs SET status = 'failed', error_json = ?1, blocked_reason = ?2, \
                         finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
                 WHERE id = ?3 AND status = 'queued' AND attempts >= max_attempts",
                params![
                    context.error_json(RETRY_LIMIT_REACHED, &detail).to_string(),
                    RETRY_LIMIT_REACHED,
                    job_id
                ],
            )?;
            if changed == 1 {
                failed.push(context);
            }
        }
        transaction.commit()?;
        let failed_count = failed.len();
        for context in failed {
            let _ = self.record_operational_log(&context.log_entry(job_failure_log_entry(
                component,
                RETRY_LIMIT_REACHED,
                Some(&detail),
            )));
        }
        Ok(failed_count)
    }

    pub(crate) fn fail_exhausted_parse_document_job(&mut self, job_id: &str) -> StoreResult<usize> {
        self.fail_exhausted_jobs(
            EXHAUSTED_JOB_BY_ID,
            job_id,
            PARSE_DOCUMENT_JOB_TYPE,
            "job.parse_document",
        )
    }

    pub(crate) fn fail_exhausted_reconcile_document_jobs(
        &mut self,
        document_id: &str,
    ) -> StoreResult<usize> {
        self.fail_exhausted_jobs(
            EXHAUSTED_JOBS_BY_DOCUMENT,
            document_id,
            RECONCILE_DOCUMENT_JOB_TYPE,
            "job.reconcile_document",
        )
    }

    pub(crate) fn fail_exhausted_review_batch_job(&mut self, job_id: &str) -> StoreResult<usize> {
        self.fail_exhausted_jobs(
            EXHAUSTED_JOB_BY_ID,
            job_id,
            COMMIT_REVIEW_BATCH_JOB_TYPE,
            "job.commit_review_batch",
        )
    }

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
        // Spec 0015: a transient failure — a busy or locked database, an
        // interrupted read — is worth another attempt, and the attempt that
        // spends the last retry fails the job. Without this branch a transient
        // error stranded the document's staged records behind a terminal job
        // with no retry path at all.
        //
        // Unlike the parse path, the terminal decision comes from the
        // retryability the runtime classified alone: `reported_code` marks a
        // deterministic rejection by the parser sidecar, and no sidecar reports
        // a reconcile failure.
        let terminal = detail.is_some_and(|detail| !detail.retryable);
        let changed = self.connection.execute(
            "UPDATE jobs SET \
                 status = CASE WHEN ?1 OR attempts >= max_attempts THEN 'failed' ELSE 'queued' END, \
                 error_json = CASE WHEN ?1 OR attempts >= max_attempts THEN ?2 ELSE NULL END, \
                 blocked_reason = CASE WHEN ?1 OR attempts >= max_attempts THEN ?3 ELSE NULL END, \
                 lease_owner = NULL, lease_until = NULL, \
                 finished_at = CASE WHEN ?1 OR attempts >= max_attempts THEN CURRENT_TIMESTAMP ELSE NULL END, \
                 updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?4 AND job_type = ?5 \
               AND status = 'running' AND lease_owner = ?6",
            params![
                terminal,
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
            // An attempt that still has retries left only requeues the job; the
            // same SQL decides that, so the level mirrors it.
            let level = if terminal || context.attempt >= context.max_attempts {
                OperationalLogLevel::Error
            } else {
                OperationalLogLevel::Warning
            };
            let _ = self.record_operational_log(
                &context
                    .log_entry(job_failure_log_entry(
                        "job.reconcile_document",
                        reason,
                        detail,
                    ))
                    .with_level(level),
            );
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
        // Spec 0015: a deterministic sidecar rejection (budget exhausted, an
        // invalid structured proposal, ungrounded evidence) repeats on every
        // attempt, so it fails terminally instead of consuming retries and AI
        // budget. Transient failures keep the existing attempts accounting.
        let terminal =
            detail.is_some_and(|detail| !detail.retryable && detail.reported_code.is_some());
        let changed = self.connection.execute(
            "UPDATE jobs SET \
                 status = CASE WHEN ?1 OR attempts >= max_attempts THEN 'failed' ELSE 'queued' END, \
                 result_json = NULL, \
                 error_json = CASE WHEN ?1 OR attempts >= max_attempts THEN ?2 ELSE NULL END, \
                 blocked_reason = CASE WHEN ?1 OR attempts >= max_attempts THEN ?3 ELSE NULL END, \
                 lease_owner = NULL, lease_until = NULL, \
                 finished_at = CASE WHEN ?1 OR attempts >= max_attempts THEN CURRENT_TIMESTAMP ELSE NULL END, \
                 updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?4 AND related_source_document_id = ?5 AND job_type = ?6 \
               AND status = 'running' AND lease_owner = ?7",
            params![
                terminal,
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
            let level = if terminal || context.attempt >= context.max_attempts {
                OperationalLogLevel::Error
            } else {
                OperationalLogLevel::Warning
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
