use super::*;
#[cfg(target_os = "macos")]
use std::io::Write as _;

struct FakeOcrEngine {
    blocks: Vec<OcrTextBlock>,
    calls: Vec<u32>,
    fails: bool,
}

impl PdfOcrEngine for FakeOcrEngine {
    fn engine(&self) -> &'static str {
        "fake-vision"
    }

    fn engine_version(&self) -> &'static str {
        "fake-v1"
    }

    fn recognize_page(
        &mut self,
        page: u32,
        _rendered_page_png: &[u8],
    ) -> io::Result<Vec<OcrTextBlock>> {
        self.calls.push(page);
        if self.fails {
            return Err(io::Error::other("fake Vision failure"));
        }
        Ok(std::mem::take(&mut self.blocks))
    }
}

fn native_text_observations(page: u32, text: &str) -> Vec<SourceObservation> {
    let trimmed_text = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    let lines: Vec<&str> = trimmed_text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let row = (index + 1) as u64;
            SourceObservation {
                id: format!("pdf-page-{page}-native-text-{row}"),
                kind: SourceObservationKind::NativeText,
                page: Some(page),
                row: Some(row),
                column: None,
                text: line.to_owned(),
                text_span: Some(TextSpan {
                    start: 0,
                    end: line.encode_utf16().count() as u64,
                }),
                bounding_box: None,
                engine: PDF_ENGINE.to_owned(),
                engine_version: PDF_ENGINE_VERSION.to_owned(),
                confidence: None,
            }
        })
        .collect()
}

#[test]
fn extracts_deterministic_csv_table_cells() {
    let bundle = extract_bundle(
        "document-csv",
        &"a".repeat(64),
        "text/csv",
        b"date,memo\r\n2026-07-23,coffee\r\n",
        None,
    )
    .expect("extract CSV observations");

    assert_eq!(bundle.observations.len(), 4);
    assert_eq!(bundle.observations[0].id, "csv-row-1-column-1");
    assert_eq!(bundle.observations[0].row, Some(1));
    assert_eq!(bundle.observations[0].column, Some(1));
    assert_eq!(bundle.observations[3].text, "coffee");
    assert_eq!(bundle.observations[3].engine, CSV_ENGINE);
    assert_eq!(bundle.observations[3].engine_version, CSV_ENGINE_VERSION);
}

#[test]
fn zeroizes_job_scoped_observation_text() {
    let mut bundle = extract_bundle(
        "document-csv",
        &"a".repeat(64),
        "text/csv",
        b"account,balance\n000-12345,100.00\n",
        None,
    )
    .expect("extract sensitive CSV observations");

    bundle.zeroize();

    assert!(bundle.source_document_id.is_empty());
    assert!(bundle.file_sha256.is_empty());
    assert!(
        bundle
            .observations
            .iter()
            .all(|observation| observation.text.is_empty())
    );
}

#[test]
fn reliable_native_text_does_not_call_vision() {
    let native = native_text_observations(1, "Opening balance\n1,234.56 SGD");
    let mut engine = FakeOcrEngine {
        blocks: Vec::new(),
        calls: Vec::new(),
        fails: false,
    };

    let observations = extract_pdf_observations_with_ocr(
        native,
        |_| panic!("reliable native text must not render a page for Vision"),
        &mut engine,
    )
    .expect("preserve reliable native text without OCR");

    assert!(engine.calls.is_empty());
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].kind, SourceObservationKind::NativeText);
    assert_eq!(observations[0].row, Some(1));
    assert_eq!(observations[1].kind, SourceObservationKind::NativeText);
    assert_eq!(observations[1].row, Some(2));
}

