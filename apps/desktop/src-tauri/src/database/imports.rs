use super::*;

pub(crate) struct SourceCapture {
    pub(super) files: FileVault,
    pub(super) master_key: Zeroizing<[u8; KEY_LEN]>,
    pub(super) replace_existing: bool,
}

impl SourceCapture {
    pub(crate) fn store_prepared(&self, source: &PreparedSource) -> StoreResult<StoredFile> {
        Ok(self
            .files
            .store_prepared(&self.master_key, source, self.replace_existing)?)
    }
}

pub(crate) enum SourceCapturePlan {
    Capture(SourceCapture),
    RestoreConfirmationRequired(SourceDocumentImportOutcome),
}

pub(crate) struct SourceDocumentReadPlan {
    pub(super) encrypted_locator: String,
    pub(super) file_sha256: String,
    pub(super) files: FileVault,
    pub(super) master_key: Zeroizing<[u8; KEY_LEN]>,
    pub(super) mime_type: String,
}

impl SourceDocumentReadPlan {
    pub(crate) fn read(self) -> StoreResult<SourceDocumentFileInput> {
        let plaintext = self
            .files
            .open_in_memory(&self.master_key, &self.encrypted_locator)?;
        Ok(SourceDocumentFileInput {
            file_sha256: self.file_sha256,
            mime_type: self.mime_type,
            plaintext,
        })
    }
}

pub(super) fn find_exact_document(
    connection: &Connection,
    file_sha256: &str,
) -> rusqlite::Result<Option<ExistingDocument>> {
    connection
        .query_row(
            "SELECT id, encrypted_locator, file_sha256, file_state \
             FROM source_documents WHERE file_sha256 = ?1",
            [file_sha256],
            |row| {
                Ok(ExistingDocument {
                    document_id: row.get(0)?,
                    encrypted_locator: row.get(1)?,
                    file_sha256: row.get(2)?,
                    file_state: row.get(3)?,
                })
            },
        )
        .optional()
}

pub(super) fn find_document_by_id(
    connection: &Connection,
    document_id: &str,
) -> rusqlite::Result<Option<ExistingDocument>> {
    connection
        .query_row(
            "SELECT id, encrypted_locator, file_sha256, file_state \
             FROM source_documents WHERE id = ?1",
            [document_id],
            |row| {
                Ok(ExistingDocument {
                    document_id: row.get(0)?,
                    encrypted_locator: row.get(1)?,
                    file_sha256: row.get(2)?,
                    file_state: row.get(3)?,
                })
            },
        )
        .optional()
}

pub(super) fn persist_source_deletion(
    connection: &mut Connection,
    document: &ExistingDocument,
    audit_id: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, 'source_file_deletion_decided', \
                   'user', 'user_requested', ?3, 'source-file-deletion-v1')",
        params![audit_id, document.document_id, document.file_sha256],
    )?;
    let changed = transaction.execute(
        "UPDATE source_documents \
         SET encrypted_locator = NULL, file_state = 'deleted', \
             deleted_at = CURRENT_TIMESTAMP, deletion_audit_id = ?1 \
         WHERE id = ?2 AND file_state = 'available'",
        params![audit_id, document.document_id],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    transaction.execute(
        "INSERT INTO review_items(id, external_record_id, reason_code, status) \
         SELECT ?1 || ':' || external_records.id, external_records.id, \
                'source_file_deleted', 'open' \
         FROM external_records \
         WHERE source_document_id = ?2 AND status IN ('staged', 'review') \
           AND NOT EXISTS ( \
             SELECT 1 FROM review_items \
             WHERE external_record_id = external_records.id \
               AND reason_code = 'source_file_deleted' AND status = 'open' \
           )",
        params![audit_id, document.document_id],
    )?;
    transaction.execute(
        "UPDATE external_records SET status = 'review' \
         WHERE source_document_id = ?1 AND status = 'staged'",
        [&document.document_id],
    )?;
    transaction.commit()
}

