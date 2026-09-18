use super::*;
use crate::diagnostics::{
    APP_VERSION, JobFailureDetail, OPERATIONAL_LOG_RETENTION_DAYS, OperationalDiagnosticsCategory,
    OperationalDiagnosticsPreview, OperationalLogEntry, OperationalLogExport, OperationalLogRow,
    render_export, render_export_lines,
};

/// The most recent entries one diagnostic export can contain. The export file
/// states when the window was truncated so a partial report is never mistaken
/// for a complete one.
const OPERATIONAL_LOG_EXPORT_LIMIT: u32 = 5_000;
/// Distinct labels listed per dimension in a preview.
const OPERATIONAL_LOG_PREVIEW_LABELS: usize = 12;
/// Redacted lines shown in a preview before the user writes the file.
const OPERATIONAL_LOG_PREVIEW_LINES: usize = 20;
const JOB_FAILURE_COLUMNS: &str = "job_type, status, attempts, max_attempts, \
     CASE WHEN started_at IS NULL THEN NULL \
          ELSE CAST((julianday('now') - julianday(started_at)) * 86400000 AS INTEGER) END, \
     related_source_document_id, related_money_source_id, related_review_item_id";

impl ManualImportStore {
    /// Appends one redacted operational log entry.
    ///
    /// Callers treat logging as best-effort: a Vault that cannot record a
    /// diagnostic must still report the original failure to the user, and the
    /// durable job row stays the authoritative record.
    pub(crate) fn record_operational_log(&self, entry: &OperationalLogEntry) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO operational_logs( \
               level, component, error_code, error_kind, detail, cause, \
               job_type, job_status, attempt, duration_ms, app_version \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                entry.level.as_str(),
                entry.component,
                entry.error_code.as_deref(),
                entry.error_kind,
                entry.detail.as_str(),
                entry.cause.as_ref().map(|cause| cause.as_str()),
                entry.job_type.as_deref(),
                entry.job_status.as_deref(),
                entry.attempt,
                entry.duration_ms,
                APP_VERSION,
            ],
        )?;
        Ok(())
    }

    /// Deletes entries older than the retention window. Runs on every Vault
    /// open, so retention holds without a background timer.
    pub(crate) fn purge_expired_operational_logs(&mut self) -> StoreResult<usize> {
        let deleted = self.connection.execute(
            "DELETE FROM operational_logs WHERE created_at < datetime('now', ?1)",
            [format!("-{OPERATIONAL_LOG_RETENTION_DAYS} days")],
        )?;
        Ok(deleted)
    }

    /// What a diagnostic export would contain: categories and counts only, with
    /// the same redacted sample lines the export writes.
    pub(crate) fn operational_diagnostics_preview(
        &self,
    ) -> StoreResult<OperationalDiagnosticsPreview> {
        let (entry_count, oldest_entry_at, newest_entry_at) = self.connection.query_row(
            "SELECT COUNT(*), MIN(created_at), MAX(created_at) FROM operational_logs",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )?;
        Ok(OperationalDiagnosticsPreview {
            app_version: APP_VERSION.to_owned(),
            retention_days: OPERATIONAL_LOG_RETENTION_DAYS,
            entry_count: u32::try_from(entry_count).unwrap_or(u32::MAX),
            export_limit: OPERATIONAL_LOG_EXPORT_LIMIT,
            oldest_entry_at,
            newest_entry_at,
            components: self.operational_log_category_counts("component")?,
            error_codes: self.operational_log_category_counts("COALESCE(error_code, '(none)')")?,
            sample_lines: render_export_lines(
                &self.operational_log_rows(OPERATIONAL_LOG_PREVIEW_LINES as u32)?,
            ),
        })
    }

    /// The full export file: header, boundary statement, and one redacted line
    /// per retained entry, oldest first.
    pub(crate) fn operational_diagnostics_export(&self) -> StoreResult<String> {
        let rows = self.operational_log_rows(OPERATIONAL_LOG_EXPORT_LIMIT)?;
        let entry_count =
            self.connection
                .query_row("SELECT COUNT(*) FROM operational_logs", [], |row| {
                    row.get::<_, i64>(0)
                })?;
        let generated_at = self.connection.query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
            [],
            |row| row.get::<_, String>(0),
        )?;
        Ok(render_export(&OperationalLogExport {
            app_version: APP_VERSION,
            generated_at: &generated_at,
            retention_days: OPERATIONAL_LOG_RETENTION_DAYS,
            entry_count: u32::try_from(entry_count).unwrap_or(u32::MAX),
            rows: &rows,
        }))
    }

    /// The job context behind a failure, keyed by job id. Used by the review and
    /// parse job writers, which hold the claim.
    pub(super) fn job_failure_context(&self, job_id: &str) -> StoreResult<JobFailureContext> {
        Ok(self.connection.query_row(
            &format!("SELECT {JOB_FAILURE_COLUMNS} FROM jobs WHERE id = ?1"),
            [job_id],
            job_failure_context_from_row,
        )?)
    }

    /// The job context behind a failure, keyed by the document and job type the
    /// reconcile writer updates.
    ///
    /// The read carries the same claim filter as the update it feeds — the job
    /// is running under [`RECONCILE_DOCUMENT_LEASE_OWNER`] — so a superseded run
    /// of the same document cannot contribute its attempt count and related ids
    /// to a newer failure's `error_json`.
    pub(super) fn reconcile_job_failure_context(
        &self,
        document_id: &str,
    ) -> StoreResult<Option<JobFailureContext>> {
        Ok(self
            .connection
            .query_row(
                &format!(
                    "SELECT {JOB_FAILURE_COLUMNS} FROM jobs \
                     WHERE related_source_document_id = ?1 AND job_type = ?2 \
                       AND status = 'running' AND lease_owner = ?3 \
                     ORDER BY created_at DESC, id DESC LIMIT 1"
                ),
                params![
                    document_id,
                    RECONCILE_DOCUMENT_JOB_TYPE,
                    RECONCILE_DOCUMENT_LEASE_OWNER
                ],
                job_failure_context_from_row,
            )
            .optional()?)
    }

    fn operational_log_category_counts(
        &self,
        expression: &'static str,
    ) -> StoreResult<Vec<OperationalDiagnosticsCategory>> {
        let sql = format!(
            "SELECT {expression} AS label, COUNT(*) AS entries FROM operational_logs \
             GROUP BY label ORDER BY entries DESC, label ASC LIMIT {OPERATIONAL_LOG_PREVIEW_LABELS}"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let categories = statement
            .query_map([], |row| {
                Ok(OperationalDiagnosticsCategory {
                    label: row.get::<_, String>(0)?,
                    count: u32::try_from(row.get::<_, i64>(1)?).unwrap_or(u32::MAX),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(categories)
    }

    /// The most recent entries, oldest first.
    ///
    /// Read by the preview and the export, and by the tests that pin what a
    /// failure leaves behind.
    pub(crate) fn operational_log_rows(&self, limit: u32) -> StoreResult<Vec<OperationalLogRow>> {
        let mut statement = self.connection.prepare(
            "SELECT created_at, level, component, error_code, error_kind, detail, cause, \
                    job_type, job_status, attempt, duration_ms, app_version \
             FROM operational_logs ORDER BY id DESC LIMIT ?1",
        )?;
        let mut rows = statement
            .query_map([limit], |row| {
                Ok(OperationalLogRow {
                    created_at: row.get(0)?,
                    level: row.get(1)?,
                    component: row.get(2)?,
                    error_code: row.get(3)?,
                    error_kind: row.get(4)?,
                    detail: row.get(5)?,
                    cause: row.get(6)?,
                    job_type: row.get(7)?,
                    job_status: row.get(8)?,
                    attempt: row.get(9)?,
                    duration_ms: row.get(10)?,
                    app_version: row.get(11)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.reverse();
        Ok(rows)
    }
}

fn job_failure_context_from_row(row: &Row<'_>) -> rusqlite::Result<JobFailureContext> {
    Ok(JobFailureContext {
        job_type: row.get(0)?,
        status: row.get(1)?,
        attempt: row.get(2)?,
        max_attempts: row.get(3)?,
        duration_ms: row.get(4)?,
        related_source_document_id: row.get(5)?,
        related_money_source_id: row.get(6)?,
        related_review_item_id: row.get(7)?,
    })
}

/// The job row state a failure is recorded against.
#[derive(Debug)]
pub(super) struct JobFailureContext {
    pub(super) job_type: String,
    pub(super) status: String,
    pub(super) attempt: i64,
    pub(super) max_attempts: i64,
    pub(super) duration_ms: Option<i64>,
    pub(super) related_source_document_id: Option<String>,
    pub(super) related_money_source_id: Option<String>,
    pub(super) related_review_item_id: Option<String>,
}

impl JobFailureContext {
    /// Adds this job's type, state, retry count, and duration to a log entry.
    pub(super) fn log_entry(&self, entry: OperationalLogEntry) -> OperationalLogEntry {
        entry.with_job(
            self.job_type.clone(),
            self.status.clone(),
            self.attempt,
            self.duration_ms,
        )
    }

    /// The two-layer `error_json` value: the static code the renderer and retry
    /// policy already use, plus the spec 0015 technical layer built from this
    /// job's own row and the caller's redacted detail.
    pub(super) fn error_json(
        &self,
        error_code: &str,
        detail: &JobFailureDetail,
    ) -> serde_json::Value {
        serde_json::json!({
            "errorCode": error_code,
            "technical": {
                "errorCode": error_code,
                "errorKind": detail.error_kind,
                "message": detail.message.as_str(),
                "cause": detail.cause.as_ref().map(|cause| cause.as_str()),
                "jobType": self.job_type,
                "attempt": self.attempt,
                "maxAttempts": self.max_attempts,
                "retryable": detail.retryable,
                "provider": detail.provider,
                "reportedCode": detail.reported_code,
                "related": {
                    "sourceDocumentId": self.related_source_document_id,
                    "moneySourceId": self.related_money_source_id,
                    "reviewItemId": self.related_review_item_id,
                },
            },
        })
    }
}
