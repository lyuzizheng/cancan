use super::*;
use serde::Serialize;

const MONEY_SOURCE_CANDIDATE_KEY_VERSION: i64 = 1;
const SOURCE_CONFIRMATION_POLICY_VERSION: &str = "source-confirmation-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum SourceConfirmationScopeKind {
    ProviderSingleton,
    ProviderRootId,
}

impl SourceConfirmationScopeKind {
    fn from_database(value: &str) -> StoreResult<Self> {
        match value {
            "provider_singleton" => Ok(Self::ProviderSingleton),
            "provider_root_id" => Ok(Self::ProviderRootId),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Money Source candidate scope",
            )
            .into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum SourceConfirmationPromptStatus {
    Pending,
    KeptUnassigned,
}

impl SourceConfirmationPromptStatus {
    fn from_database(value: &str) -> StoreResult<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "kept_unassigned" => Ok(Self::KeptUnassigned),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Money Source candidate status",
            )
            .into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct SourceConfirmationPrompt {
    pub(crate) candidate_id: String,
    pub(crate) document_count: i64,
    pub(crate) latest_document_id: Option<String>,
    pub(crate) latest_document_title: Option<String>,
    pub(crate) provider_key: String,
    pub(crate) scope_kind: SourceConfirmationScopeKind,
    pub(crate) status: SourceConfirmationPromptStatus,
    pub(crate) version: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MoneySourceCandidateScope<'a> {
    ProviderRootId(&'a str),
    ProviderSingleton,
}

impl<'a> MoneySourceCandidateScope<'a> {
    fn parts(self) -> (&'static str, &'a str) {
        match self {
            Self::ProviderRootId(value) => ("provider_root_id", value),
            Self::ProviderSingleton => ("provider_singleton", ""),
        }
    }
}

pub(crate) struct MoneySourceCandidateInput<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) document_id: &'a str,
    pub(crate) provider_key: &'a str,
    pub(crate) scope: MoneySourceCandidateScope<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum MoneySourceCandidateStatus {
    Confirmed,
    KeptUnassigned,
    Pending,
}

impl MoneySourceCandidateStatus {
    fn from_database(value: &str) -> StoreResult<Self> {
        match value {
            "confirmed" => Ok(Self::Confirmed),
            "kept_unassigned" => Ok(Self::KeptUnassigned),
            "pending" => Ok(Self::Pending),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Money Source candidate status",
            )
            .into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct MoneySourceCandidateState {
    pub(crate) candidate_id: String,
    pub(crate) confirmed_money_source_id: Option<String>,
    pub(crate) status: MoneySourceCandidateStatus,
    pub(crate) version: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct ConfirmedMoneySourceCandidate {
    pub(crate) candidate_id: String,
    pub(crate) money_source_id: String,
    pub(crate) version: i64,
}

pub(crate) struct ConfirmMoneySourceCandidateInput<'a> {
    pub(crate) audit_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) display_name: &'a str,
    pub(crate) expected_version: i64,
    pub(crate) proposed_money_source_id: &'a str,
    pub(crate) source_type: &'a str,
}

impl ManualImportStore {
    pub(crate) fn attach_money_source_candidate(
        &mut self,
        input: &MoneySourceCandidateInput<'_>,
    ) -> StoreResult<MoneySourceCandidateState> {
        let transaction = self.connection.transaction()?;
        let state = Self::attach_money_source_candidate_in_transaction(&transaction, input)?;
        transaction.commit()?;
        Ok(state)
    }

    /// Attaches (or resolves) the source candidate inside a caller-owned
    /// transaction, so a document classification and its candidate parking
    /// commit as one unit under one claim check.
    pub(crate) fn attach_money_source_candidate_in_transaction(
        transaction: &Transaction<'_>,
        input: &MoneySourceCandidateInput<'_>,
    ) -> StoreResult<MoneySourceCandidateState> {
        validate_candidate_input(input)?;
        let (scope_kind, scope_value) = input.scope.parts();
        let document_owner = transaction
            .query_row(
                "SELECT money_source_id, money_source_candidate_id \
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
        let Some((money_source_id, current_candidate_id)) = document_owner else {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        };
        if money_source_id.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "source document is already assigned",
            )
            .into());
        }
        transaction.execute(
            "INSERT INTO money_source_candidates( \
               id, candidate_key_version, provider_key, candidate_scope_kind, \
               candidate_scope_value, status \
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending') \
             ON CONFLICT(candidate_key_version, provider_key, candidate_scope_kind, \
                         candidate_scope_value) DO NOTHING",
            params![
                input.candidate_id,
                MONEY_SOURCE_CANDIDATE_KEY_VERSION,
                input.provider_key,
                scope_kind,
                scope_value,
            ],
        )?;
        let state =
            read_candidate_by_identity(transaction, input.provider_key, scope_kind, scope_value)?;
        if current_candidate_id
            .as_deref()
            .is_some_and(|candidate_id| candidate_id != state.candidate_id)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "source document has a different candidate owner",
            )
            .into());
        }
        match state.status {
            MoneySourceCandidateStatus::Pending => {
                transaction.execute(
                    "UPDATE source_documents \
                     SET money_source_candidate_id = ?1, attention_parked_reason = NULL, \
                         attention_parked_at = NULL \
                     WHERE id = ?2",
                    params![state.candidate_id, input.document_id],
                )?;
            }
            MoneySourceCandidateStatus::KeptUnassigned => {
                transaction.execute(
                    "UPDATE source_documents \
                     SET money_source_candidate_id = ?1, \
                         attention_parked_at = CASE \
                           WHEN attention_parked_reason = 'source_confirmation' \
                             AND attention_parked_at IS NOT NULL \
                           THEN attention_parked_at ELSE CURRENT_TIMESTAMP END, \
                         attention_parked_reason = 'source_confirmation' \
                     WHERE id = ?2",
                    params![state.candidate_id, input.document_id],
                )?;
            }
            MoneySourceCandidateStatus::Confirmed => {
                let money_source_id =
                    state.confirmed_money_source_id.as_deref().ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "confirmed candidate has no Money Source",
                        )
                    })?;
                transaction.execute(
                    "UPDATE source_documents \
                     SET money_source_id = ?1, money_source_candidate_id = NULL, \
                         attention_parked_reason = NULL, attention_parked_at = NULL \
                     WHERE id = ?2",
                    params![money_source_id, input.document_id],
                )?;
            }
        }
        Ok(state)
    }

    pub(crate) fn keep_money_source_candidate_unassigned(
        &mut self,
        candidate_id: &str,
        expected_version: i64,
    ) -> StoreResult<MoneySourceCandidateState> {
        validate_identifier(candidate_id, "Money Source candidate id")?;
        if !(1..i64::MAX).contains(&expected_version) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid Money Source candidate version",
            )
            .into());
        }
        let transaction = self.connection.transaction()?;
        let current = read_candidate_by_id(&transaction, candidate_id)?;
        if current.status == MoneySourceCandidateStatus::KeptUnassigned
            && current.version == expected_version + 1
        {
            return Ok(current);
        }
        if current.status != MoneySourceCandidateStatus::Pending
            || current.version != expected_version
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Money Source candidate changed",
            )
            .into());
        }
        let changed = transaction.execute(
            "UPDATE money_source_candidates \
             SET status = 'kept_unassigned', version = version + 1, \
                 updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?1 AND status = 'pending' AND version = ?2",
            params![candidate_id, expected_version],
        )?;
        if changed != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Money Source candidate changed concurrently",
            )
            .into());
        }
        transaction.execute(
            "UPDATE source_documents \
             SET attention_parked_at = CASE \
                   WHEN attention_parked_reason = 'source_confirmation' \
                     AND attention_parked_at IS NOT NULL \
                   THEN attention_parked_at ELSE CURRENT_TIMESTAMP END, \
                 attention_parked_reason = 'source_confirmation' \
             WHERE money_source_candidate_id = ?1 AND money_source_id IS NULL",
            [candidate_id],
        )?;
        let state = read_candidate_by_id(&transaction, candidate_id)?;
        transaction.commit()?;
        Ok(state)
    }

    pub(crate) fn confirm_money_source_candidate(
        &mut self,
        input: &ConfirmMoneySourceCandidateInput<'_>,
    ) -> StoreResult<ConfirmedMoneySourceCandidate> {
        validate_confirmation_input(input)?;
        let transaction = self.connection.transaction()?;
        let current = read_candidate_by_id(&transaction, input.candidate_id)?;
        if current.status == MoneySourceCandidateStatus::Confirmed {
            return Ok(ConfirmedMoneySourceCandidate {
                candidate_id: current.candidate_id,
                money_source_id: current.confirmed_money_source_id.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "confirmed candidate has no Money Source",
                    )
                })?,
                version: current.version,
            });
        }
        if current.version != input.expected_version {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Money Source candidate changed",
            )
            .into());
        }
        let (provider_key, scope_kind, scope_value): (String, String, String) = transaction
            .query_row(
                "SELECT provider_key, candidate_scope_kind, candidate_scope_value \
                 FROM money_source_candidates WHERE id = ?1",
                [input.candidate_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        let money_source_id = match scope_kind.as_str() {
            "provider_root_id" => {
                let mut statement = transaction.prepare(
                    "SELECT id FROM money_sources \
                     WHERE provider_key = ?1 AND provider_root_id = ?2 \
                     ORDER BY id LIMIT 2",
                )?;
                let existing_sources = statement
                    .query_map(params![&provider_key, &scope_value], |row| {
                        row.get::<_, String>(0)
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                drop(statement);
                match existing_sources.as_slice() {
                    [] => {
                        transaction.execute(
                            "INSERT INTO money_sources( \
                               id, provider_key, provider_root_id, display_name, source_type \
                             ) VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![
                                input.proposed_money_source_id,
                                &provider_key,
                                &scope_value,
                                input.display_name,
                                input.source_type,
                            ],
                        )?;
                        input.proposed_money_source_id.to_owned()
                    }
                    [money_source_id] => money_source_id.clone(),
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Money Source candidate matches multiple configured sources",
                        )
                        .into());
                    }
                }
            }
            "provider_singleton" => {
                let mut statement = transaction.prepare(
                    "SELECT id FROM money_sources \
                     WHERE provider_key = ?1 AND provider_root_id IS NULL \
                     ORDER BY id LIMIT 2",
                )?;
                let existing_sources = statement
                    .query_map([&provider_key], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                drop(statement);
                match existing_sources.as_slice() {
                    [] => {
                        transaction.execute(
                            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
                             VALUES (?1, ?2, ?3, ?4)",
                            params![
                                input.proposed_money_source_id,
                                &provider_key,
                                input.display_name,
                                input.source_type,
                            ],
                        )?;
                        input.proposed_money_source_id.to_owned()
                    }
                    [money_source_id] => money_source_id.clone(),
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Money Source candidate matches multiple configured sources",
                        )
                        .into());
                    }
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unknown Money Source candidate scope",
                )
                .into());
            }
        };
        let changed = transaction.execute(
            "UPDATE money_source_candidates \
             SET status = 'confirmed', version = version + 1, \
                 confirmed_money_source_id = ?1, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?2 AND status IN ('pending', 'kept_unassigned') AND version = ?3",
            params![money_source_id, input.candidate_id, input.expected_version],
        )?;
        if changed != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Money Source candidate changed concurrently",
            )
            .into());
        }
        let document_ids: Vec<String> = {
            let mut statement = transaction.prepare(
                "SELECT id FROM source_documents \
                 WHERE money_source_candidate_id = ?1 AND money_source_id IS NULL \
                 ORDER BY id",
            )?;
            let ids = statement
                .query_map([input.candidate_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            drop(statement);
            ids
        };
        transaction.execute(
            "UPDATE source_documents \
             SET money_source_id = ?1, money_source_candidate_id = NULL, \
                 attention_parked_reason = NULL, attention_parked_at = NULL \
             WHERE money_source_candidate_id = ?2 AND money_source_id IS NULL",
            params![money_source_id, input.candidate_id],
        )?;
        for document_id in &document_ids {
            let active: bool = transaction.query_row(
                "SELECT EXISTS( \
                   SELECT 1 FROM jobs \
                   WHERE related_source_document_id = ?1 AND job_type = ?2 \
                     AND status IN ('queued', 'running') \
                 )",
                params![document_id, crate::database::PARSE_DOCUMENT_JOB_TYPE],
                |row| row.get(0),
            )?;
            if !active {
                crate::database::enqueue_parse_document(
                    &transaction,
                    document_id,
                    &new_database_id("parse-run"),
                )?;
            }
        }
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'money_source_candidate', ?2, \
                       'money_source_candidate_confirmed', 'user', \
                       'user_confirmed_trusted_source', ?3, ?4)",
            params![
                input.audit_id,
                input.candidate_id,
                money_source_id,
                SOURCE_CONFIRMATION_POLICY_VERSION,
            ],
        )?;
        transaction.commit()?;
        Ok(ConfirmedMoneySourceCandidate {
            candidate_id: input.candidate_id.to_owned(),
            money_source_id,
            version: input.expected_version + 1,
        })
    }

    pub(crate) fn list_source_confirmation_prompts(
        &self,
    ) -> StoreResult<Vec<SourceConfirmationPrompt>> {
        let mut statement = self.connection.prepare(
            "SELECT c.id, c.provider_key, c.candidate_scope_kind, c.status, c.version, \
                    (SELECT COUNT(*) FROM source_documents sd \
                     WHERE sd.money_source_candidate_id = c.id \
                       AND sd.money_source_id IS NULL), \
                    (SELECT sd.original_filename FROM source_documents sd \
                     WHERE sd.money_source_candidate_id = c.id \
                       AND sd.money_source_id IS NULL \
                     ORDER BY sd.received_at DESC, sd.id DESC LIMIT 1), \
                    (SELECT sd.id FROM source_documents sd \
                     WHERE sd.money_source_candidate_id = c.id \
                       AND sd.money_source_id IS NULL \
                     ORDER BY sd.received_at DESC, sd.id DESC LIMIT 1) \
             FROM money_source_candidates c \
             WHERE c.status IN ('pending', 'kept_unassigned') \
             ORDER BY c.updated_at, c.id",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(SourceConfirmationPrompt {
                    candidate_id: row.get(0)?,
                    provider_key: row.get(1)?,
                    scope_kind: SourceConfirmationScopeKind::from_database(
                        &row.get::<_, String>(2)?,
                    )
                    .map_err(rusqlite::Error::ToSqlConversionFailure)?,
                    status: SourceConfirmationPromptStatus::from_database(
                        &row.get::<_, String>(3)?,
                    )
                    .map_err(rusqlite::Error::ToSqlConversionFailure)?,
                    version: row.get(4)?,
                    document_count: row.get(5)?,
                    latest_document_title: row.get(6)?,
                    latest_document_id: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

fn validate_candidate_input(input: &MoneySourceCandidateInput<'_>) -> StoreResult<()> {
    validate_identifier(input.candidate_id, "Money Source candidate id")?;
    validate_identifier(input.document_id, "source document id")?;
    if input.provider_key.is_empty() || input.provider_key.len() > 128 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid provider key").into());
    }
    match input.scope {
        MoneySourceCandidateScope::ProviderRootId(value)
            if value.is_empty() || value.len() > 512 =>
        {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid provider root identity",
            )
            .into())
        }
        _ => Ok(()),
    }
}

fn validate_confirmation_input(input: &ConfirmMoneySourceCandidateInput<'_>) -> StoreResult<()> {
    validate_identifier(input.audit_id, "audit id")?;
    validate_identifier(input.candidate_id, "Money Source candidate id")?;
    validate_identifier(input.proposed_money_source_id, "Money Source id")?;
    if !(1..i64::MAX).contains(&input.expected_version)
        || input.display_name.is_empty()
        || input.display_name.len() > 256
        || input.source_type.is_empty()
        || input.source_type.len() > 128
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid Money Source confirmation",
        )
        .into());
    }
    Ok(())
}

