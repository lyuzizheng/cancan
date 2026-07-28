use super::*;
use sha2::{Digest, Sha256};

impl VaultRuntime {
    pub(crate) fn list_review_items(&self) -> Result<Vec<ReviewItemSummary>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .list_review_items()
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(crate) fn review_item_detail(
        &self,
        review_item_id: &str,
    ) -> Result<Option<ReviewItemDetail>, RuntimeError> {
        if review_item_id.is_empty() {
            return Err(RuntimeError::new("invalid_review_request"));
        }
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .review_item_detail(review_item_id)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(crate) fn list_recent_activity(&self) -> Result<Vec<RecentActivitySummary>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .list_recent_activity()
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(crate) fn money_overview(&self) -> Result<MoneyOverview, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .money_overview()
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn relationship_candidate_input(
        &self,
        review_item_id: &str,
        expected_record_version: i64,
    ) -> Result<Option<ReviewRelationshipCandidateInput>, RuntimeError> {
        if review_item_id.is_empty() || expected_record_version <= 0 {
            return Err(RuntimeError::new("invalid_review_request"));
        }
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .relationship_candidate_input(review_item_id, expected_record_version)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn relationship_candidate_summaries(
        &self,
        candidate_ids: &[String],
    ) -> Result<Vec<RelationshipCandidateSummary>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .relationship_candidate_summaries(candidate_ids)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn edit_review_record(
        &self,
        review_item_id: &str,
        expected_record_version: i64,
        posted_on: Option<&str>,
        amount_value: Option<&str>,
        account_balance_delta: Option<&str>,
    ) -> Result<ReviewMutationOutcome, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .edit_review_record(
                review_item_id,
                expected_record_version,
                posted_on,
                amount_value,
                account_balance_delta,
            )
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn remove_review_record(
        &self,
        review_item_id: &str,
        expected_record_version: i64,
    ) -> Result<ReviewMutationOutcome, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .remove_review_record(review_item_id, expected_record_version)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn accept_review_relationship(
        &self,
        review_item_id: &str,
        expected_record_version: i64,
        candidate_record_id: &str,
        expected_candidate_version: i64,
        event: &CorePreparedReviewEvent,
    ) -> Result<ReviewMutationOutcome, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .accept_review_relationship(
                review_item_id,
                expected_record_version,
                candidate_record_id,
                expected_candidate_version,
                event,
            )
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn enqueue_commit_review_batch(
        &self,
        review_item_ids: &[String],
    ) -> Result<ReviewJobSummary, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .enqueue_commit_review_batch(review_item_ids)
            .map_err(|_| RuntimeError::new("invalid_review_request"))
    }

    pub(super) fn review_job(
        &self,
        job_id: &str,
    ) -> Result<Option<ReviewJobSummary>, RuntimeError> {
        if job_id.is_empty() {
            return Err(RuntimeError::new("invalid_review_request"));
        }
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .review_job(job_id)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn queued_review_job_ids(&self) -> Result<Vec<String>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .queued_review_job_ids()
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn claim_review_batch(
        &self,
        job_id: &str,
        lease_owner: &str,
    ) -> Result<Option<ClaimedReviewBatch>, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .claim_review_batch(job_id, lease_owner)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn prepare_commit_review_groups(
        &self,
        claimed: &ClaimedReviewBatch,
    ) -> Result<(Vec<CommitReviewGroup>, Vec<ReviewBatchGroupOutcome>), RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .prepare_commit_review_groups(claimed)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn commit_prepared_review_group(
        &self,
        claimed: &ClaimedReviewBatch,
        group: &CommitReviewGroup,
        event: &CorePreparedReviewEvent,
    ) -> Result<ReviewBatchGroupOutcome, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .commit_prepared_review_group(claimed, group, event)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn finish_review_batch(
        &self,
        claimed: &ClaimedReviewBatch,
        outcomes: &[ReviewBatchGroupOutcome],
    ) -> Result<ReviewJobSummary, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .finish_review_batch(claimed, outcomes)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn fail_review_batch(
        &self,
        claimed: &ClaimedReviewBatch,
    ) -> Result<ReviewJobSummary, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .fail_review_batch(claimed, "review_core_failed")
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn committed_review_event_for_reversal(
        &self,
        event_id: &str,
    ) -> Result<Option<CorePreparedReviewEvent>, RuntimeError> {
        if event_id.is_empty() {
            return Err(RuntimeError::new("invalid_review_request"));
        }
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .committed_review_event_for_reversal(event_id)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    pub(super) fn persist_review_reversal(
        &self,
        original_event_id: &str,
        reversal: &CorePreparedReversalEvent,
    ) -> Result<Option<UndoOutcome>, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .persist_review_reversal(original_event_id, reversal)
            .map_err(|_| RuntimeError::new("review_unavailable"))
    }

    #[cfg(test)]
    pub(super) fn apply_normalizer_result(
        &self,
        document_id: &str,
        extraction_bundle: &ExtractionBundle,
        result: NormalizerResult,
    ) -> Result<SourceDocumentRoutingOutcome, RuntimeError> {
        let claim = {
            let mut store = self.store()?;
            let store = store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            let job = store
                .queued_parse_document_jobs()
                .map_err(|_| RuntimeError::new("classification_failed"))?
                .into_iter()
                .find(|job| job.document_id == document_id)
                .ok_or_else(|| RuntimeError::new("classification_failed"))?;
            store
                .start_parse_document_job(&job)
                .map_err(|_| RuntimeError::new("classification_failed"))?
                .ok_or_else(|| RuntimeError::new("classification_failed"))?
        };
        self.apply_normalizer_result_with_job(document_id, &claim, extraction_bundle, None, result)
    }

    pub(super) fn apply_normalizer_result_for_job(
        &self,
        claim: &ParseDocumentClaim,
        extraction: &ExtractedDocument,
        result: NormalizerResult,
    ) -> Result<SourceDocumentRoutingOutcome, RuntimeError> {
        self.apply_normalizer_result_with_job(
            &claim.document_id,
            claim,
            &extraction.bundle,
            Some(extraction.vault_session_generation),
            result,
        )
    }

    fn apply_normalizer_result_with_job(
        &self,
        document_id: &str,
        parse_job: &ParseDocumentClaim,
        extraction_bundle: &ExtractionBundle,
        vault_session_generation: Option<u64>,
        result: NormalizerResult,
    ) -> Result<SourceDocumentRoutingOutcome, RuntimeError> {
        let output_hash = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&result).map_err(|_| RuntimeError::new("normalizer_failed"))?,
            )
        );
        let (profile, proposal) = match result {
            NormalizerResult::Classified { profile, proposal } => (*profile, proposal),
            NormalizerResult::NeedsAttention { reason } => {
                let reason = if reason == "unsupported_document" {
                    "unsupported_document"
                } else {
                    "classification_uncertain"
                };
                return self.finish_normalizer_outcome_with_job(
                    parse_job,
                    vault_session_generation,
                    SourceDocumentRoutingOutcome::needs_attention(document_id, reason),
                );
            }
        };
        let Some(statement_id) = proposal.document.statement_id.as_deref() else {
            return self.finish_normalizer_outcome_with_job(
                parse_job,
                vault_session_generation,
                SourceDocumentRoutingOutcome::needs_attention(
                    document_id,
                    "classification_uncertain",
                ),
            );
        };
        if !valid_normalization_profile(&profile, &proposal, extraction_bundle) {
            return self.finish_normalizer_outcome_with_job(
                parse_job,
                vault_session_generation,
                SourceDocumentRoutingOutcome::needs_attention(
                    document_id,
                    "normalization_profile_invalid",
                ),
            );
        }
        if !valid_profiled_proposal(&proposal) {
            return self.finish_normalizer_outcome_with_job(
                parse_job,
                vault_session_generation,
                SourceDocumentRoutingOutcome::needs_attention(
                    document_id,
                    "classification_uncertain",
                ),
            );
        }
        let semantic_document_key = format!(
            "{}:{}",
            proposal.document.provider_key.as_str(),
            statement_id
        );
        let account_ids = (0..proposal.accounts.len())
            .map(|_| random_identifier("account"))
            .collect::<Vec<_>>();
        let accounts = proposal
            .accounts
            .iter()
            .zip(&account_ids)
            .map(|(account, account_id)| TrustedAccountCandidate {
                account_id,
                account_type: &account.account_type,
                currency: account.currency.as_deref(),
                display_name: "Detected account",
                masked_identifier: account.masked_identifier.as_deref(),
                provider_account_id: account.provider_account_id.as_deref(),
            })
            .collect::<Vec<_>>();
        let audit_id = random_identifier("audit");
        let classification = TrustedDocumentClassification {
            accounts: &accounts,
            audit_id: &audit_id,
            document_id,
            document_type: Some(&proposal.document.document_type),
            provider_key: &proposal.document.provider_key,
            semantic_document_key: &semantic_document_key,
            statement_period_from: proposal
                .document
                .statement_period
                .as_ref()
                .and_then(|period| period.from.as_deref()),
            statement_period_to: proposal
                .document
                .statement_period
                .as_ref()
                .and_then(|period| period.to.as_deref()),
        };
        self.require_parse_vault_session(vault_session_generation)?;
        let outcome = {
            let mut store = self.store()?;
            store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?
                .apply_trusted_classification_for_parse_job(&classification, parse_job)
                .map_err(|_| RuntimeError::new("classification_failed"))?
        };
        if outcome.status != crate::database::SourceDocumentRoutingStatus::Routed {
            return self.finish_normalizer_outcome_with_job(
                parse_job,
                vault_session_generation,
                outcome,
            );
        }
        let parse = validated_structured_parse_input(&proposal, &profile, &outcome.account_ids)
            .ok_or_else(|| RuntimeError::new("normalizer_failed"))?;
        self.require_parse_vault_session(vault_session_generation)?;
        {
            let mut store = self.store()?;
            store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?
                .persist_validated_structured_parse_for_claimed_job(
                    document_id,
                    &parse,
                    parse_job,
                    &extraction_bundle.file_sha256,
                    &output_hash,
                )
                .map_err(|_| RuntimeError::new("classification_failed"))?;
        }
        self.finish_normalizer_outcome_with_job(parse_job, vault_session_generation, outcome)
    }

    fn finish_normalizer_outcome_with_job(
        &self,
        parse_job: &ParseDocumentClaim,
        vault_session_generation: Option<u64>,
        outcome: SourceDocumentRoutingOutcome,
    ) -> Result<SourceDocumentRoutingOutcome, RuntimeError> {
        self.require_parse_vault_session(vault_session_generation)?;
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        finish_parse_job(store, parse_job, &outcome)?;
        Ok(outcome)
    }

    fn require_parse_vault_session(
        &self,
        vault_session_generation: Option<u64>,
    ) -> Result<(), RuntimeError> {
        if let Some(vault_session_generation) = vault_session_generation {
            self.require_vault_session(vault_session_generation)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn seed_money_source(
        &self,
        id: &str,
        provider_key: &str,
        display_name: &str,
        source_type: &str,
    ) -> Result<(), RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .seed_money_source(id, provider_key, display_name, source_type)
            .map_err(|_| RuntimeError::new("seed_failed"))
    }
}