#[test]
fn broken_native_text_keeps_native_and_appends_transformed_ocr_observation() {
    let native = native_text_observations(1, "Statement\u{fffd}");
    let mut engine = FakeOcrEngine {
        blocks: vec![OcrTextBlock {
            text: "Statement total 123.45".to_owned(),
            confidence: 0.93,
            vision_bounding_box: BoundingBox {
                x: 0.1,
                y: 0.2,
                width: 0.3,
                height: 0.4,
            },
        }],
        calls: Vec::new(),
        fails: false,
    };

    let observations = extract_pdf_observations_with_ocr(
        native,
        |page| {
            assert_eq!(page, 1);
            Ok(Zeroizing::new(vec![0x89, b'P', b'N', b'G']))
        },
        &mut engine,
    )
    .expect("append Vision observations");

    assert_eq!(engine.calls, vec![1]);
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].kind, SourceObservationKind::NativeText);
    assert_eq!(observations[0].row, Some(1));
    assert_eq!(observations[0].id, "pdf-page-1-native-text-1");
    let ocr = &observations[1];
    assert_eq!(ocr.id, "pdf-page-1-ocr-text-1");
    assert_eq!(ocr.kind, SourceObservationKind::OcrText);
    assert_eq!(ocr.page, Some(1));
    assert_eq!(ocr.row, Some(1));
    assert_eq!(ocr.text, "Statement total 123.45");
    assert_eq!(ocr.confidence, Some(0.93));
    assert_eq!(ocr.engine, "fake-vision");
    assert_eq!(ocr.engine_version, "fake-v1");
    let bounding_box = ocr.bounding_box.as_ref().expect("canonical OCR box");
    assert!((bounding_box.x - 0.1).abs() < f64::EPSILON);
    assert!((bounding_box.y - 0.4).abs() < f64::EPSILON);
    assert!((bounding_box.width - 0.3).abs() < f64::EPSILON);
    assert!((bounding_box.height - 0.4).abs() < f64::EPSILON);
}

#[test]
fn broken_nonempty_native_text_survives_vision_failure() {
    let native = native_text_observations(1, "Statement total 123.45\u{fffd}");
    let mut engine = FakeOcrEngine {
        blocks: Vec::new(),
        calls: Vec::new(),
        fails: true,
    };

    let observations = extract_pdf_observations_with_ocr(
        native,
        |_| Ok(Zeroizing::new(vec![0x89, b'P', b'N', b'G'])),
        &mut engine,
    )
    .expect("preserve usable native text when supplemental OCR fails");

    assert_eq!(engine.calls, vec![1]);
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].kind, SourceObservationKind::NativeText);
    assert_eq!(observations[0].row, Some(1));
    assert_eq!(observations[0].text, "Statement total 123.45\u{fffd}");
}

#[test]
fn unusable_native_text_vision_failure_fails_closed() {
    let native = native_text_observations(1, "\0");
    let mut engine = FakeOcrEngine {
        blocks: Vec::new(),
        calls: Vec::new(),
        fails: true,
    };

    let error = extract_pdf_observations_with_ocr(
        native,
        |_| Ok(Zeroizing::new(vec![0x89, b'P', b'N', b'G'])),
        &mut engine,
    )
    .expect_err("OCR failure must reject a page without usable native text");

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(engine.calls, vec![1]);
}