fn read_candidate_by_identity(
    transaction: &Transaction<'_>,
    provider_key: &str,
    scope_kind: &str,
    scope_value: &str,
) -> StoreResult<MoneySourceCandidateState> {
    let state = transaction.query_row(
        "SELECT id, status, version, confirmed_money_source_id \
         FROM money_source_candidates \
         WHERE candidate_key_version = ?1 AND provider_key = ?2 \
           AND candidate_scope_kind = ?3 AND candidate_scope_value = ?4",
        params![
            MONEY_SOURCE_CANDIDATE_KEY_VERSION,
            provider_key,
            scope_kind,
            scope_value,
        ],
        candidate_state_from_row,
    )?;
    Ok(state)
}

fn read_candidate_by_id(
    transaction: &Transaction<'_>,
    candidate_id: &str,
) -> StoreResult<MoneySourceCandidateState> {
    let state = transaction
        .query_row(
            "SELECT id, status, version, confirmed_money_source_id \
             FROM money_source_candidates WHERE id = ?1",
            [candidate_id],
            candidate_state_from_row,
        )
        .optional()?;
    state.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Money Source candidate not found").into()
    })
}

fn candidate_state_from_row(row: &Row<'_>) -> rusqlite::Result<MoneySourceCandidateState> {
    let status = row.get::<_, String>(1)?;
    let status = MoneySourceCandidateStatus::from_database(&status)
        .map_err(rusqlite::Error::ToSqlConversionFailure)?;
    Ok(MoneySourceCandidateState {
        candidate_id: row.get(0)?,
        status,
        version: row.get(2)?,
        confirmed_money_source_id: row.get(3)?,
    })
}
