use super::{ManualImportStore, MoneySourceView, StoreResult, intake::validate_identifier};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::io;

/// The audit policy version recorded for a user-authored Money Source change.
const MONEY_SOURCE_POLICY_VERSION: &str = "money-source-v1";
const MAX_DISPLAY_NAME_BYTES: usize = 256;
const MAX_PROVIDER_KEY_BYTES: usize = 128;
const MAX_SOURCE_TYPE_BYTES: usize = 128;

/// A user-authored Money Source creation.
///
/// The provider identity is a host decision — `runtime::sources` matches it
/// against the supported-provider catalog before this input exists — and the
/// row stays a provider singleton: `provider_root_id` is left NULL, so this
/// path never claims a provider root identity.
pub(crate) struct CreateMoneySourceInput<'a> {
    pub(crate) audit_id: &'a str,
    pub(crate) display_name: &'a str,
    pub(crate) money_source_id: &'a str,
    pub(crate) provider_key: &'a str,
    pub(crate) source_type: &'a str,
}

impl ManualImportStore {
    /// Creates the one configured Money Source for a provider.
    ///
    /// Routing resolves a classified document through
    /// `(provider_key, provider_root_id)` and treats two matches as ambiguous,
    /// so a second provider-scoped singleton would make every future
    /// classification of that provider fail closed. The check and the insert
    /// share one immediate transaction, and a duplicate is reported as
    /// `AlreadyExists` instead of writing a row.
    pub(crate) fn create_money_source(
        &mut self,
        input: &CreateMoneySourceInput<'_>,
    ) -> StoreResult<MoneySourceView> {
        validate_money_source_identity(
            input.money_source_id,
            input.provider_key,
            input.source_type,
        )?;
        validate_display_name(input.display_name)?;
        validate_identifier(input.audit_id, "audit id")?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let configured: bool = transaction.query_row(
            "SELECT EXISTS( \
               SELECT 1 FROM money_sources \
               WHERE provider_key = ?1 AND provider_root_id IS NULL \
             )",
            [input.provider_key],
            |row| row.get(0),
        )?;
        if configured {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Money Source provider is already configured",
            )
            .into());
        }
        transaction.execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                input.money_source_id,
                input.provider_key,
                input.display_name,
                input.source_type,
            ],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'money_source', ?2, 'money_source_created', 'user', \
                       'user_created_source', ?3, ?4)",
            params![
                input.audit_id,
                input.money_source_id,
                input.provider_key,
                MONEY_SOURCE_POLICY_VERSION,
            ],
        )?;
        transaction.commit()?;
        Ok(MoneySourceView {
            display_name: input.display_name.to_owned(),
            money_source_id: input.money_source_id.to_owned(),
            source_type: input.source_type.to_owned(),
        })
    }

    /// Renames a Money Source.
    ///
    /// Only `display_name` changes: the identifier, provider identity, and
    /// every evidence/ledger link stay stable, so a rename is one audited
    /// column update and never a new source row.
    pub(crate) fn rename_money_source(
        &mut self,
        money_source_id: &str,
        display_name: &str,
        audit_id: &str,
    ) -> StoreResult<MoneySourceView> {
        validate_identifier(money_source_id, "Money Source id")?;
        validate_identifier(audit_id, "audit id")?;
        validate_display_name(display_name)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let source_type = transaction
            .query_row(
                "SELECT source_type FROM money_sources WHERE id = ?1",
                [money_source_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(source_type) = source_type else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Money Source not found").into());
        };
        transaction.execute(
            "UPDATE money_sources SET display_name = ?1 WHERE id = ?2",
            params![display_name, money_source_id],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES (?1, 'money_source', ?2, 'money_source_renamed', 'user', \
                       'user_renamed_source', ?3)",
            params![audit_id, money_source_id, MONEY_SOURCE_POLICY_VERSION],
        )?;
        transaction.commit()?;
        Ok(MoneySourceView {
            display_name: display_name.to_owned(),
            money_source_id: money_source_id.to_owned(),
            source_type,
        })
    }

    pub(crate) fn money_source(
        &self,
        money_source_id: &str,
    ) -> StoreResult<Option<MoneySourceView>> {
        self.connection
            .query_row(
                "SELECT id, display_name, source_type FROM money_sources WHERE id = ?1",
                [money_source_id],
                |row| {
                    Ok(MoneySourceView {
                        money_source_id: row.get(0)?,
                        display_name: row.get(1)?,
                        source_type: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Whether this source has a statement password stored as `saved`.
    ///
    /// The same rule `statement_password_sources` applies to the whole list,
    /// scoped to one source so the source-detail projection does not need the
    /// Keychain reference itself.
    pub(crate) fn has_saved_statement_password(&self, money_source_id: &str) -> StoreResult<bool> {
        self.connection
            .query_row(
                "SELECT EXISTS( \
                   SELECT 1 FROM statement_secret_refs \
                   WHERE money_source_id = ?1 AND status = 'saved' \
                 )",
                [money_source_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }
}

fn validate_money_source_identity(
    money_source_id: &str,
    provider_key: &str,
    source_type: &str,
) -> StoreResult<()> {
    validate_identifier(money_source_id, "Money Source id")?;
    if provider_key.is_empty() || provider_key.len() > MAX_PROVIDER_KEY_BYTES {
        return Err(invalid_input("invalid Money Source provider key"));
    }
    if source_type.is_empty() || source_type.len() > MAX_SOURCE_TYPE_BYTES {
        return Err(invalid_input("invalid Money Source type"));
    }
    Ok(())
}

fn validate_display_name(display_name: &str) -> StoreResult<()> {
    if display_name.is_empty() || display_name.len() > MAX_DISPLAY_NAME_BYTES {
        return Err(invalid_input("invalid Money Source display name"));
    }
    Ok(())
}

fn invalid_input(message: &str) -> Box<dyn std::error::Error + Send + Sync> {
    io::Error::new(io::ErrorKind::InvalidInput, message.to_owned()).into()
}
