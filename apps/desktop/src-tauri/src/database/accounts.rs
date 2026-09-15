use super::{ACCOUNT_CONFIRMATION_POLICY_VERSION, ManualImportStore, StoreResult};
use rusqlite::{Connection, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct AccountConfirmationCandidate {
    pub(crate) account_id: String,
    pub(crate) account_type: String,
    pub(crate) currency: Option<String>,
    pub(crate) display_name: String,
    pub(crate) masked_identifier: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct AccountConfirmationPrompt {
    pub(crate) candidate_accounts: Vec<AccountConfirmationCandidate>,
    pub(crate) dismissed_accounts: Vec<AccountConfirmationCandidate>,
    pub(crate) display_name: String,
    pub(crate) money_source_id: String,
    pub(crate) proposal_version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateProposalVersionFields {
    account_id: String,
    account_type: String,
    currency: Option<String>,
    display_name: String,
    masked_identifier: Option<String>,
    provider_account_id: Option<String>,
    provider_key: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum CandidateAccountDecision {
    Accept,
    Dismiss,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct CandidateAccountDecisionInput {
    pub(crate) account_id: String,
    pub(crate) action: CandidateAccountDecision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum AccountConfirmationStatus {
    AlreadyConfirmed,
    Confirmed,
    Conflict,
    Restored,
    Updated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct AccountConfirmationOutcome {
    pub(crate) status: AccountConfirmationStatus,
}

impl ManualImportStore {
    pub(crate) fn list_account_confirmation_prompts(
        &self,
    ) -> StoreResult<Vec<AccountConfirmationPrompt>> {
        let mut statement = self.connection.prepare(
            "SELECT money_sources.id, money_sources.display_name, accounts.id, \
                    accounts.display_name, accounts.account_type, accounts.masked_identifier, \
                    accounts.currency, accounts.status \
             FROM accounts \
             JOIN money_sources ON money_sources.id = accounts.money_source_id \
             WHERE accounts.status IN ('candidate', 'dismissed') \
             ORDER BY money_sources.display_name, money_sources.id, accounts.status, \
                      accounts.display_name, accounts.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                AccountConfirmationCandidate {
                    account_id: row.get(2)?,
                    display_name: row.get(3)?,
                    account_type: row.get(4)?,
                    masked_identifier: row.get(5)?,
                    currency: row.get(6)?,
                },
                row.get::<_, String>(7)?,
            ))
        })?;
        let mut prompts: Vec<AccountConfirmationPrompt> = Vec::new();
        for row in rows {
            let (money_source_id, display_name, candidate, status) = row?;
            match prompts.last_mut() {
                Some(prompt) if prompt.money_source_id == money_source_id => {
                    if status == "candidate" {
                        prompt.candidate_accounts.push(candidate);
                    } else {
                        prompt.dismissed_accounts.push(candidate);
                    }
                }
                _ => {
                    let (candidate_accounts, dismissed_accounts) = if status == "candidate" {
                        (vec![candidate], Vec::new())
                    } else {
                        (Vec::new(), vec![candidate])
                    };
                    prompts.push(AccountConfirmationPrompt {
                        candidate_accounts,
                        dismissed_accounts,
                        display_name,
                        money_source_id,
                        proposal_version: String::new(),
                    });
                }
            }
        }
        for prompt in &mut prompts {
            prompt.proposal_version =
                candidate_proposal_version(&self.connection, &prompt.money_source_id)?;
        }
        Ok(prompts)
    }

    #[cfg(test)]
    pub(crate) fn confirm_candidate_accounts(
        &mut self,
        money_source_id: &str,
        expected_candidate_account_ids: &[String],
        audit_id: &str,
    ) -> StoreResult<AccountConfirmationOutcome> {
        let proposal_version = candidate_proposal_version(&self.connection, money_source_id)?;
        self.decide_candidate_accounts(
            money_source_id,
            &proposal_version,
            &expected_candidate_account_ids
                .iter()
                .cloned()
                .map(|account_id| CandidateAccountDecisionInput {
                    account_id,
                    action: CandidateAccountDecision::Accept,
                })
                .collect::<Vec<_>>(),
            audit_id,
        )
    }

    pub(crate) fn decide_candidate_accounts(
        &mut self,
        money_source_id: &str,
        proposal_version: &str,
        decisions: &[CandidateAccountDecisionInput],
        audit_id: &str,
    ) -> StoreResult<AccountConfirmationOutcome> {
        let expected_ids = decisions
            .iter()
            .map(|decision| decision.account_id.clone())
            .collect::<BTreeSet<_>>();
        if money_source_id.is_empty()
            || proposal_version.is_empty()
            || audit_id.is_empty()
            || expected_ids.is_empty()
            || expected_ids.len() != decisions.len()
            || expected_ids.iter().any(|account_id| account_id.is_empty())
        {
            return Ok(AccountConfirmationOutcome {
                status: AccountConfirmationStatus::Conflict,
            });
        }

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current_proposal_version = candidate_proposal_version(&transaction, money_source_id)?;
        let account_statuses = {
            let mut statement = transaction.prepare(
                "SELECT id, status FROM accounts WHERE money_source_id = ?1 ORDER BY id",
            )?;
            statement
                .query_map([money_source_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<BTreeMap<_, _>, _>>()?
        };
        let candidate_ids = account_statuses
            .iter()
            .filter_map(|(account_id, status)| {
                (status == "candidate").then_some(account_id.clone())
            })
            .collect::<BTreeSet<_>>();
        if candidate_ids == expected_ids {
            if current_proposal_version != proposal_version {
                return Ok(AccountConfirmationOutcome {
                    status: AccountConfirmationStatus::Conflict,
                });
            }
            for decision in decisions {
                let (status, action, reason) = match decision.action {
                    CandidateAccountDecision::Accept => (
                        "confirmed",
                        "candidate_account_accepted",
                        "user_accepted_candidate",
                    ),
                    CandidateAccountDecision::Dismiss => (
                        "dismissed",
                        "candidate_account_dismissed",
                        "user_dismissed_candidate",
                    ),
                };
                transaction.execute(
                    "UPDATE accounts SET status = ?1, updated_at = CURRENT_TIMESTAMP \
                     WHERE id = ?2 AND money_source_id = ?3 AND status = 'candidate'",
                    params![status, decision.account_id, money_source_id],
                )?;
                if decision.action == CandidateAccountDecision::Dismiss {
                    transaction.execute(
                        "UPDATE external_records SET status = 'removed' \
                         WHERE account_id = ?1 AND status IN ('staged', 'review')",
                        [&decision.account_id],
                    )?;
                    transaction.execute(
                        "UPDATE review_items SET status = 'resolved' WHERE status = 'open' \
                         AND external_record_id IN ( \
                           SELECT id FROM external_records WHERE account_id = ?1 AND status = 'removed' \
                         )",
                        [&decision.account_id],
                    )?;
                }
                transaction.execute(
                    "INSERT INTO audit_log( \
                       id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
                     ) VALUES (?1, 'account', ?2, ?3, 'user', ?4, ?5, ?6)",
                    params![
                        format!("{audit_id}:{}", decision.account_id),
                        decision.account_id,
                        action,
                        reason,
                        format!("{money_source_id}:proposal:{proposal_version}"),
                        ACCOUNT_CONFIRMATION_POLICY_VERSION,
                    ],
                )?;
            }
            transaction.commit()?;
            return Ok(AccountConfirmationOutcome {
                status: if decisions
                    .iter()
                    .all(|decision| decision.action == CandidateAccountDecision::Accept)
                {
                    AccountConfirmationStatus::Confirmed
                } else {
                    AccountConfirmationStatus::Updated
                },
            });
        }
        if candidate_ids.is_empty()
            && expected_ids.iter().all(|account_id| {
                account_statuses
                    .get(account_id)
                    .is_some_and(|status| status == "confirmed")
            })
        {
            return Ok(AccountConfirmationOutcome {
                status: AccountConfirmationStatus::AlreadyConfirmed,
            });
        }
        Ok(AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Conflict,
        })
    }

    pub(crate) fn restore_dismissed_candidate_account(
        &mut self,
        account_id: &str,
        audit_id: &str,
    ) -> StoreResult<AccountConfirmationOutcome> {
        if account_id.is_empty() || audit_id.is_empty() {
            return Ok(AccountConfirmationOutcome {
                status: AccountConfirmationStatus::Conflict,
            });
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE accounts SET status = 'candidate', updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?1 AND status = 'dismissed'",
            [account_id],
        )?;
        if changed != 1 {
            return Ok(AccountConfirmationOutcome {
                status: AccountConfirmationStatus::Conflict,
            });
        }
        transaction.execute(
            "UPDATE external_records SET status = 'review' \
             WHERE account_id = ?1 AND status = 'removed' AND NOT EXISTS ( \
               SELECT 1 FROM external_records newer \
               WHERE newer.stable_record_key = external_records.stable_record_key \
                 AND newer.version > external_records.version \
             )",
            [account_id],
        )?;
        transaction.execute(
            "INSERT INTO review_items(id, external_record_id, reason_code, status) \
             SELECT ?1 || ':' || external_records.id, external_records.id, 'account_restored', 'open' \
             FROM external_records WHERE account_id = ?2 AND status = 'review' \
               AND NOT EXISTS (SELECT 1 FROM review_items WHERE external_record_id = external_records.id \
                 AND reason_code = 'account_restored' AND status = 'open')",
            params![audit_id, account_id],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES (?1, 'account', ?2, 'candidate_account_restored', 'user', \
                       'user_restored_candidate', ?3)",
            params![audit_id, account_id, ACCOUNT_CONFIRMATION_POLICY_VERSION],
        )?;
        transaction.commit()?;
        Ok(AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Restored,
        })
    }

    pub(crate) fn money_source_exists(&self, money_source_id: &str) -> StoreResult<bool> {
        let exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM money_sources WHERE id = ?1)",
            [money_source_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }
}

fn candidate_proposal_version(
    connection: &Connection,
    money_source_id: &str,
) -> StoreResult<String> {
    let mut statement = connection.prepare(
        "SELECT id, account_type, currency, display_name, masked_identifier, \
                provider_account_id, provider_key \
         FROM accounts \
         WHERE money_source_id = ?1 AND status = 'candidate' \
         ORDER BY id",
    )?;
    let candidates = statement
        .query_map([money_source_id], |row| {
            Ok(CandidateProposalVersionFields {
                account_id: row.get(0)?,
                account_type: row.get(1)?,
                currency: row.get(2)?,
                display_name: row.get(3)?,
                masked_identifier: row.get(4)?,
                provider_account_id: row.get(5)?,
                provider_key: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let canonical = serde_json::to_vec(&candidates)?;
    Ok(format!("{:x}", Sha256::digest(canonical)))
}
