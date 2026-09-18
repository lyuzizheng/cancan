use super::*;
use crate::source_observations::{BoundingBox, SourceObservation, TextSpan};
use std::collections::BTreeMap;

fn observation(kind: SourceObservationKind, page: u32, row: u64, text: &str) -> SourceObservation {
    SourceObservation {
        id: format!("observation-{page}-{row}"),
        kind,
        page: Some(page),
        row: Some(row),
        column: None,
        text: text.to_owned(),
        text_span: Some(TextSpan { start: 0, end: 1 }),
        bounding_box: Some(BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }),
        engine: "pdfkit".to_owned(),
        engine_version: "macos-page-string-v2".to_owned(),
        confidence: Some(0.5),
    }
}

fn bundle(observations: Vec<SourceObservation>) -> ExtractionBundle {
    ExtractionBundle {
        source_document_id: "document-fingerprint".to_owned(),
        file_sha256: "b".repeat(64),
        mime_type: "application/pdf".to_owned(),
        observations,
        metadata: BTreeMap::new(),
    }
}

fn fingerprint(observations: Vec<SourceObservation>) -> ContentFingerprint {
    canonical_content_fingerprint(&bundle(observations)).expect("content fingerprint")
}

#[test]
fn hashes_canonical_content_across_export_and_layout_noise() {
    let mut reexported = observation(SourceObservationKind::NativeText, 1, 2, "TOTAL 1,234.56");
    // A re-export changes the observation id, the extraction engine versions,
    // the UTF-16 span, and the rendered bounding box without changing content.
    reexported.id = "observation-different".to_owned();
    reexported.engine = "pdfkit".to_owned();
    reexported.engine_version = "macos-page-string-v3".to_owned();
    reexported.text_span = Some(TextSpan { start: 4, end: 9 });
    reexported.bounding_box = None;
    reexported.confidence = None;

    assert_eq!(
        fingerprint(vec![observation(
            SourceObservationKind::NativeText,
            1,
            2,
            "  TOTAL 1,234.56\r\n"
        )]),
        fingerprint(vec![reexported]),
    );
}

#[test]
fn collapses_internal_whitespace_without_merging_separate_observations() {
    assert_eq!(
        fingerprint(vec![observation(
            SourceObservationKind::TableCell,
            1,
            1,
            "Spaced  out\tcell"
        )]),
        fingerprint(vec![observation(
            SourceObservationKind::TableCell,
            1,
            1,
            "Spaced out cell"
        )]),
    );
    assert_ne!(
        fingerprint(vec![observation(
            SourceObservationKind::TableCell,
            1,
            1,
            "two cells"
        )]),
        fingerprint(vec![
            observation(SourceObservationKind::TableCell, 1, 1, "two"),
            observation(SourceObservationKind::TableCell, 1, 2, "cells"),
        ]),
    );
}

#[test]
fn separates_content_kind_and_position() {
    let native = fingerprint(vec![observation(
        SourceObservationKind::NativeText,
        1,
        1,
        "amount 12.00",
    )]);
    assert_ne!(
        native,
        fingerprint(vec![observation(
            SourceObservationKind::OcrText,
            1,
            1,
            "amount 12.00"
        )]),
    );
    assert_ne!(
        native,
        fingerprint(vec![observation(
            SourceObservationKind::NativeText,
            2,
            1,
            "amount 12.00"
        )]),
    );
    assert_ne!(
        native,
        fingerprint(vec![observation(
            SourceObservationKind::NativeText,
            1,
            1,
            "amount 12.01"
        )]),
    );
    assert_ne!(
        native,
        fingerprint(vec![observation(
            SourceObservationKind::NativeText,
            1,
            1,
            "12.00"
        )]),
        "the observation text participates in the fingerprint",
    );
}

#[test]
fn withholds_a_fingerprint_when_extraction_produced_no_usable_text() {
    assert!(
        canonical_content_fingerprint(&bundle(Vec::new())).is_none(),
        "an empty extraction cannot prove equality",
    );
    assert!(
        canonical_content_fingerprint(&bundle(vec![
            observation(SourceObservationKind::NativeText, 1, 1, "  "),
            observation(SourceObservationKind::OcrText, 1, 2, ""),
        ]))
        .is_none(),
        "whitespace-only observations cannot prove equality",
    );
}

#[test]
fn reports_the_algorithm_version_with_a_lower_hex_digest() {
    let fingerprint = fingerprint(vec![observation(
        SourceObservationKind::NativeText,
        1,
        1,
        "closing balance 42.00",
    )]);
    assert_eq!(fingerprint.version, CONTENT_FINGERPRINT_VERSION);
    assert_eq!(fingerprint.hex.len(), 64);
    assert!(
        fingerprint
            .hex
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character)),
        "fingerprint must be lower-case hex: {}",
        fingerprint.hex,
    );
}
