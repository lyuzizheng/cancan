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
                    NormalizerMessage::Ready { .. }
                    | NormalizerMessage::Result { .. }
                    | NormalizerMessage::Error => return fail_normalizer(child),
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
        && !proposal.accounts.is_empty()
        && proposal.accounts.iter().all(|account| {
            matches!(
                account.account_type.as_str(),
                "deposit_account"
                    | "credit_card"
                    | "currency_balance"
                    | "brokerage_account"
                    | "cash_balance"
                    | "position_group"
                    | "insurance_policy"
                    | "manual_asset"
                    | "manual_liability"
            )
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

pub(super) fn candidate_name() -> String {
    let mut random = [0_u8; 8];
    OsRng.fill_bytes(&mut random);
    let mut name = String::from(".vault-create-");
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
