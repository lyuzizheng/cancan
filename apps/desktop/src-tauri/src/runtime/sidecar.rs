use super::*;

pub(super) fn source_document_metadata(
    source_path: &Path,
) -> Result<(String, &'static str), RuntimeError> {
    let metadata = fs::metadata(source_path).map_err(|_| RuntimeError::new("import_failed"))?;
    if !metadata.is_file() {
        return Err(RuntimeError::new("unsupported_document"));
    }
    source_document_filename_metadata(source_path)
}

pub(super) fn source_document_filename_metadata(
    source_path: &Path,
) -> Result<(String, &'static str), RuntimeError> {
    let mime_type = match source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("pdf") => "application/pdf",
        Some("csv") => "text/csv",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => return Err(RuntimeError::new("unsupported_document")),
    };
    let original_filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| RuntimeError::new("unsupported_document"))?
        .to_owned();
    Ok((original_filename, mime_type))
}

pub(super) async fn run_normalizer_sidecar(
    app: &AppHandle,
    document_id: &str,
    extraction_bundle: &ExtractionBundle,
) -> Result<NormalizerResult, RuntimeError> {
    let request_id = random_identifier("normalize");
    let command = Zeroizing::new(
        serde_json::to_vec(&NormalizerCommand {
            document_id,
            extraction_bundle,
            request_id: &request_id,
            kind: "normalize",
        })
        .map_err(|_| RuntimeError::new("normalizer_unavailable"))?,
    );
    let sidecar = app
        .shell()
        .sidecar("cancan-document-normalizer")
        .map_err(|_| RuntimeError::new("normalizer_unavailable"))?
        .env_clear();
    let (mut events, mut child) = sidecar
        .spawn()
        .map_err(|_| RuntimeError::new("normalizer_unavailable"))?;
    let deadline = Instant::now() + NORMALIZER_TIMEOUT;
    let mut ready = false;
    let result = loop {
        let event = match timeout_at(deadline, events.recv()).await {
            Ok(Some(event)) => event,
            Ok(None) | Err(_) => return fail_normalizer(child),
        };
        match event {
            CommandEvent::Stdout(bytes) => {
                if bytes.len() > NORMALIZER_MAX_MESSAGE_BYTES {
                    return fail_normalizer(child);
                }
                let message = match serde_json::from_slice::<NormalizerMessage>(&bytes) {
                    Ok(message) => message,
                    Err(_) => return fail_normalizer(child),
                };
                match message {
                    NormalizerMessage::Ready {
                        protocol_version,
                        runtime,
                        environment_cleared,
                    } if !ready
                        && valid_normalizer_ready(
                            protocol_version,
                            &runtime,
                            environment_cleared,
                        ) =>
                    {
                        let mut framed = Zeroizing::new(command.to_vec());
                        framed.push(b'\n');
                        if child.write(&framed).is_err() {
                            return fail_normalizer(child);
                        }
                        ready = true;
                    }
                    NormalizerMessage::Result {
                        request_id: response_id,
                        result,
                    } if ready && response_id == request_id => break result,
                    NormalizerMessage::Error { code } => {
                        let _ = code;
                        return fail_normalizer(child);
                    }
                    NormalizerMessage::Ready { .. } | NormalizerMessage::Result { .. } => {
                        return fail_normalizer(child);
                    }
                }
            }
            CommandEvent::Terminated(_) => {
                return Err(RuntimeError::new("normalizer_failed"));
            }
            CommandEvent::Stderr(_) | CommandEvent::Error(_) => return fail_normalizer(child),
            _ => return fail_normalizer(child),
        }
    };

    if child.write(b"{\"type\":\"shutdown\"}\n").is_err() {
        return fail_normalizer(child);
    }
    let shutdown_deadline = Instant::now() + NORMALIZER_SHUTDOWN_TIMEOUT;
    let event = match timeout_at(shutdown_deadline, events.recv()).await {
        Ok(Some(event)) => event,
        Ok(None) | Err(_) => return fail_normalizer(child),
    };
    match event {
        CommandEvent::Terminated(payload) if payload.code == Some(0) => Ok(result),
        CommandEvent::Terminated(_) => Err(RuntimeError::new("normalizer_failed")),
        CommandEvent::Stdout(_) | CommandEvent::Stderr(_) | CommandEvent::Error(_) => {
            fail_normalizer(child)
        }
        _ => fail_normalizer(child),
    }
}

