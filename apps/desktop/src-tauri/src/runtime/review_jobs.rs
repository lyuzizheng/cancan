//! The review-batch job flow: enqueue, claim, commit group by group, and
//! finish or fail the batch. Split out of `review.rs` under the source-file
//! size guardrail.

use super::*;
use crate::diagnostics::JobFailureDetail;

impl VaultRuntime {
    pub(super) fn enqueue_commit_review_batch(
        &self,
        review_item_ids: &[String],
    ) -> Result<ReviewJobSummary, RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .enqueue_commit_review_batch(review_item_ids)
            .map_store_error(
                store,
                "enqueue_commit_review_batch",
                "invalid_review_request",
            )
    }

    pub(super) fn review_job(
        &self,
        job_id: &str,
    ) -> Result<Option<ReviewJobSummary>, RuntimeError> {
        if job_id.is_empty() {
            return Err(RuntimeError::new("invalid_review_request"));
        }
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .review_job(job_id)
            .map_store_error(store, "review_job", "review_unavailable")
    }

    pub(super) fn queued_review_job_ids(&self) -> Result<Vec<String>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store.queued_review_job_ids().map_store_error(
            store,
            "queued_review_job_ids",
            "review_unavailable",
        )
    }

    pub(super) fn claim_review_batch(
        &self,
        job_id: &str,
        lease_owner: &str,
    ) -> Result<Option<ClaimedReviewBatch>, RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .claim_review_batch(job_id, lease_owner)
            .map_store_error(store, "claim_review_batch", "review_unavailable")
    }

    pub(super) fn prepare_commit_review_groups(
        &self,
        claimed: &ClaimedReviewBatch,
    ) -> Result<(Vec<CommitReviewGroup>, Vec<ReviewBatchGroupOutcome>), RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store.prepare_commit_review_groups(claimed).map_store_error(
            store,
            "prepare_commit_review_groups",
            "review_unavailable",
        )
    }

    pub(super) fn commit_prepared_review_group(
        &self,
        claimed: &ClaimedReviewBatch,
        group: &CommitReviewGroup,
        event: &CorePreparedReviewEvent,
    ) -> Result<ReviewBatchGroupOutcome, RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .commit_prepared_review_group(claimed, group, event)
            .map_store_error(store, "commit_prepared_review_group", "review_unavailable")
    }

    pub(super) fn finish_review_batch(
        &self,
        claimed: &ClaimedReviewBatch,
        outcomes: &[ReviewBatchGroupOutcome],
    ) -> Result<ReviewJobSummary, RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .finish_review_batch(claimed, outcomes)
            .map_store_error(store, "finish_review_batch", "review_unavailable")
    }

    pub(super) fn fail_review_batch(
        &self,
        claimed: &ClaimedReviewBatch,
        detail: &JobFailureDetail,
    ) -> Result<ReviewJobSummary, RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .fail_review_batch(claimed, "review_core_failed", Some(detail))
            .map_store_error(store, "fail_review_batch", "review_unavailable")
    }
}
