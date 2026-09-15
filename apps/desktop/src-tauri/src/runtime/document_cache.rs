use super::*;

/// The viewer's session cache: page flips and CSV previews would otherwise
/// re-read and re-decrypt the whole envelope every time.
/// The most recently decrypted source document, keyed by the vault session that
/// decrypted it. Viewing a PDF re-reads and re-decrypts the whole envelope for
/// every page flip, so one document is kept until it is replaced by the next
/// viewed document, the vault locks (which advances the session), or the
/// document is deleted. `SourceDocumentFileInput` shares its plaintext through
/// an `Arc`, so a cache hit costs no copy.
pub(super) struct CachedSourceDocument {
    document_id: String,
    vault_session_generation: u64,
    input: SourceDocumentFileInput,
}

impl CachedSourceDocument {
    fn matches(&self, document_id: &str, vault_session_generation: u64, file_sha256: &str) -> bool {
        self.document_id == document_id
            && self.vault_session_generation == vault_session_generation
            && self.input.file_sha256 == file_sha256
    }
}

impl VaultRuntime {
    pub(super) fn cached_source_document(
        &self,
        document_id: &str,
        generation: u64,
        file_sha256: &str,
    ) -> Option<SourceDocumentFileInput> {
        self.inner
            .source_document_cache
            .lock()
            .ok()?
            .as_ref()
            .filter(|cached| cached.matches(document_id, generation, file_sha256))
            .map(|cached| cached.input.clone())
    }

    pub(super) fn remember_source_document(
        &self,
        document_id: &str,
        generation: u64,
        input: &SourceDocumentFileInput,
    ) -> Result<(), RuntimeError> {
        let mut cache = self
            .inner
            .source_document_cache
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))?;
        *cache = Some(CachedSourceDocument {
            document_id: document_id.to_owned(),
            vault_session_generation: generation,
            input: input.clone(),
        });
        Ok(())
    }

    pub(super) fn clear_cached_source_document(&self) -> Result<(), RuntimeError> {
        let mut cache = self
            .inner
            .source_document_cache
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))?;
        *cache = None;
        Ok(())
    }

    pub(super) fn forget_cached_source_document(&self, document_id: &str) {
        if let Ok(mut cache) = self.inner.source_document_cache.lock()
            && cache
                .as_ref()
                .is_some_and(|cached| cached.document_id == document_id)
        {
            *cache = None;
        }
    }
}
