use super::{ManualImportStore, PARSE_DOCUMENT_JOB_TYPE, StoreResult};
use rusqlite::params;

impl ManualImportStore {
    pub(crate) fn requeue_password_blocked_parse_document_job(
        &mut self,
        document_id: &str,
    ) -> StoreResult<bool> {
        let transaction = self.connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE jobs SET status = 'queued', blocked_reason = NULL, lease_owner = NULL, \
                     lease_until = NULL, finished_at = NULL, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ( \
               SELECT id FROM jobs \
               WHERE related_source_document_id = ?1 AND job_type = ?2 \
                 AND status = 'blocked' AND blocked_reason = 'password_required' \
               ORDER BY updated_at DESC, id DESC LIMIT 1 \
             )",
            params![document_id, PARSE_DOCUMENT_JOB_TYPE],
        )?;
        transaction.commit()?;
        Ok(changed == 1)
    }

    pub(super) fn recover_interrupted_parse_document_jobs(&mut self) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs \
             SET status = 'queued', lease_owner = NULL, lease_until = NULL, updated_at = CURRENT_TIMESTAMP \
             WHERE job_type = ?1 AND status = 'running'",
            [PARSE_DOCUMENT_JOB_TYPE],
        )?;
        Ok(())
    }
}