fn finish_parse_job(
    store: &mut ManualImportStore,
    parse_job: &ParseDocumentClaim,
    outcome: &SourceDocumentRoutingOutcome,
) -> Result<(), RuntimeError> {
    store
        .finish_parse_document_job(parse_job, outcome)
        .map_err(|_| RuntimeError::new("classification_failed"))
}

pub(super) fn review_conflict(reason: &'static str) -> ReviewMutationOutcome {
    ReviewMutationOutcome {
        reason: Some(reason),
        record_version: None,
        review_item_id: None,
        status: ReviewMutationStatus::Conflict,
    }
}

pub(super) async fn resume_review_jobs_after_unlock(app: &AppHandle, runtime: VaultRuntime) {
    let job_ids = tauri::async_runtime::spawn_blocking({
        let runtime = runtime.clone();
        move || runtime.queued_review_job_ids()
    })
    .await
    .ok()
    .and_then(Result::ok)
    .unwrap_or_default();
    for job_id in job_ids {
        let job = tauri::async_runtime::spawn_blocking({
            let runtime = runtime.clone();
            move || runtime.review_job(&job_id)
        })
        .await
        .ok()
        .and_then(Result::ok)
        .flatten();
        if let Some(job) = job {
            let _ = process_review_batch_job(app, runtime.clone(), job).await;
        }
    }
}

