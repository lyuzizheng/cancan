#![allow(dead_code)]

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StatementCoveragePolicy<'a> {
    pub(crate) cadence_months: u32,
    pub(crate) document_type: &'a str,
    pub(crate) grace_days: i64,
    pub(crate) provider_key: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatementCoveragePromptStatus {
    ConfirmedMissing,
    LikelyMissing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatementCoveragePrompt {
    pub(crate) account_id: String,
    pub(crate) document_type: String,
    pub(crate) money_source_id: String,
    pub(crate) statement_period_from: String,
    pub(crate) statement_period_to: String,
    pub(crate) status: StatementCoveragePromptStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatementCoverageDecision {
    NotExpected,
    RemindLater,
}

pub(crate) struct StatementCoverageDecisionInput<'a> {
    pub(crate) account_id: &'a str,
    pub(crate) audit_id: &'a str,
    pub(crate) decision: StatementCoverageDecision,
    pub(crate) document_type: &'a str,
    pub(crate) money_source_id: &'a str,
    pub(crate) remind_after: Option<&'a str>,
    pub(crate) statement_period_from: &'a str,
    pub(crate) statement_period_to: &'a str,
}

#[derive(Clone, Debug)]
struct CoveragePeriod {
    statement_period_from: String,
    statement_period_to: String,
}

impl ManualImportStore {
    pub(crate) fn statement_coverage_today(&self) -> StoreResult<String> {
        Ok(self
            .connection
            .query_row("SELECT date('now')", [], |row| row.get(0))?)
    }

    pub(crate) fn list_statement_coverage_prompts(
        &self,
        policies: &[StatementCoveragePolicy<'_>],
        today: &str,
    ) -> StoreResult<Vec<StatementCoveragePrompt>> {
        if !valid_iso_date(today)
            || policies
                .iter()
                .any(|policy| policy.cadence_months == 0 || policy.grace_days < 0)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid statement coverage policy or date",
            )
            .into());
        }
        let mut statement = self.connection.prepare(
            "SELECT money_sources.provider_key, source_documents.money_source_id, \
                    source_document_accounts.account_id, source_documents.document_type, \
                    source_documents.statement_period_from, source_documents.statement_period_to \
             FROM source_documents \
             JOIN source_document_accounts \
               ON source_document_accounts.source_document_id = source_documents.id \
             JOIN money_sources ON money_sources.id = source_documents.money_source_id \
             WHERE source_documents.document_type IS NOT NULL \
               AND source_documents.statement_period_from IS NOT NULL \
               AND source_documents.statement_period_to IS NOT NULL",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut periods = BTreeMap::<(String, String, String, String), Vec<CoveragePeriod>>::new();
        for row in rows {
            let (provider_key, money_source_id, account_id, document_type, from, to) = row?;
            if policies.iter().any(|policy| {
                policy.provider_key == provider_key && policy.document_type == document_type
            }) {
                periods
                    .entry((provider_key, money_source_id, account_id, document_type))
                    .or_default()
                    .push(CoveragePeriod {
                        statement_period_from: from,
                        statement_period_to: to,
                    });
            }
        }
        let mut prompts = Vec::new();
        for ((provider_key, money_source_id, account_id, document_type), periods) in &mut periods {
            let policy = policies
                .iter()
                .find(|policy| {
                    policy.provider_key == provider_key && policy.document_type == document_type
                })
                .expect("policy selected with the same provider and document type");
            periods.sort_by(|left, right| {
                left.statement_period_from
                    .cmp(&right.statement_period_from)
                    .then(left.statement_period_to.cmp(&right.statement_period_to))
            });
            periods.dedup_by(|left, right| {
                left.statement_period_from == right.statement_period_from
                    && left.statement_period_to == right.statement_period_to
            });
            for pair in periods.windows(2) {
                let previous = &pair[0];
                let next = &pair[1];
                let previous_period_ends_month = is_month_end_iso(&previous.statement_period_to)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                    })?;
                let mut expected_from = add_months_iso(
                    &previous.statement_period_from,
                    policy.cadence_months,
                    false,
                )
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                })?;
                let mut expected_to = add_months_iso(
                    &previous.statement_period_to,
                    policy.cadence_months,
                    previous_period_ends_month,
                )
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                })?;
                while expected_from < next.statement_period_from {
                    if !self.statement_coverage_is_suppressed(
                        money_source_id,
                        account_id,
                        document_type,
                        &expected_from,
                        &expected_to,
                        today,
                    )? {
                        prompts.push(StatementCoveragePrompt {
                            account_id: account_id.clone(),
                            document_type: document_type.clone(),
                            money_source_id: money_source_id.clone(),
                            statement_period_from: expected_from.clone(),
                            statement_period_to: expected_to.clone(),
                            status: StatementCoveragePromptStatus::ConfirmedMissing,
                        });
                    }
                    expected_from = add_months_iso(&expected_from, policy.cadence_months, false)
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                        })?;
                    expected_to = add_months_iso(
                        &expected_to,
                        policy.cadence_months,
                        previous_period_ends_month,
                    )
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                    })?;
                }
            }
            if periods.len() >= 2 {
                let latest = periods.last().expect("length checked");
                let latest_period_ends_month = is_month_end_iso(&latest.statement_period_to)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                    })?;
                let expected_from =
                    add_months_iso(&latest.statement_period_from, policy.cadence_months, false)
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                        })?;
                let expected_to = add_months_iso(
                    &latest.statement_period_to,
                    policy.cadence_months,
                    latest_period_ends_month,
                )
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                })?;
                let grace_end = self.add_days_iso(&expected_to, policy.grace_days)?;
                if today > grace_end.as_str()
                    && !self.statement_coverage_is_suppressed(
                        money_source_id,
                        account_id,
                        document_type,
                        &expected_from,
                        &expected_to,
                        today,
                    )?
                {
                    prompts.push(StatementCoveragePrompt {
                        account_id: account_id.clone(),
                        document_type: document_type.clone(),
                        money_source_id: money_source_id.clone(),
                        statement_period_from: expected_from,
                        statement_period_to: expected_to,
                        status: StatementCoveragePromptStatus::LikelyMissing,
                    });
                }
            }
        }
        Ok(prompts)
    }

    pub(crate) fn record_statement_coverage_decision(
        &mut self,
        input: &StatementCoverageDecisionInput<'_>,
    ) -> StoreResult<()> {
        if input.audit_id.is_empty()
            || input.money_source_id.is_empty()
            || input.account_id.is_empty()
            || input.document_type.is_empty()
            || !valid_iso_date(input.statement_period_from)
            || !valid_iso_date(input.statement_period_to)
            || input.statement_period_from > input.statement_period_to
            || !valid_optional_date(input.remind_after)
            || matches!(input.decision, StatementCoverageDecision::NotExpected)
                && input.remind_after.is_some()
            || matches!(input.decision, StatementCoverageDecision::RemindLater)
                && input.remind_after.is_none()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid statement coverage decision",
            )
            .into());
        }
        let transaction = self.connection.transaction()?;
        let account_matches_source: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = ?1 AND money_source_id = ?2)",
            params![input.account_id, input.money_source_id],
            |row| row.get(0),
        )?;
        if !account_matches_source {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "statement coverage account does not belong to source",
            )
            .into());
        }
        let (decision, remind_after) = match input.decision {
            StatementCoverageDecision::NotExpected => ("not_expected", None),
            StatementCoverageDecision::RemindLater => {
                let remind_after = input.remind_after.expect("validated remind date");
                let future: bool = transaction.query_row(
                    "SELECT date(?1) > date('now')",
                    [remind_after],
                    |row| row.get(0),
                )?;
                if !future {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "statement coverage reminder must be in the future",
                    )
                    .into());
                }
                ("remind_later", Some(remind_after))
            }
        };
        let existing = transaction
            .query_row(
                "SELECT decision, remind_after FROM statement_coverage_decisions \
                 WHERE money_source_id = ?1 AND account_id = ?2 AND document_type = ?3 \
                   AND statement_period_from = ?4 AND statement_period_to = ?5",
                params![
                    input.money_source_id,
                    input.account_id,
                    input.document_type,
                    input.statement_period_from,
                    input.statement_period_to,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        if existing
            .as_ref()
            .is_some_and(|(existing_decision, existing_remind_after)| {
                existing_decision == decision && existing_remind_after.as_deref() == remind_after
            })
        {
            transaction.commit()?;
            return Ok(());
        }
        transaction.execute(
            "INSERT INTO statement_coverage_decisions( \
               money_source_id, account_id, document_type, statement_period_from, \
               statement_period_to, decision, remind_after \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(money_source_id, account_id, document_type, statement_period_from, statement_period_to) \
             DO UPDATE SET decision = excluded.decision, remind_after = excluded.remind_after, \
                           updated_at = CURRENT_TIMESTAMP",
            params![
                input.money_source_id,
                input.account_id,
                input.document_type,
                input.statement_period_from,
                input.statement_period_to,
                decision,
                remind_after,
            ],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES (?1, 'statement_coverage', ?2, 'statement_coverage_decided', \
                       'user', ?3, 'coverage-v1')",
            params![
                input.audit_id,
                format!(
                    "{}:{}:{}:{}:{}",
                    input.money_source_id,
                    input.account_id,
                    input.document_type,
                    input.statement_period_from,
                    input.statement_period_to,
                ),
                decision,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn statement_coverage_is_suppressed(
        &self,
        money_source_id: &str,
        account_id: &str,
        document_type: &str,
        statement_period_from: &str,
        statement_period_to: &str,
        today: &str,
    ) -> StoreResult<bool> {
        let decision = self
            .connection
            .query_row(
                "SELECT decision, remind_after FROM statement_coverage_decisions \
                 WHERE money_source_id = ?1 AND account_id = ?2 AND document_type = ?3 \
                   AND statement_period_from = ?4 AND statement_period_to = ?5",
                params![
                    money_source_id,
                    account_id,
                    document_type,
                    statement_period_from,
                    statement_period_to,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        Ok(match decision {
            Some((decision, _)) if decision == "not_expected" => true,
            Some((decision, Some(remind_after))) if decision == "remind_later" => {
                remind_after.as_str() > today
            }
            _ => false,
        })
    }

    fn add_days_iso(&self, date: &str, days: i64) -> StoreResult<String> {
        let modifier = format!("+{days} days");
        Ok(self
            .connection
            .query_row("SELECT date(?1, ?2)", params![date, modifier], |row| {
                row.get(0)
            })?)
    }
}