pub(super) fn fail_normalizer(child: CommandChild) -> Result<NormalizerResult, RuntimeError> {
    let _ = child.kill();
    Err(RuntimeError::new("normalizer_failed"))
}

pub(super) async fn run_review_core_sidecar<T: Serialize>(
    app: &AppHandle,
    operation: &'static str,
    input: &T,
) -> Result<ReviewCoreResult, RuntimeError> {
    let request_id = random_identifier("review-core");
    let command = Zeroizing::new(
        serde_json::to_vec(&ReviewCoreCommand {
            input,
            operation,
            request_id: &request_id,
            kind: "core",
        })
        .map_err(|_| RuntimeError::new("review_core_failed"))?,
    );
    let sidecar = app
        .shell()
        .sidecar("cancan-document-normalizer")
        .map_err(|_| RuntimeError::new("review_core_failed"))?
        .env_clear();
    let (mut events, mut child) = sidecar
        .spawn()
        .map_err(|_| RuntimeError::new("review_core_failed"))?;
    let deadline = Instant::now() + NORMALIZER_TIMEOUT;
    let mut ready = false;
    let result = loop {
        let event = match timeout_at(deadline, events.recv()).await {
            Ok(Some(event)) => event,
            Ok(None) | Err(_) => return fail_review_core(child),
        };
        match event {
            CommandEvent::Stdout(bytes) => {
                let message = match serde_json::from_slice::<ReviewCoreMessage>(&bytes) {
                    Ok(message) => message,
                    Err(_) => return fail_review_core(child),
                };
                match message {
                    ReviewCoreMessage::Ready {
                        protocol_version,
                        runtime,
                        environment_cleared,
                    } if !ready
                        && valid_normalizer_ready(
                            protocol_version,
                            &runtime,
                            environment_cleared,
                        ) =>
                    {
                        let mut framed = Zeroizing::new(command.to_vec());
                        framed.push(b'\n');
                        if child.write(&framed).is_err() {
                            return fail_review_core(child);
                        }
                        ready = true;
                    }
                    ReviewCoreMessage::Result {
                        request_id: response_id,
                        result,
                    } if ready && response_id == request_id => break result,
                    ReviewCoreMessage::Error { code } => {
                        let _ = code;
                        return fail_review_core(child);
                    }
                    ReviewCoreMessage::Ready { .. } | ReviewCoreMessage::Result { .. } => {
                        return fail_review_core(child);
                    }
                }
            }
            CommandEvent::Terminated(_) => return Err(RuntimeError::new("review_core_failed")),
            CommandEvent::Stderr(_) | CommandEvent::Error(_) => return fail_review_core(child),
            _ => return fail_review_core(child),
        }
    };

    if child.write(b"{\"type\":\"shutdown\"}\n").is_err() {
        return fail_review_core(child);
    }
    let shutdown_deadline = Instant::now() + NORMALIZER_SHUTDOWN_TIMEOUT;
    let event = match timeout_at(shutdown_deadline, events.recv()).await {
        Ok(Some(event)) => event,
        Ok(None) | Err(_) => return fail_review_core(child),
    };
    match event {
        CommandEvent::Terminated(payload) if payload.code == Some(0) => Ok(result),
        CommandEvent::Terminated(_) => Err(RuntimeError::new("review_core_failed")),
        CommandEvent::Stdout(_) | CommandEvent::Stderr(_) | CommandEvent::Error(_) => {
            fail_review_core(child)
        }
        _ => fail_review_core(child),
    }
}

pub(super) fn fail_review_core(child: CommandChild) -> Result<ReviewCoreResult, RuntimeError> {
    let _ = child.kill();
    Err(RuntimeError::new("review_core_failed"))
}

pub(super) fn is_unique_requested_relationship_candidate(
    candidates: &[CoreCandidate],
    candidate_record_id: &str,
) -> bool {
    matches!(
        candidates,
        [candidate] if candidate.id.as_str() == candidate_record_id
    )
}

pub(super) fn valid_normalizer_ready(
    protocol_version: u8,
    runtime: &str,
    environment_cleared: bool,
) -> bool {
    protocol_version == 1 && runtime == "single-pass-mock" && environment_cleared
}