#[tauri::command]
pub(crate) async fn list_review_items(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<ReviewItemSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_review_items()).await
}

#[tauri::command]
pub(crate) async fn get_review_detail(
    review_item_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<Option<ReviewItemDetail>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.review_item_detail(&review_item_id)).await
}

#[tauri::command]
pub(crate) async fn list_recent_activity(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<RecentActivitySummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_recent_activity()).await
}

#[tauri::command]
pub(crate) async fn get_money_overview(
    runtime: State<'_, VaultRuntime>,
) -> Result<MoneyOverview, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.money_overview()).await
}

#[tauri::command]
pub(crate) async fn list_relationship_candidates(
    review_item_id: String,
    expected_record_version: i64,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<RelationshipCandidateSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let candidate_input = {
        let runtime = runtime.clone();
        tauri::async_runtime::spawn_blocking(move || {
            runtime.relationship_candidate_input(&review_item_id, expected_record_version)
        })
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    let Some(ReviewRelationshipCandidateInput {
        candidates,
        event_type,
        record,
        ..
    }) = candidate_input
    else {
        return Ok(Vec::new());
    };
    if event_type.is_empty() {
        return Ok(Vec::new());
    }
    let result = run_review_core_sidecar(
        &app,
        "find_relationship_candidates",
        &RelationshipCandidatesInput {
            candidates: &candidates,
            event_type: &event_type,
            record: &record,
        },
    )
    .await
    .map_err(VaultCommandError::from)?;
    let ReviewCoreResult::Candidates {
        candidates: matches,
    } = result
    else {
        return Err(VaultCommandError::new("review_core_failed"));
    };
    if matches.len() != 1 {
        return Ok(Vec::new());
    }
    let candidate_id = &matches[0].id;
    if candidate_id.is_empty()
        || !candidates
            .iter()
            .any(|candidate| candidate.id == *candidate_id)
    {
        return Err(VaultCommandError::new("review_core_failed"));
    }
    let candidate_ids = vec![candidate_id.clone()];
    run_runtime_task(move || runtime.relationship_candidate_summaries(&candidate_ids)).await
}

#[tauri::command]
pub(crate) async fn edit_review_record(
    review_item_id: String,
    expected_record_version: i64,
    posted_on: Option<String>,
    amount_value: Option<String>,
    account_balance_delta: Option<String>,
    runtime: State<'_, VaultRuntime>,
) -> Result<ReviewMutationOutcome, VaultCommandError> {
    if review_item_id.is_empty() || expected_record_version <= 0 {
        return Err(VaultCommandError::new("invalid_review_request"));
    }
    let runtime = runtime.inner().clone();
    run_runtime_task(move || {
        runtime.edit_review_record(
            &review_item_id,
            expected_record_version,
            posted_on.as_deref(),
            amount_value.as_deref(),
            account_balance_delta.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn remove_review_record(
    review_item_id: String,
    expected_record_version: i64,
    runtime: State<'_, VaultRuntime>,
) -> Result<ReviewMutationOutcome, VaultCommandError> {
    if review_item_id.is_empty() || expected_record_version <= 0 {
        return Err(VaultCommandError::new("invalid_review_request"));
    }
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.remove_review_record(&review_item_id, expected_record_version))
        .await
}

#[tauri::command]
pub(crate) async fn accept_review_relationship(
    review_item_id: String,
    expected_record_version: i64,
    candidate_record_id: String,
    expected_candidate_version: i64,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<ReviewMutationOutcome, VaultCommandError> {
    if review_item_id.is_empty()
        || expected_record_version <= 0
        || candidate_record_id.is_empty()
        || expected_candidate_version <= 0
    {
        return Err(VaultCommandError::new("invalid_review_request"));
    }
    let runtime = runtime.inner().clone();
    let candidate_input = {
        let runtime = runtime.clone();
        let review_item_id = review_item_id.clone();
        tauri::async_runtime::spawn_blocking(move || {
            runtime.relationship_candidate_input(&review_item_id, expected_record_version)
        })
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    let Some(ReviewRelationshipCandidateInput {
        candidates,
        event_type,
        record,
        ..
    }) = candidate_input
    else {
        return Ok(review_conflict("stale_review_item"));
    };
    if event_type.is_empty() {
        return Ok(review_conflict("relationship_needs_review"));
    }
    let result = run_review_core_sidecar(
        &app,
        "find_relationship_candidates",
        &RelationshipCandidatesInput {
            candidates: &candidates,
            event_type: &event_type,
            record: &record,
        },
    )
    .await
    .map_err(VaultCommandError::from)?;
    let ReviewCoreResult::Candidates {
        candidates: matches,
    } = result
    else {
        return Err(VaultCommandError::new("review_core_failed"));
    };
    if !is_unique_requested_relationship_candidate(&matches, &candidate_record_id) {
        return Ok(review_conflict("relationship_needs_review"));
    }
    let Some(candidate) = candidates
        .into_iter()
        .find(|candidate| candidate.id == candidate_record_id)
    else {
        return Ok(review_conflict("relationship_needs_review"));
    };
    let records = [record, candidate];
    let result = run_review_core_sidecar(
        &app,
        "prepare_review_relationship",
        &RelationshipPreparationInput {
            event_type: &event_type,
            records: &records,
        },
    )
    .await
    .map_err(VaultCommandError::from)?;
    let event = match result {
        ReviewCoreResult::Ready {
            event: ReviewCoreReadyEvent::Relationship(event),
        } => event,
        ReviewCoreResult::Review { reasons } => {
            let _ = reasons;
            return Ok(review_conflict("relationship_needs_review"));
        }
        ReviewCoreResult::Ready {
            event: ReviewCoreReadyEvent::Reversal(_),
        }
        | ReviewCoreResult::Candidates { .. } => {
            return Err(VaultCommandError::new("review_core_failed"));
        }
    };
    run_runtime_task(move || {
        runtime.accept_review_relationship(
            &review_item_id,
            expected_record_version,
            &candidate_record_id,
            expected_candidate_version,
            &event,
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn enqueue_commit_review_batch(
    review_item_ids: Vec<String>,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<ReviewJobSummary, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let job = {
        let runtime = runtime.clone();
        let review_item_ids = review_item_ids.clone();
        tauri::async_runtime::spawn_blocking(move || {
            runtime.enqueue_commit_review_batch(&review_item_ids)
        })
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    process_review_batch_job(&app, runtime, job).await
}

pub(super) async fn process_review_batch_job(
    app: &AppHandle,
    runtime: VaultRuntime,
    job: ReviewJobSummary,
) -> Result<ReviewJobSummary, VaultCommandError> {
    let job_id = job.job_id.clone();
    let lease_owner = random_identifier("review-worker");
    let claimed = {
        let runtime = runtime.clone();
        let job_id = job_id.clone();
        let lease_owner = lease_owner.clone();
        tauri::async_runtime::spawn_blocking(move || {
            runtime.claim_review_batch(&job_id, &lease_owner)
        })
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    let Some(claimed) = claimed else {
        return Ok(job);
    };
    let (groups, mut outcomes) = {
        let runtime = runtime.clone();
        let claimed = claimed.clone();
        tauri::async_runtime::spawn_blocking(move || runtime.prepare_commit_review_groups(&claimed))
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    for group in groups {
        let result = run_review_core_sidecar(
            app,
            "prepare_review_relationship",
            &RelationshipPreparationInput {
                event_type: &group.event_type,
                records: &group.records,
            },
        )
        .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let failure_runtime = runtime.clone();
                let failure_claimed = claimed.clone();
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    failure_runtime.fail_review_batch(&failure_claimed)
                })
                .await;
                return Err(error.into());
            }
        };
        match result {
            ReviewCoreResult::Ready {
                event: ReviewCoreReadyEvent::Relationship(event),
            } => {
                let outcome = {
                    let runtime = runtime.clone();
                    let claimed = claimed.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        runtime.commit_prepared_review_group(&claimed, &group, &event)
                    })
                    .await
                    .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
                };
                outcomes.push(outcome);
            }
            ReviewCoreResult::Review { reasons } => {
                let _ = reasons;
                outcomes.push(ReviewBatchGroupOutcome {
                    reason: Some("relationship_needs_review".to_owned()),
                    record_ids: group
                        .records
                        .iter()
                        .map(|record| record.id.clone())
                        .collect(),
                    status: ReviewBatchGroupStatus::StillNeedsReview,
                });
            }
            ReviewCoreResult::Ready {
                event: ReviewCoreReadyEvent::Reversal(_),
            }
            | ReviewCoreResult::Candidates { .. } => {
                let failure_runtime = runtime.clone();
                let failure_claimed = claimed.clone();
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    failure_runtime.fail_review_batch(&failure_claimed)
                })
                .await;
                return Err(VaultCommandError::new("review_core_failed"));
            }
        }
    }
    run_runtime_task(move || runtime.finish_review_batch(&claimed, &outcomes)).await
}

#[tauri::command]
pub(crate) async fn get_review_job(
    job_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<Option<ReviewJobSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.review_job(&job_id)).await
}