#[test]
fn clusters_pdf_ocr_blocks_by_y_overlap_into_rows_and_sorts_by_x() {
    // Vision coordinates (y=0 at bottom):
    // Top-left transform: y_tl = 1.0 - y_vis - height
    // Line 1: y_tl around 0.1 -> y_vis = 1.0 - 0.1 - 0.05 = 0.85
    // Line 2: y_tl around 0.3 -> y_vis = 1.0 - 0.3 - 0.05 = 0.65
    let blocks = vec![
        // Line 1, block B: x=0.7 (Amount)
        OcrTextBlock {
            text: "123.45".to_owned(),
            confidence: 0.95,
            vision_bounding_box: BoundingBox {
                x: 0.7,
                y: 0.85,
                width: 0.2,
                height: 0.05,
            },
        },
        // Line 2, block D: x=0.1 (Date)
        OcrTextBlock {
            text: "2026-07-24".to_owned(),
            confidence: 0.91,
            vision_bounding_box: BoundingBox {
                x: 0.1,
                y: 0.65,
                width: 0.2,
                height: 0.05,
            },
        },
        // Line 1, block A: x=0.1 (Date)
        OcrTextBlock {
            text: "2026-07-23".to_owned(),
            confidence: 0.92,
            vision_bounding_box: BoundingBox {
                x: 0.1,
                y: 0.849,
                width: 0.2,
                height: 0.05,
            },
        },
        // Line 1, block C: x=0.35 (Description)
        OcrTextBlock {
            text: "Coffee Shop".to_owned(),
            confidence: 0.94,
            vision_bounding_box: BoundingBox {
                x: 0.35,
                y: 0.851,
                width: 0.3,
                height: 0.05,
            },
        },
        // Line 2, block E: x=0.35 (Description)
        OcrTextBlock {
            text: "Tea House 50.00".to_owned(),
            confidence: 0.93,
            vision_bounding_box: BoundingBox {
                x: 0.35,
                y: 0.65,
                width: 0.55,
                height: 0.05,
            },
        },
    ];

    let observations = cluster_pdf_ocr_blocks(
        1,
        blocks,
        "test-engine",
        "test-v1",
        DEFAULT_OCR_ROW_Y_OVERLAP_THRESHOLD,
    )
    .expect("cluster OCR blocks");

    assert_eq!(observations.len(), 2);

    // Row 1: Line 1 (sorted by x: Date, Description, Amount)
    let row1 = &observations[0];
    assert_eq!(row1.id, "pdf-page-1-ocr-text-1");
    assert_eq!(row1.row, Some(1));
    assert_eq!(row1.page, Some(1));
    assert_eq!(row1.text, "2026-07-23 Coffee Shop 123.45");
    let box1 = row1.bounding_box.as_ref().expect("bounding box");
    assert!((box1.x - 0.1).abs() < 1e-6);
    assert!((box1.x + box1.width - 0.9).abs() < 1e-6);
    let expected_conf = (0.95 + 0.92 + 0.94) / 3.0;
    assert!((row1.confidence.unwrap() - expected_conf).abs() < 1e-6);

    // Row 2: Line 2 (sorted by x: Date, Description)
    let row2 = &observations[1];
    assert_eq!(row2.id, "pdf-page-1-ocr-text-2");
    assert_eq!(row2.row, Some(2));
    assert_eq!(row2.page, Some(1));
    assert_eq!(row2.text, "2026-07-24 Tea House 50.00");
}

#[test]
fn ocr_clustering_respects_custom_y_overlap_threshold() {
    // Two blocks with 40% vertical overlap (0.02 overlap on 0.05 height)
    // Block 1: y_vis = 0.80, h = 0.05 (y_tl = 0.15)
    // Block 2: y_vis = 0.77, h = 0.05 (y_tl = 0.18, interval [0.18, 0.23])
    // Overlap in y_tl: [0.18, 0.20] -> length 0.02.
    // Ratio = 0.02 / 0.05 = 0.40.
    let make_blocks = || {
        vec![
            OcrTextBlock {
                text: "Block A".to_owned(),
                confidence: 0.90,
                vision_bounding_box: BoundingBox {
                    x: 0.1,
                    y: 0.80,
                    width: 0.3,
                    height: 0.05,
                },
            },
            OcrTextBlock {
                text: "Block B".to_owned(),
                confidence: 0.90,
                vision_bounding_box: BoundingBox {
                    x: 0.5,
                    y: 0.77,
                    width: 0.3,
                    height: 0.05,
                },
            },
        ]
    };

    // With threshold 0.5: overlap ratio 0.40 < 0.5 -> 2 separate rows
    let separate = cluster_pdf_ocr_blocks(1, make_blocks(), "test", "v1", 0.5)
        .expect("cluster with 0.5 threshold");
    assert_eq!(separate.len(), 2);
    assert_eq!(separate[0].row, Some(1));
    assert_eq!(separate[0].text, "Block A");
    assert_eq!(separate[1].row, Some(2));
    assert_eq!(separate[1].text, "Block B");

    // With threshold 0.3: overlap ratio 0.40 >= 0.3 -> merged into 1 row
    let merged = cluster_pdf_ocr_blocks(1, make_blocks(), "test", "v1", 0.3)
        .expect("cluster with 0.3 threshold");
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].row, Some(1));
    assert_eq!(merged[0].text, "Block A Block B");
}