pub(super) fn valid_normalization_profile(
    profile: &NormalizerProfile,
    proposal: &NormalizerProposal,
    extraction_bundle: &ExtractionBundle,
) -> bool {
    profile.normalizer_runtime == "single-pass-mock"
        && profile.input_strategy == "native-observations-v1"
        && profile.model_provider == "cancan-deterministic-mock"
        && profile.model == "fixture-v1"
        && profile.review_only
        && profile.id.len() <= 256
        && profile.provider_key == proposal.document.provider_key
        && profile.document_type == proposal.document.document_type
        && valid_profile_engines(profile, extraction_bundle)
        && match profile.package_id.as_str() {
            "synthetic/bank_transfer_export@1" => {
                profile.id == "synthetic-bank-transfer-export-v1"
                    && matches!(
                        extraction_bundle.mime_type.as_str(),
                        "text/csv" | "application/pdf"
                    )
                    && profile.provider_key == "synthetic-bank"
                    && profile.document_type == "transfer_export"
                    && profile.package_version == "1.0.0"
                    && profile.parser_version == "synthetic-bank-v1"
                    && profile.skill_version == "synthetic-bank-v1"
                    && profile.prompt_version == "synthetic-bank-v1"
                    && profile.schema_version == "structured-proposal-v1"
                    && profile.tool_contract_version == "synthetic-bank-v1"
                    && profile.validator_version == "synthetic-bank-v1"
                    && valid_synthetic_fingerprint(extraction_bundle, proposal)
            }
            "dbs/bank_statement@1" => {
                valid_provider_package_profile(profile, extraction_bundle, "dbs", "bank_statement")
            }
            "dbs/credit_card_statement@1" => valid_provider_package_profile(
                profile,
                extraction_bundle,
                "dbs",
                "credit_card_statement",
            ),
            "hsbc/bank_statement@1" => {
                valid_provider_package_profile(profile, extraction_bundle, "hsbc", "bank_statement")
            }
            _ => false,
        }
}

pub(super) fn valid_synthetic_fingerprint(
    extraction_bundle: &ExtractionBundle,
    proposal: &NormalizerProposal,
) -> bool {
    extraction_bundle
        .observations
        .iter()
        .any(|observation| observation.text.contains("CANCAN_SYNTHETIC_STATEMENT_V1"))
        && extraction_bundle
            .observations
            .iter()
            .any(|observation| observation.text.contains("provider=synthetic-bank"))
        && extraction_bundle
            .observations
            .iter()
            .any(|observation| observation.text.contains("statement_id=transfer-2026-07"))
        && proposal.document.provider_key == "synthetic-bank"
        && proposal.document.document_type == "transfer_export"
        && proposal.document.statement_id.as_deref() == Some("transfer-2026-07")
        && proposal.status == "valid"
        && proposal
            .document
            .statement_period
            .as_ref()
            .is_some_and(|period| {
                period.from.as_deref() == Some("2026-07-01")
                    && period.to.as_deref() == Some("2026-07-31")
            })
        && proposal.accounts.len() == 2
        && proposal.accounts.iter().any(|account| {
            account.proposal_account_id == "account-checking"
                && account.account_type == "deposit_account"
                && account.provider_account_id.as_deref() == Some("checking-001")
                && account.masked_identifier.as_deref() == Some("••001")
                && account.currency.as_deref() == Some("SGD")
        })
        && proposal.accounts.iter().any(|account| {
            account.proposal_account_id == "account-savings"
                && account.account_type == "deposit_account"
                && account.provider_account_id.as_deref() == Some("savings-002")
                && account.masked_identifier.as_deref() == Some("••002")
                && account.currency.as_deref() == Some("SGD")
        })
}

pub(super) fn valid_provider_package_profile(
    profile: &NormalizerProfile,
    extraction_bundle: &ExtractionBundle,
    provider_key: &str,
    document_type: &str,
) -> bool {
    extraction_bundle.mime_type == "application/pdf"
        && profile.provider_key == provider_key
        && profile.document_type == document_type
        && profile.package_version == "1.0.0"
        && profile.parser_version == "1.0.0"
        && profile.skill_version == "1.0.0"
        && profile.prompt_version == "1.0.0"
        && profile.schema_version == "1.0.0"
        && profile.tool_contract_version == "1.0.0"
        && profile.validator_version == "1.0.0"
        && profile.id == provider_normalization_profile_id(profile)
}

