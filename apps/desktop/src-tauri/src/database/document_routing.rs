use super::*;

impl ManualImportStore {
    /// Applies the trusted classification and, when the document routes, the
    /// structured parse derived from it in one transaction. `build_parse`
    /// receives the routed outcome (which carries the resolved account ids) and
    /// returns the parse payload to persist, or `None` when the proposal cannot
    /// produce one — the whole call then rolls back, so a parse job can never
    /// leave a classified document with no parse record.
    pub(crate) fn apply_classification_and_parse_for_claimed_job(
        &mut self,
        input: &TrustedDocumentClassification<'_>,
        claim: &ParseDocumentClaim,
        input_hash: &str,
        output_hash: &str,
        build_parse: impl FnOnce(&SourceDocumentRoutingOutcome) -> Option<ValidatedStructuredParseInput>,
    ) -> StoreResult<SourceDocumentRoutingOutcome> {
        self.apply_trusted_classification_with_parse_job(
            input,
            Some(ParseJobContext {
                claim,
                input_hash,
                output_hash,
            }),
            build_parse,
        )
    }

    fn apply_trusted_classification_with_parse_job(
        &mut self,
        input: &TrustedDocumentClassification<'_>,
        parse_job: Option<ParseJobContext<'_>>,
        build_parse: impl FnOnce(&SourceDocumentRoutingOutcome) -> Option<ValidatedStructuredParseInput>,
    ) -> StoreResult<SourceDocumentRoutingOutcome> {
        validate_classification(input)?;
        let transaction = self.connection.transaction()?;
        if let Some(job) = parse_job
            && !parse_job_claimed(&transaction, job.claim)?
        {
            return Err(io::Error::other("parse job is no longer claimed").into());
        }
        let existing_identity = transaction
            .query_row(
                "SELECT money_source_id, semantic_document_key \
                 FROM source_documents WHERE id = ?1",
                [input.document_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?;
        let Some((assigned_source_id, semantic_document_key)) = existing_identity else {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        };

        let source_ids = if let Some(provider_root_id) = input.provider_root_id {
            let mut source_statement = transaction.prepare(
                "SELECT id FROM money_sources \
                 WHERE provider_key = ?1 AND provider_root_id = ?2 \
                 ORDER BY id LIMIT 2",
            )?;
            source_statement
                .query_map(params![input.provider_key, provider_root_id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut source_statement = transaction.prepare(
                "SELECT id FROM money_sources \
                 WHERE provider_key = ?1 AND provider_root_id IS NULL \
                 ORDER BY id LIMIT 2",
            )?;
            source_statement
                .query_map([input.provider_key], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        let money_source_id = match source_ids.as_slice() {
            [] => {
                let candidate_id = new_database_id("source-candidate");
                let scope = match input.provider_root_id {
                    Some(root_id) => {
                        crate::database::intake::MoneySourceCandidateScope::ProviderRootId(root_id)
                    }
                    None => crate::database::intake::MoneySourceCandidateScope::ProviderSingleton,
                };
                Self::attach_money_source_candidate_in_transaction(
                    &transaction,
                    &crate::database::intake::MoneySourceCandidateInput {
                        candidate_id: &candidate_id,
                        document_id: input.document_id,
                        provider_key: input.provider_key,
                        scope,
                    },
                )?;
                transaction.commit()?;
                return Ok(SourceDocumentRoutingOutcome::needs_attention(
                    input.document_id,
                    "source_confirmation_required",
                ));
            }
            [money_source_id] => money_source_id,
            _ => {
                return Ok(SourceDocumentRoutingOutcome::needs_attention(
                    input.document_id,
                    "money_source_ambiguous",
                ));
            }
        };
        if assigned_source_id
            .as_deref()
            .is_some_and(|id| id != money_source_id)
            || semantic_document_key
                .as_deref()
                .is_some_and(|key| key != input.semantic_document_key)
        {
            return Ok(SourceDocumentRoutingOutcome::needs_attention(
                input.document_id,
                "classification_conflict",
            ));
        }

        ensure_fiat_currency_instruments(&transaction, input.accounts)?;

        let mut account_ids = Vec::with_capacity(input.accounts.len());
        for account in input.accounts {
            let Some(provider_account_id) = account.provider_account_id else {
                return Ok(SourceDocumentRoutingOutcome::needs_attention(
                    input.document_id,
                    "account_mapping_needed",
                ));
            };
            let existing = transaction
                .query_row(
                    "SELECT id, status FROM accounts \
                     WHERE money_source_id = ?1 AND provider_key = ?2 \
                       AND provider_account_id = ?3 AND status <> 'merged'",
                    params![money_source_id, input.provider_key, provider_account_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            match existing {
                Some((_, status)) if status == "archived" => {
                    return Ok(SourceDocumentRoutingOutcome::needs_attention(
                        input.document_id,
                        "account_restore_required",
                    ));
                }
                Some((account_id, _)) => account_ids.push(account_id),
                None => {
                    transaction.execute(
                        "INSERT INTO accounts( \
                           id, money_source_id, provider_key, provider_account_id, account_type, \
                           display_name, masked_identifier, currency, status \
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'candidate')",
                        params![
                            account.account_id,
                            money_source_id,
                            input.provider_key,
                            provider_account_id,
                            account.account_type,
                            account.display_name,
                            account.masked_identifier,
                            account.currency,
                        ],
                    )?;
                    account_ids.push(account.account_id.to_owned());
                }
            }
        }

        transaction.execute(
            "UPDATE source_documents \
             SET money_source_id = ?1, semantic_document_key = ?2, \
                 document_type = ?3, statement_period_from = ?4, statement_period_to = ?5 \
             WHERE id = ?6",
            params![
                money_source_id,
                input.semantic_document_key,
                input.document_type,
                input.statement_period_from,
                input.statement_period_to,
                input.document_id,
            ],
        )?;
        transaction.execute(
            "DELETE FROM source_document_accounts WHERE source_document_id = ?1",
            [input.document_id],
        )?;
        for account_id in &account_ids {
            transaction.execute(
                "INSERT INTO source_document_accounts(source_document_id, account_id) \
                 VALUES (?1, ?2)",
                params![input.document_id, account_id],
            )?;
        }
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES (?1, 'source_document', ?2, 'trusted_classification_applied', \
                       'system', 'provider_and_account_verified', 'classification-v1')",
            params![input.audit_id, input.document_id],
        )?;
        let outcome = SourceDocumentRoutingOutcome {
            account_ids,
            document_id: input.document_id.to_owned(),
            money_source_id: Some(money_source_id.to_owned()),
            reason: None,
            status: SourceDocumentRoutingStatus::Routed,
        };
        if let Some(job) = parse_job {
            let parse = build_parse(&outcome).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "normalizer result produced no validated parse",
                )
            })?;
            persist_validated_structured_parse_in_transaction(
                &transaction,
                input.document_id,
                &parse,
                job.claim.logical_run_key.as_str(),
                job.input_hash,
                job.output_hash,
                job.claim,
            )?;
        }
        transaction.commit()?;
        Ok(outcome)
    }

    /// Test-only entry point that writes one validated structured parse for a
    /// claimed job without a classification, so store and pipeline tests can
    /// reach parse states directly. Production writes a parse only together
    /// with its classification in
    /// [`Self::apply_classification_and_parse_for_claimed_job`].
    #[cfg(test)]
    pub(crate) fn persist_validated_structured_parse_for_claimed_job(
        &mut self,
        document_id: &str,
        input: &ValidatedStructuredParseInput,
        claim: &ParseDocumentClaim,
        input_hash: &str,
        output_hash: &str,
    ) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        if !parse_job_claimed(&transaction, claim)? {
            return Err(io::Error::other("parse job is no longer claimed").into());
        }
        persist_validated_structured_parse_in_transaction(
            &transaction,
            document_id,
            input,
            &claim.logical_run_key,
            input_hash,
            output_hash,
            claim,
        )?;
        transaction.commit()?;
        Ok(())
    }
}

/// Test-only entry point for a classification with no parse job. The store API
/// itself lives in [`super::database_test_support`]; this bridge keeps
/// [`ManualImportStore::apply_trusted_classification_with_parse_job`] private.
#[cfg(test)]
pub(super) fn test_support_apply_trusted_classification(
    store: &mut ManualImportStore,
    input: &TrustedDocumentClassification<'_>,
) -> StoreResult<SourceDocumentRoutingOutcome> {
    store.apply_trusted_classification_with_parse_job(input, None, |_| None)
}

/// Persists one validated structured parse inside a caller-owned transaction
/// whose claim has already been checked.
fn persist_validated_structured_parse_in_transaction(
    transaction: &Transaction<'_>,
    document_id: &str,
    input: &ValidatedStructuredParseInput,
    logical_run_key: &str,
    input_hash: &str,
    output_hash: &str,
    parse_job: &ParseDocumentClaim,
) -> StoreResult<()> {
    validate_structured_parse_input(document_id, input)?;
    if logical_run_key.is_empty()
        || input_hash.is_empty()
        || output_hash.is_empty()
        || logical_run_key.len() > 256
        || input_hash.len() > 128
        || output_hash.len() > 128
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid parse run key").into());
    }
    if !parse_job_claimed(transaction, parse_job)? {
        return Err(io::Error::other("parse job is no longer claimed").into());
    }
    let document_exists: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM source_documents WHERE id = ?1)",
        [document_id],
        |row| row.get(0),
    )?;
    if !document_exists {
        return Err(io::Error::new(io::ErrorKind::NotFound, "source document not found").into());
    }
    let existing_run = transaction
        .query_row(
            "SELECT profile_json, input_hash, output_hash FROM parse_runs \
                 WHERE source_document_id = ?1 AND logical_run_key = ?2",
            params![document_id, logical_run_key],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    if let Some((profile_json, existing_input_hash, existing_output_hash)) = existing_run {
        if profile_json != input.profile_json
            || existing_input_hash != input_hash
            || existing_output_hash.as_deref() != Some(output_hash)
        {
            return Err(
                io::Error::new(io::ErrorKind::InvalidData, "logical parse run changed").into(),
            );
        }
        return Ok(());
    }

    let parse_run_id = new_database_id("parse");
    transaction.execute(
        "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, logical_run_key, profile_json, \
               input_hash, output_hash, status \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'validated')",
        params![
            parse_run_id,
            document_id,
            input.normalization_profile_id,
            logical_run_key,
            input.profile_json,
            input_hash,
            output_hash,
        ],
    )?;
    for record in &input.records {
        let committed_record = transaction
                .query_row(
                    "SELECT id, record_type, event_type, posted_on, amount_value, currency, account_balance_delta, raw_json \
                     FROM external_records \
                     WHERE stable_record_key = ?1 AND status = 'committed' \
                     ORDER BY version DESC LIMIT 1",
                    [&record.stable_record_key],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, String>(7)?,
                        ))
                    },
                )
                .optional()?;

        if let Some((
            committed_id,
            committed_record_type,
            committed_event_type,
            committed_posted_on,
            committed_amount_value,
            committed_currency,
            committed_account_balance_delta,
            committed_raw_json,
        )) = committed_record
        {
            let diverges = committed_record_type != record.record_type
                || committed_event_type != record.event_type
                || committed_posted_on != record.posted_on
                || committed_amount_value != record.amount_value
                || committed_currency != record.currency
                || committed_account_balance_delta != record.account_balance_delta;

            if diverges {
                let review_item_id = new_database_id("review");
                transaction.execute(
                    "INSERT INTO review_items(id, external_record_id, reason_code, status) \
                         SELECT ?1, ?2, 'reparse_divergence', 'open' \
                         WHERE NOT EXISTS ( \
                           SELECT 1 FROM review_items \
                           WHERE external_record_id = ?2 \
                             AND reason_code = 'reparse_divergence' \
                             AND status = 'open' \
                         )",
                    params![review_item_id, committed_id],
                )?;
            }

            let old_raw_sha256 = format!("{:x}", Sha256::digest(committed_raw_json.as_bytes()));
            let new_raw_sha256 = format!("{:x}", Sha256::digest(record.raw_json.as_bytes()));
            let source_ref = format!(
                "{}:{old_raw_sha256}:{new_raw_sha256}",
                record.stable_record_key
            );

            transaction.execute(
                    "INSERT INTO audit_log( \
                       id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
                     ) VALUES (?1, 'external_record', ?2, 'reparse_skipped_committed_record', 'system', \
                               'reparse', ?3, ?4)",
                    params![
                        new_audit_id(),
                        committed_id,
                        source_ref,
                        REVIEW_POLICY_VERSION,
                    ],
                )?;

            continue;
        }

        let previous = transaction
            .query_row(
                "SELECT id, version FROM external_records \
                     WHERE stable_record_key = ?1 ORDER BY version DESC LIMIT 1",
                [&record.stable_record_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        let version = previous.as_ref().map_or(1, |(_, version)| version + 1);
        if previous.is_some() {
            transaction.execute(
                "UPDATE external_records SET status = 'superseded' \
                     WHERE stable_record_key = ?1 AND status IN ('staged', 'review', 'removed')",
                [&record.stable_record_key],
            )?;
            transaction.execute(
                "UPDATE review_items SET status = 'resolved' \
                     WHERE external_record_id IN ( \
                       SELECT id FROM external_records \
                       WHERE stable_record_key = ?1 AND status = 'superseded' \
                     ) AND status = 'open'",
                [&record.stable_record_key],
            )?;
        }
        let dismissed: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = ?1 AND status = 'dismissed')",
            [&record.account_id],
            |row| row.get(0),
        )?;
        transaction.execute(
            "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                   status, record_type, event_type, posted_on, amount_value, currency, \
                   account_balance_delta, posting_status, raw_json, validation_json \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                new_database_id("record"),
                parse_run_id,
                document_id,
                record.account_id,
                record.stable_record_key,
                version,
                if dismissed { "removed" } else { "staged" },
                record.record_type,
                record.event_type,
                record.posted_on,
                record.amount_value,
                record.currency,
                record.account_balance_delta,
                record.posting_status,
                record.raw_json,
                record.validation_json,
            ],
        )?;
    }
    Ok(())
}