pub(super) fn persist_import(
    connection: &mut Connection,
    input: &SourceDocumentImport<'_>,
    stored: &StoredFile,
    restore_deleted_document_id: Option<&str>,
    intake_item_id: Option<&str>,
) -> rusqlite::Result<SourceDocumentImportOutcome> {
    let transaction = connection.transaction()?;
    let existing = find_exact_document(&transaction, &stored.file_sha256)?;
    match (existing.as_ref(), restore_deleted_document_id) {
        (Some(existing), None) if existing.file_state == "deleted" => {
            let outcome = SourceDocumentImportOutcome {
                document_id: existing.document_id.clone(),
                status: SourceDocumentImportStatus::RestoreConfirmationRequired,
                intake_item_id: None,
            };
            if let Some(item_id) = intake_item_id {
                super::intake::finalize_document_intake_item(
                    &transaction,
                    item_id,
                    &outcome.document_id,
                    outcome.status,
                )?;
                transaction.commit()?;
            }
            return Ok(outcome);
        }
        (Some(existing), Some(expected_document_id))
            if existing.file_state == "deleted" && existing.document_id == expected_document_id => {
        }
        (_, Some(_)) => return Err(rusqlite::Error::InvalidQuery),
        _ => {}
    }
    let (document_id, status) = if let Some(existing) = existing {
        let status = if existing.file_state == "available" {
            SourceDocumentImportStatus::AlreadyPresent
        } else {
            transaction.execute(
                "UPDATE source_documents \
                 SET encrypted_locator = ?1, file_state = 'available', \
                     deleted_at = NULL, deletion_audit_id = NULL \
                 WHERE id = ?2",
                params![stored.encrypted_locator, existing.document_id],
            )?;
            SourceDocumentImportStatus::Restored
        };
        (existing.document_id, status)
    } else {
        let byte_size = i64::try_from(stored.byte_size)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        transaction.execute(
            "INSERT INTO source_documents( \
               id, file_sha256, original_filename, mime_type, byte_size, \
               encrypted_locator, file_state \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'available')",
            params![
                input.document_id,
                stored.file_sha256,
                input.original_filename,
                input.mime_type,
                byte_size,
                stored.encrypted_locator,
            ],
        )?;
        (
            input.document_id.to_owned(),
            SourceDocumentImportStatus::Imported,
        )
    };
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            input.audit_id,
            document_id,
            import_audit_action(status),
            input.audit_actor,
            input.audit_reason,
            stored.file_sha256,
            input.audit_policy_version,
        ],
    )?;
    if matches!(
        status,
        SourceDocumentImportStatus::Imported | SourceDocumentImportStatus::Restored
    ) {
        enqueue_parse_document(&transaction, &document_id, &new_database_id("parse-run"))?;
    }
    if let Some(item_id) = intake_item_id {
        super::intake::finalize_document_intake_item(&transaction, item_id, &document_id, status)?;
    }
    transaction.commit()?;
    Ok(SourceDocumentImportOutcome {
        document_id,
        status,
        intake_item_id: None,
    })
}

pub(super) fn mark_missing(
    connection: &mut Connection,
    document: &ExistingDocument,
    file_sha256: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE source_documents \
         SET file_state = 'missing', deleted_at = NULL, deletion_audit_id = NULL \
         WHERE id = ?1",
        [&document.document_id],
    )?;
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, 'source_file_missing', \
                   'system', 'storage_verification_failed', ?3, 'vault-storage-v1')",
        params![new_audit_id(), document.document_id, file_sha256],
    )?;
    transaction.commit()
}

pub(super) fn import_audit_action(status: SourceDocumentImportStatus) -> &'static str {
    match status {
        SourceDocumentImportStatus::Imported => "manual_import_imported",
        SourceDocumentImportStatus::AlreadyPresent => "manual_import_already_present",
        SourceDocumentImportStatus::Restored => "manual_import_restored",
        SourceDocumentImportStatus::RestoreConfirmationRequired => {
            "manual_import_restore_confirmation_required"
        }
    }
}