pub(super) fn valid_profile_engines(
    profile: &NormalizerProfile,
    extraction_bundle: &ExtractionBundle,
) -> bool {
    let profile_extraction = profile
        .extraction_engines
        .iter()
        .map(|engine| {
            canonical_profile_extraction_engine(engine).then(|| {
                profile_extraction_engine_key(&engine.kind, &engine.engine, &engine.version)
            })
        })
        .collect::<Option<Vec<_>>>();
    let profile_ocr = profile
        .ocr_engines
        .iter()
        .map(|engine| {
            canonical_profile_ocr_engine(engine)
                .then(|| format!("{}\0{}", engine.engine, engine.version))
        })
        .collect::<Option<Vec<_>>>();
    let (Some(profile_extraction), Some(profile_ocr)) = (profile_extraction, profile_ocr) else {
        return false;
    };
    let mut profile_extraction_sorted = profile_extraction.clone();
    let mut profile_ocr_sorted = profile_ocr.clone();
    profile_extraction_sorted.sort();
    profile_ocr_sorted.sort();
    if profile_extraction != profile_extraction_sorted || profile_ocr != profile_ocr_sorted {
        return false;
    }

    let mut extraction_engines = HashSet::new();
    let mut ocr_engines = HashSet::new();
    for observation in &extraction_bundle.observations {
        match &observation.kind {
            SourceObservationKind::NativeText => {
                extraction_engines.insert(profile_extraction_engine_key(
                    &NormalizerProfileExtractionKind::NativeText,
                    &observation.engine,
                    &observation.engine_version,
                ));
            }
            SourceObservationKind::TableCell => {
                extraction_engines.insert(profile_extraction_engine_key(
                    &NormalizerProfileExtractionKind::TableCell,
                    &observation.engine,
                    &observation.engine_version,
                ));
            }
            SourceObservationKind::OcrText => {
                ocr_engines.insert(format!(
                    "{}\0{}",
                    observation.engine, observation.engine_version
                ));
            }
        }
    }
    let mut extraction_engines = extraction_engines.into_iter().collect::<Vec<_>>();
    let mut ocr_engines = ocr_engines.into_iter().collect::<Vec<_>>();
    extraction_engines.sort();
    ocr_engines.sort();
    profile_extraction == extraction_engines && profile_ocr == ocr_engines
}

pub(super) fn canonical_profile_extraction_engine(
    engine: &NormalizerProfileExtractionEngine,
) -> bool {
    matches!(
        (
            &engine.kind,
            engine.engine.as_str(),
            engine.version.as_str()
        ),
        (
            NormalizerProfileExtractionKind::NativeText,
            "pdfkit",
            "macos-page-string-v1"
        ) | (
            NormalizerProfileExtractionKind::TableCell,
            "rust-csv",
            "1.4.0"
        )
    )
}

pub(super) fn canonical_profile_ocr_engine(engine: &NormalizerProfileOcrEngine) -> bool {
    engine.engine == "apple-vision"
        && engine.version == "vnrecognizetextrequest-revision-3-accurate"
}

pub(super) fn profile_extraction_engine_key(
    kind: &NormalizerProfileExtractionKind,
    engine: &str,
    version: &str,
) -> String {
    let kind = match kind {
        NormalizerProfileExtractionKind::NativeText => "native_text",
        NormalizerProfileExtractionKind::TableCell => "table_cell",
    };
    format!("{kind}\0{engine}\0{version}")
}

pub(super) fn provider_normalization_profile_id(profile: &NormalizerProfile) -> String {
    let engines = profile
        .extraction_engines
        .iter()
        .map(|engine| {
            let kind = match &engine.kind {
                NormalizerProfileExtractionKind::NativeText => "native_text",
                NormalizerProfileExtractionKind::TableCell => "table_cell",
            };
            format!("extract-{kind}-{}-{}", engine.engine, engine.version)
        })
        .chain(
            profile
                .ocr_engines
                .iter()
                .map(|engine| format!("ocr-{}-{}", engine.engine, engine.version)),
        )
        .collect::<Vec<_>>()
        .join("+");
    format!(
        "mock:{}:native-observations-v1:{engines}",
        profile.package_id
    )
}