#[test]
fn mixed_page_with_one_broken_line_triggers_ocr_for_whole_page() {
    let native = vec![
        SourceObservation {
            id: "pdf-page-1-native-text-1".to_owned(),
            kind: SourceObservationKind::NativeText,
            page: Some(1),
            row: Some(1),
            column: None,
            text: "Valid statement header".to_owned(),
            text_span: Some(TextSpan { start: 0, end: 22 }),
            bounding_box: None,
            engine: PDF_ENGINE.to_owned(),
            engine_version: PDF_ENGINE_VERSION.to_owned(),
            confidence: None,
        },
        SourceObservation {
            id: "pdf-page-1-native-text-2".to_owned(),
            kind: SourceObservationKind::NativeText,
            page: Some(1),
            row: Some(2),
            column: None,
            text: "Corrupted row\u{fffd}".to_owned(),
            text_span: Some(TextSpan { start: 0, end: 14 }),
            bounding_box: None,
            engine: PDF_ENGINE.to_owned(),
            engine_version: PDF_ENGINE_VERSION.to_owned(),
            confidence: None,
        },
    ];

    let mut engine = FakeOcrEngine {
        blocks: vec![OcrTextBlock {
            text: "Corrupted row fixed".to_owned(),
            confidence: 0.90,
            vision_bounding_box: BoundingBox {
                x: 0.1,
                y: 0.5,
                width: 0.4,
                height: 0.1,
            },
        }],
        calls: Vec::new(),
        fails: false,
    };

    let observations = extract_pdf_observations_with_ocr(
        native,
        |_| Ok(Zeroizing::new(vec![0x89, b'P', b'N', b'G'])),
        &mut engine,
    )
    .expect("run supplemental OCR for page with broken line");

    assert_eq!(engine.calls, vec![1]);
    assert_eq!(observations.len(), 3);
    assert_eq!(observations[0].id, "pdf-page-1-native-text-1");
    assert_eq!(observations[1].id, "pdf-page-1-native-text-2");
    assert_eq!(observations[2].id, "pdf-page-1-ocr-text-1");
    assert_eq!(observations[2].row, Some(1));
}

#[test]
fn image_ocr_keeps_vision_provenance_without_coordinates() {
    let mut engine = FakeOcrEngine {
        blocks: vec![OcrTextBlock {
            text: "TOTAL 123.45".to_owned(),
            confidence: 0.88,
            vision_bounding_box: BoundingBox {
                x: 0.2,
                y: 0.1,
                width: 0.3,
                height: 0.4,
            },
        }],
        calls: Vec::new(),
        fails: false,
    };

    let observations = extract_image_observations_with_ocr(b"synthetic image", &mut engine)
        .expect("extract image OCR observations");

    assert_eq!(engine.calls, vec![1]);
    assert_eq!(observations.len(), 1);
    let observation = &observations[0];
    assert_eq!(observation.id, "image-ocr-text-1");
    assert_eq!(observation.kind, SourceObservationKind::OcrText);
    assert_eq!(observation.page, None);
    assert_eq!(observation.text, "TOTAL 123.45");
    assert_eq!(observation.engine, "fake-vision");
    assert_eq!(observation.engine_version, "fake-v1");
    assert_eq!(observation.confidence, Some(0.88));
    assert_eq!(observation.bounding_box, None);
}

#[test]
fn image_ocr_failure_rejects_the_bundle() {
    let mut engine = FakeOcrEngine {
        blocks: Vec::new(),
        calls: Vec::new(),
        fails: true,
    };

    let error = extract_image_observations_with_ocr(b"synthetic image", &mut engine)
        .expect_err("image OCR failure must reject the extraction bundle");

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(engine.calls, vec![1]);
}