pub(super) fn new_audit_id() -> String {
    let mut random = [0_u8; 16];
    OsRng.fill_bytes(&mut random);
    format!("audit-{}", hex_encode(&random))
}

pub(super) fn validate_import(input: &SourceDocumentImport<'_>) -> StoreResult<()> {
    for (name, value) in [
        ("audit_actor", input.audit_actor),
        ("audit_id", input.audit_id),
        ("audit_policy_version", input.audit_policy_version),
        ("audit_reason", input.audit_reason),
        ("document_id", input.document_id),
        ("mime_type", input.mime_type),
        ("original_filename", input.original_filename),
    ] {
        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must not be empty"),
            )
            .into());
        }
    }
    Ok(())
}

pub(super) fn validate_captured_container(mime_type: &str, plaintext: &[u8]) -> StoreResult<()> {
    match mime_type {
        "application/pdf" if !plaintext.starts_with(b"%PDF-") => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "captured PDF does not have a PDF header",
        )
        .into()),
        "text/csv" => {
            std::str::from_utf8(plaintext).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "captured CSV is not UTF-8")
            })?;
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .flexible(true)
                .from_reader(plaintext);
            for record in reader.records() {
                record.map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "captured CSV is malformed")
                })?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub(super) fn ensure_fiat_currency_instruments(
    transaction: &Transaction<'_>,
    accounts: &[TrustedAccountCandidate<'_>],
) -> StoreResult<()> {
    let currencies = accounts
        .iter()
        .filter_map(|account| account.currency)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for currency in currencies {
        let instrument_id = fiat_currency_instrument_id(&currency);
        let existing = transaction
            .query_row(
                "SELECT instrument_type, symbol, currency FROM instruments WHERE id = ?1",
                [&instrument_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;
        if let Some((instrument_type, symbol, stored_currency)) = existing
            && (instrument_type != "fiat_currency"
                || symbol != currency
                || stored_currency.as_deref() != Some(currency.as_str()))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fiat currency instrument conflicts with its deterministic identity",
            )
            .into());
        }
        let fiat_currency_instrument_count: i64 = transaction.query_row(
            "SELECT count(*) FROM instruments \
             WHERE instrument_type = 'fiat_currency' AND currency = ?1",
            params![currency],
            |row| row.get(0),
        )?;
        match fiat_currency_instrument_count {
            0 => {
                transaction.execute(
                    "INSERT INTO instruments(id, instrument_type, symbol, currency, display_name) \
                     VALUES (?1, 'fiat_currency', ?2, ?3, ?4)",
                    params![instrument_id, currency, currency, currency],
                )?;
            }
            1 => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "multiple fiat currency instruments exist",
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(super) fn fiat_currency_instrument_id(currency: &str) -> String {
    format!("instrument-fiat-{currency}")
}

pub(super) fn validate_classification(
    input: &TrustedDocumentClassification<'_>,
) -> StoreResult<()> {
    for (name, value) in [
        ("audit_id", input.audit_id),
        ("document_id", input.document_id),
        ("provider_key", input.provider_key),
        ("semantic_document_key", input.semantic_document_key),
    ] {
        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must not be empty"),
            )
            .into());
        }
    }
    if input.accounts.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "classification must contain at least one account",
        )
        .into());
    }
    if !valid_optional_date(input.statement_period_from)
        || !valid_optional_date(input.statement_period_to)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "statement period must contain valid ISO dates",
        )
        .into());
    }
    for account in input.accounts {
        for (name, value) in [
            ("account_id", account.account_id),
            ("account_type", account.account_type),
            ("display_name", account.display_name),
        ] {
            if value.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{name} must not be empty"),
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}

pub(super) fn hex_encode_secret(bytes: &[u8]) -> Zeroizing<String> {
    let mut hex = Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut *hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}