pub(super) fn valid_profiled_proposal(proposal: &NormalizerProposal) -> bool {
    if proposal.status != "valid"
        || proposal.document.provider_key.is_empty()
        || proposal.document.provider_key.len() > 128
        || proposal.document.document_type.is_empty()
        || proposal.document.document_type.len() > 128
        || proposal.accounts.is_empty()
        || proposal.accounts.len() > 128
    {
        return false;
    }
    let account_ids = proposal
        .accounts
        .iter()
        .map(|account| account.proposal_account_id.as_str())
        .collect::<HashSet<_>>();
    if account_ids.len() != proposal.accounts.len()
        || proposal.accounts.iter().any(|account| {
            account.proposal_account_id.is_empty()
                || account.proposal_account_id.len() > 256
                || account.account_type.is_empty()
                || account.account_type.len() > 128
                || !optional_normalizer_string(&account.currency, 16)
                || !optional_normalizer_string(&account.masked_identifier, 256)
                || !optional_normalizer_string(&account.provider_account_id, 256)
        })
    {
        return false;
    }
    proposal_records(proposal).is_some_and(|records| {
        !records.is_empty()
            && records.iter().all(|record| {
                valid_normalizer_record(record)
                    && record
                        .proposal_account_id
                        .as_deref()
                        .is_some_and(|account_id| account_ids.contains(account_id))
            })
            && records
                .iter()
                .map(|record| record.proposal_record_id.as_str())
                .collect::<HashSet<_>>()
                .len()
                == records.len()
            && records
                .iter()
                .map(|record| record.stable_record_key.as_str())
                .collect::<HashSet<_>>()
                .len()
                == records.len()
    })
}

pub(super) fn proposal_records(proposal: &NormalizerProposal) -> Option<Vec<&NormalizerRecord>> {
    let count = proposal.opening_snapshots.len()
        + proposal.records.len()
        + proposal.closing_snapshots.len();
    if count == 0 || count > 1_000 {
        return None;
    }
    Some(
        proposal
            .opening_snapshots
            .iter()
            .chain(&proposal.records)
            .chain(&proposal.closing_snapshots)
            .collect(),
    )
}

pub(super) fn valid_normalizer_record(record: &NormalizerRecord) -> bool {
    let validation = &record.validation;
    validation.schema_valid
        && validation.raw_grounded
        && validation.deterministic_validation_passed
        && !record.proposal_record_id.is_empty()
        && record.proposal_record_id.len() <= 256
        && !record.stable_record_key.is_empty()
        && record.stable_record_key.len() <= 256
        && matches!(
            record.record_type.as_str(),
            "transaction" | "balance" | "position" | "trade" | "valuation" | "fee" | "interest"
        )
        && record.raw.is_object()
        && serde_json::to_vec(&record.raw).is_ok_and(|raw| raw.len() <= 16 * 1024)
        && optional_normalizer_string(&record.proposal_account_id, 256)
        && optional_normalizer_string(&record.provider_record_id, 256)
        && optional_normalizer_string(&record.event_type, 128)
        && optional_normalizer_string(&record.posted_on, 32)
        && record
            .posting_status
            .as_deref()
            .is_none_or(|value| matches!(value, "provisional" | "posted"))
        && optional_normalizer_string(&record.transaction_on, 32)
        && optional_normalizer_string(&record.posted_at, 64)
        && optional_normalizer_string(&record.description_raw, 4 * 1024)
        && optional_normalizer_string(&record.description_normalized, 4 * 1024)
        && optional_normalizer_string(&record.instrument_symbol, 256)
        && optional_normalizer_string(&record.quantity, 128)
        && optional_normalizer_string(&record.statement_entry_side, 16)
        && [
            record.amount.as_ref(),
            record.account_balance_delta.as_ref(),
            record.balance_after.as_ref(),
            record.valuation.as_ref(),
        ]
        .iter()
        .flatten()
        .all(valid_normalizer_money)
}

pub(super) fn optional_normalizer_string(value: &Option<String>, max_bytes: usize) -> bool {
    value
        .as_deref()
        .is_none_or(|value| !value.is_empty() && value.len() <= max_bytes)
}

pub(super) fn valid_normalizer_money(money: &&NormalizerMoney) -> bool {
    !money.value.is_empty()
        && money.value.len() <= 128
        && money.currency.len() == 3
        && money.currency.bytes().all(|byte| byte.is_ascii_uppercase())
}

