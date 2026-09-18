//! Canonical-content fingerprint of one locally extracted artifact.
//!
//! Spec `0004-parser-contract.md` defines this fingerprint as equality evidence:
//! it may prove that byte-different files carry the same locally extracted
//! text/table observations, and it is computed inside the trusted host from
//! canonicalized local observations before AI normalization. It never
//! classifies a provider or source, and it can never by itself commit, delete,
//! or overwrite financial facts. Spec `0002-database-schema.md` owns the
//! nullable, deliberately non-unique versioned storage columns.

use crate::source_observations::{ExtractionBundle, SourceObservationKind};
use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Version of the canonical-content fingerprint algorithm.
///
/// A version bump makes every stored fingerprint incomparable, so a match always
/// requires an equal version. Bump it whenever the canonicalization or the hashed
/// observation projection changes.
pub(crate) const CONTENT_FINGERPRINT_VERSION: i64 = 1;

/// Domain separator so a canonical-content digest can never collide with the
/// other SHA-256 digests this Vault stores (file identity, envelope integrity).
const FINGERPRINT_DOMAIN: &[u8] = b"cancan.canonical-content-fingerprint.v1";

/// Canonical content identity of one locally extracted artifact.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ContentFingerprint {
    pub(crate) version: i64,
    pub(crate) hex: String,
}

/// Hashes the canonicalized local observations of one extraction bundle.
///
/// Only the structural position of an observation and its whitespace-collapsed
/// text contribute. Engine names and versions, OCR confidence values, UTF-16
/// spans, bounding boxes, and observation ids stay out: PDF metadata,
/// compression, and encoding changes move or rewrite them without changing the
/// statement content, and OCR confidence is not reproducible across hosts.
///
/// Returns `None` when no observation carries usable text, because extraction
/// that cannot prove equality must continue through normal classification and
/// parsing rather than guess.
pub(crate) fn canonical_content_fingerprint(
    bundle: &ExtractionBundle,
) -> Option<ContentFingerprint> {
    let mut hasher = Sha256::new();
    hasher.update(FINGERPRINT_DOMAIN);
    let mut contributed = false;
    for observation in &bundle.observations {
        let text = canonical_observation_text(&observation.text);
        if text.is_empty() {
            continue;
        }
        contributed = true;
        hasher.update([observation_kind_tag(observation.kind)]);
        for position in [
            observation.page.map(u64::from),
            observation.row,
            observation.column,
        ] {
            hasher.update(position.unwrap_or(u64::MAX).to_be_bytes());
        }
        hasher.update(
            u64::try_from(text.len())
                .expect("canonical observation text length fits u64")
                .to_be_bytes(),
        );
        hasher.update(text.as_bytes());
    }
    if !contributed {
        return None;
    }
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Some(ContentFingerprint {
        version: CONTENT_FINGERPRINT_VERSION,
        hex,
    })
}

/// Whitespace-collapsed text of one observation.
///
/// Leading and trailing whitespace is dropped and every internal run of
/// whitespace becomes one space, so a re-export that only re-wraps lines,
/// re-pads table cells, or changes line endings still hashes identically.
/// Extraction already pins PDF and OCR text to Unicode NFC; table cells compare
/// their extracted UTF-8 bytes.
fn canonical_observation_text(value: &str) -> String {
    let mut canonical = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_whitespace() {
            if !canonical.is_empty() && !canonical.ends_with(' ') {
                canonical.push(' ');
            }
        } else {
            canonical.push(character);
        }
    }
    if canonical.ends_with(' ') {
        canonical.pop();
    }
    canonical
}

fn observation_kind_tag(kind: SourceObservationKind) -> u8 {
    match kind {
        SourceObservationKind::NativeText => 1,
        SourceObservationKind::OcrText => 2,
        SourceObservationKind::TableCell => 3,
    }
}

#[cfg(test)]
mod tests;