#[test]
fn detects_only_clear_broken_native_text_markers() {
    assert!(native_text_needs_ocr("   \n\t"));
    assert!(native_text_needs_ocr("bad\0text"));
    assert!(native_text_needs_ocr("bad\u{fffd}text"));
    assert!(native_text_needs_ocr("bad\u{001b}text"));
    assert!(!native_text_has_usable_content("\0\u{fffd}\u{001b}"));
    assert!(native_text_has_usable_content("good\u{fffd}"));
    assert!(!native_text_needs_ocr(
        "native text\nwith normal line breaks"
    ));
    for layout_control in ['\t', '\n', '\u{000b}', '\u{000c}', '\r', '\u{0085}'] {
        assert!(
            !native_text_needs_ocr(&format!("before{layout_control}after")),
            "layout control U+{:04X} must not trigger OCR",
            layout_control as u32
        );
    }
}

#[test]
fn missing_vision_result_collection_fails_closed() {
    assert_eq!(
        require_vision_results::<()>(None)
            .expect_err("missing Vision results must fail")
            .kind(),
        io::ErrorKind::Other
    );
}

#[cfg(target_os = "macos")]
#[test]
fn invokes_local_vision_on_synthetic_text_png() {
    let png = render_pdf_page_png_with_password(&synthetic_text_pdf(), 1, None)
        .expect("render synthetic text page");
    let mut engine = VisionOcrEngine;

    let blocks = engine
        .recognize_page(1, &png)
        .expect("run local Vision on synthetic PNG");
    let recognized = blocks
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        recognized.contains("TOTAL") && recognized.contains("123.45"),
        "Vision did not recognize the synthetic statement text: {recognized:?}"
    );
    assert!(
        blocks
            .iter()
            .all(|block| is_unit_square_box(&block.vision_bounding_box))
    );
}

#[cfg(target_os = "macos")]
#[test]
fn extracts_local_image_ocr_without_location_claims() {
    let png = render_pdf_page_png_with_password(&synthetic_text_pdf(), 1, None)
        .expect("render synthetic text page");

    let observations =
        extract_image_observations(&png, "image/png").expect("extract image OCR text");
    let recognized = observations
        .iter()
        .map(|observation| observation.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        recognized.contains("TOTAL") && recognized.contains("123.45"),
        "Vision did not recognize the synthetic statement text: {recognized:?}"
    );
    assert!(
        observations.iter().all(|observation| {
            observation.page.is_none() && observation.bounding_box.is_none()
        })
    );
}

#[cfg(target_os = "macos")]
#[test]
fn observation_text_is_nfc_normalized_across_macos_forms() {
    use objc2_foundation::NSString;

    // Decomposed form (NFD): "e" + combining acute accent, as returned
    // by PDFKit/Vision on some macOS releases.
    let decomposed = NSString::from_str("Cafe\u{0301}");
    assert_eq!(nfc_normalized_text(&decomposed), "Caf\u{00e9}");
    // Precomposed form (NFC) passes through unchanged.
    let precomposed = NSString::from_str("Caf\u{00e9}");
    assert_eq!(nfc_normalized_text(&precomposed), "Caf\u{00e9}");
}

#[cfg(target_os = "macos")]
fn synthetic_text_pdf() -> Vec<u8> {
    let content = "BT /F1 56 Tf 36 80 Td (TOTAL 123.45) Tj ET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 500 180] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        write!(&mut pdf, "{} 0 obj\n{}\nendobj\n", index + 1, object).expect("write PDF object");
    }
    let xref = pdf.len();
    write!(
        &mut pdf,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    )
    .expect("write xref");
    for offset in offsets {
        writeln!(&mut pdf, "{offset:010} 00000 n ").expect("write xref entry");
    }
    write!(
        &mut pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("write trailer");
    pdf
}