pub(super) fn validated_structured_parse_input(
    proposal: &NormalizerProposal,
    profile: &NormalizerProfile,
    account_ids: &[String],
) -> Option<ValidatedStructuredParseInput> {
    if proposal.accounts.len() != account_ids.len() {
        return None;
    }
    let account_ids = proposal
        .accounts
        .iter()
        .zip(account_ids)
        .map(|(account, account_id)| (account.proposal_account_id.as_str(), account_id.as_str()))
        .collect::<HashMap<_, _>>();
    let records = proposal_records(proposal)?
        .into_iter()
        .map(|record| {
            let account_id = account_ids
                .get(record.proposal_account_id.as_deref()?)?
                .to_string();
            let money = [
                record.amount.as_ref(),
                record.account_balance_delta.as_ref(),
                record.balance_after.as_ref(),
                record.valuation.as_ref(),
            ];
            let currency = money.iter().flatten().next()?.currency.clone();
            if money
                .iter()
                .flatten()
                .any(|money| money.currency != currency)
            {
                return None;
            }
            Some(ValidatedExternalRecordInput {
                account_id,
                account_balance_delta: record
                    .account_balance_delta
                    .as_ref()
                    .map(|money| money.value.clone()),
                amount_value: record.amount.as_ref().map(|money| money.value.clone()),
                currency: Some(currency),
                event_type: record.event_type.clone(),
                posted_on: record.posted_on.clone(),
                posting_status: record.posting_status.clone(),
                raw_json: serde_json::to_string(&record.raw).ok()?,
                record_type: record.record_type.clone(),
                stable_record_key: record.stable_record_key.clone(),
                validation_json: serde_json::to_string(&record.validation).ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ValidatedStructuredParseInput {
        normalization_profile_id: profile.id.clone(),
        profile_json: serde_json::to_string(profile).ok()?,
        records,
    })
}

pub(super) fn random_identifier(prefix: &str) -> String {
    let mut random = [0_u8; 16];
    OsRng.fill_bytes(&mut random);
    let mut identifier = String::with_capacity(prefix.len() + 1 + random.len() * 2);
    identifier.push_str(prefix);
    identifier.push('-');
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut identifier, "{byte:02x}").expect("writing to String cannot fail");
    }
    identifier
}

pub(super) fn statement_password_storage_key(money_source_id: &str) -> String {
    format!("money-source:{money_source_id}")
}

pub(super) const VAULT_CANDIDATE_PREFIX: &str = ".vault-create-";

pub(super) fn candidate_name() -> String {
    let mut random = [0_u8; 8];
    OsRng.fill_bytes(&mut random);
    let mut name = String::from(VAULT_CANDIDATE_PREFIX);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut name, "{byte:02x}").expect("writing to String cannot fail");
    }
    name
}

pub(super) fn write_new_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent directory"))?;
    let temporary = parent.join(format!(
        ".cancan-write-{}.tmp",
        random_identifier("recovery")
    ));
    let result = (|| {
        write_new_synced(&temporary, bytes)?;
        fs::rename(&temporary, path)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub(super) fn ensure_copy_outside_vault(
    vault_root: &Path,
    destination: &Path,
) -> Result<(), RuntimeError> {
    let vault_root = fs::canonicalize(vault_root)
        .map_err(|_| RuntimeError::new("source_copy_location_invalid"))?;
    let parent = destination
        .parent()
        .ok_or_else(|| RuntimeError::new("source_copy_location_invalid"))?;
    let parent =
        fs::canonicalize(parent).map_err(|_| RuntimeError::new("source_copy_location_invalid"))?;
    if parent.starts_with(&vault_root)
        || destination
            .canonicalize()
            .is_ok_and(|path| path.starts_with(&vault_root))
    {
        return Err(RuntimeError::new("source_copy_location_invalid"));
    }
    Ok(())
}

pub(super) fn write_export_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent directory"))?;
    let temporary = parent.join(format!(".cancan-export-{}.tmp", random_identifier("copy")));
    let result = (|| {
        write_new_synced(&temporary, bytes)?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    } else {
        // After rename, the complete user copy is visible and cannot be rolled back safely
        // without risking removal of a valid export.
        let _ = sync_directory(parent);
    }
    result
}

pub(super) fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}
