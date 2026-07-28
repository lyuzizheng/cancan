use super::{
    ManualImportStore, PARSE_DOCUMENT_JOB_TYPE, StoreResult, enqueue_parse_document,
    new_database_id,
};
use rusqlite::params;

impl ManualImportStore {
    pub(crate) fn enqueue_source_document_pipeline(
        &mut self,
        document_id: &str,
    ) -> StoreResult<bool> {
        if document_id.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "source document id must not be empty",
            )
            .into());
        }
        let transaction = self.connection.transaction()?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM source_documents WHERE id = ?1)",
            [document_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "source document not found",
            )
            .into());
        }
        let active: bool = transaction.query_row(
            "SELECT EXISTS( \
               SELECT 1 FROM jobs \
               WHERE related_source_document_id = ?1 AND job_type = ?2 \
                 AND status IN ('queued', 'running') \
             )",
            params![document_id, PARSE_DOCUMENT_JOB_TYPE],
            |row| row.get(0),
        )?;
        if active {
            return Ok(false);
        }
        enqueue_parse_document(&transaction, document_id, &new_database_id("parse-run"))?;
        transaction.commit()?;
        Ok(true)
    }

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
        // The app holds exclusive process ownership of the Vault, so a running
        // parse claim at open cannot still belong to a live peer.
        self.connection.execute(
            "UPDATE jobs \
             SET status = 'queued', lease_owner = NULL, lease_until = NULL, updated_at = CURRENT_TIMESTAMP \
             WHERE job_type = ?1 AND status = 'running'",
            [PARSE_DOCUMENT_JOB_TYPE],
        )?;
        Ok(())
    }
}
