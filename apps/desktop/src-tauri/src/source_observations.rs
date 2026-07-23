use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, io};
use zeroize::{Zeroize, Zeroizing};

#[cfg(target_os = "macos")]
use crate::viewer::{render_image_preview_png, render_pdf_page_png_with_password};

const CSV_ENGINE: &str = "rust-csv";
const CSV_ENGINE_VERSION: &str = "1.4.0";
const EXTRACTION_VERSION: &str = "native-observations-v1";
const PDF_ENGINE: &str = "pdfkit";
const PDF_ENGINE_VERSION: &str = "macos-page-string-v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceObservationKind {
    NativeText,
    OcrText,
    TableCell,
    DocumentRegion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TextSpan {
    pub(crate) start: u64,
    pub(crate) end: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BoundingBox {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

struct OcrTextBlock {
    text: String,
    confidence: f64,
    vision_bounding_box: BoundingBox,
}

impl Zeroize for OcrTextBlock {
    fn zeroize(&mut self) {
        self.text.zeroize();
    }
}

impl Drop for OcrTextBlock {
    fn drop(&mut self) {
        self.zeroize();
    }
}

trait PdfOcrEngine {
    fn engine(&self) -> &'static str;
    fn engine_version(&self) -> &'static str;
    fn recognize_page(
        &mut self,
        page: u32,
        rendered_page_png: &[u8],
    ) -> io::Result<Vec<OcrTextBlock>>;
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceObservation {
    pub(crate) id: String,
    pub(crate) kind: SourceObservationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) row: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) column: Option<u64>,
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) text_span: Option<TextSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bounding_box: Option<BoundingBox>,
    pub(crate) engine: String,
    pub(crate) engine_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) confidence: Option<f64>,
}

impl Zeroize for SourceObservation {
    fn zeroize(&mut self) {
        self.id.zeroize();
        self.text.zeroize();
        self.engine.zeroize();
        self.engine_version.zeroize();
    }
}

impl Drop for SourceObservation {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractionBundle {
    pub(crate) source_document_id: String,
    pub(crate) file_sha256: String,
    pub(crate) mime_type: String,
    pub(crate) observations: Vec<SourceObservation>,
    pub(crate) metadata: BTreeMap<String, Value>,
}

impl Zeroize for ExtractionBundle {
    fn zeroize(&mut self) {
        self.source_document_id.zeroize();
        self.file_sha256.zeroize();
        self.mime_type.zeroize();
        for observation in &mut self.observations {
            observation.zeroize();
        }
    }
}

impl Drop for ExtractionBundle {
    fn drop(&mut self) {
        self.zeroize();
    }
}

pub(crate) fn extract_bundle(
    source_document_id: &str,
    file_sha256: &str,
    mime_type: &str,
    plaintext: &[u8],
    password: Option<&[u8]>,
) -> io::Result<ExtractionBundle> {
    let observations = match mime_type {
        "application/pdf" => extract_pdf_observations(plaintext, password)?,
        "text/csv" => extract_csv_observations(plaintext)?,
        "image/png" | "image/jpeg" => extract_image_observations(plaintext, mime_type)?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported source observation MIME type",
            ));
        }
    };
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "extractionVersion".to_owned(),
        Value::String(EXTRACTION_VERSION.to_owned()),
    );
    metadata.insert(
        "observationCount".to_owned(),
        Value::from(
            u64::try_from(observations.len())
                .map_err(|_| io::Error::other("source observation count does not fit u64"))?,
        ),
    );
    Ok(ExtractionBundle {
        source_document_id: source_document_id.to_owned(),
        file_sha256: file_sha256.to_owned(),
        mime_type: mime_type.to_owned(),
        observations,
        metadata,
    })
}

fn extract_csv_observations(plaintext: &[u8]) -> io::Result<Vec<SourceObservation>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(plaintext);
    let mut observations = Vec::new();
    for (row_index, record) in reader.records().enumerate() {
        let row = u64::try_from(row_index + 1)
            .map_err(|_| io::Error::other("CSV row index does not fit u64"))?;
        let record = record.map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid UTF-8 CSV evidence")
        })?;
        for (column_index, text) in record.iter().enumerate() {
            let column = u64::try_from(column_index + 1)
                .map_err(|_| io::Error::other("CSV column index does not fit u64"))?;
            observations.push(SourceObservation {
                id: format!("csv-row-{row}-column-{column}"),
                kind: SourceObservationKind::TableCell,
                page: None,
                row: Some(row),
                column: Some(column),
                text: text.to_owned(),
                text_span: None,
                bounding_box: None,
                engine: CSV_ENGINE.to_owned(),
                engine_version: CSV_ENGINE_VERSION.to_owned(),
                confidence: None,
            });
        }
    }
    Ok(observations)
}

#[cfg(target_os = "macos")]
fn extract_pdf_observations(
    plaintext: &[u8],
    password: Option<&[u8]>,
) -> io::Result<Vec<SourceObservation>> {
    let observations = extract_native_pdf_observations(plaintext, password)?;
    let mut engine = VisionOcrEngine;
    extract_pdf_observations_with_ocr(
        observations,
        |page| render_pdf_page_png_with_password(plaintext, page, password),
        &mut engine,
    )
}

#[cfg(target_os = "macos")]
fn extract_native_pdf_observations(
    plaintext: &[u8],
    password: Option<&[u8]>,
) -> io::Result<Vec<SourceObservation>> {
    use objc2::AllocAnyThread;
    use objc2_foundation::{NSData, NSString};
    use objc2_pdf_kit::PDFDocument;

    let data = NSData::with_bytes(plaintext);
    let document = unsafe { PDFDocument::initWithData(PDFDocument::alloc(), &data) }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid PDF evidence"))?;
    if unsafe { document.isLocked() } {
        let password = password
            .and_then(|value| std::str::from_utf8(value).ok())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::PermissionDenied, "PDF password required")
            })?;
        let password = NSString::from_str(password);
        if !unsafe { document.unlockWithPassword(&password) } {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "PDF password required",
            ));
        }
    }
    let page_count = unsafe { document.pageCount() };
    let mut observations = Vec::with_capacity(page_count);
    for page_index in 0..page_count {
        let page = unsafe { document.pageAtIndex(page_index) }
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "PDF page is unavailable"))?;
        let text = unsafe { page.string() }
            .map(|value| value.to_string())
            .unwrap_or_default();
        let page = u32::try_from(page_index + 1)
            .map_err(|_| io::Error::other("PDF page index does not fit u32"))?;
        // JSON consumers use JavaScript string offsets, so spans are UTF-16 code units.
        let end = u64::try_from(text.encode_utf16().count())
            .map_err(|_| io::Error::other("PDF text length does not fit u64"))?;
        observations.push(SourceObservation {
            id: format!("pdf-page-{page}-native-text"),
            kind: SourceObservationKind::NativeText,
            page: Some(page),
            row: None,
            column: None,
            text,
            text_span: Some(TextSpan { start: 0, end }),
            bounding_box: None,
            engine: PDF_ENGINE.to_owned(),
            engine_version: PDF_ENGINE_VERSION.to_owned(),
            confidence: None,
        });
    }
    Ok(observations)
}

fn extract_pdf_observations_with_ocr<RenderPage, OcrEngine>(
    mut observations: Vec<SourceObservation>,
    mut render_page: RenderPage,
    engine: &mut OcrEngine,
) -> io::Result<Vec<SourceObservation>>
where
    RenderPage: FnMut(u32) -> io::Result<Zeroizing<Vec<u8>>>,
    OcrEngine: PdfOcrEngine,
{
    let pages_needing_ocr = observations
        .iter()
        .filter(|observation| {
            observation.kind == SourceObservationKind::NativeText
                && native_text_needs_ocr(&observation.text)
        })
        .map(|observation| {
            observation.page.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "native PDF text observation is missing a page number",
                )
            })
        })
        .collect::<io::Result<Vec<_>>>()?;

    for page in pages_needing_ocr {
        let rendered_page_png = render_page(page)?;
        let recognized_text = engine.recognize_page(page, &rendered_page_png)?;
        let ocr_observations = recognized_text
            .into_iter()
            .enumerate()
            .map(|(index, mut block)| {
                let bounding_box = vision_bounding_box_to_top_left(block.vision_bounding_box)?;
                Ok(SourceObservation {
                    id: format!("pdf-page-{page}-ocr-text-{}", index + 1),
                    kind: SourceObservationKind::OcrText,
                    page: Some(page),
                    row: None,
                    column: None,
                    text: std::mem::take(&mut block.text),
                    text_span: None,
                    bounding_box: Some(bounding_box),
                    engine: engine.engine().to_owned(),
                    engine_version: engine.engine_version().to_owned(),
                    confidence: Some(block.confidence),
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        observations.extend(ocr_observations);
    }

    Ok(observations)
}

#[cfg(target_os = "macos")]
fn extract_image_observations(
    plaintext: &[u8],
    mime_type: &str,
) -> io::Result<Vec<SourceObservation>> {
    let preview = render_image_preview_png(plaintext, mime_type)?;
    let mut engine = VisionOcrEngine;
    extract_image_observations_with_ocr(&preview, &mut engine)
}

fn extract_image_observations_with_ocr<OcrEngine>(
    image: &[u8],
    engine: &mut OcrEngine,
) -> io::Result<Vec<SourceObservation>>
where
    OcrEngine: PdfOcrEngine,
{
    engine
        .recognize_page(1, image)?
        .into_iter()
        .enumerate()
        .map(|(index, mut block)| {
            Ok(SourceObservation {
                id: format!("image-ocr-text-{}", index + 1),
                kind: SourceObservationKind::OcrText,
                page: None,
                row: None,
                column: None,
                text: std::mem::take(&mut block.text),
                text_span: None,
                bounding_box: None,
                engine: engine.engine().to_owned(),
                engine_version: engine.engine_version().to_owned(),
                confidence: Some(block.confidence),
            })
        })
        .collect()
}

fn native_text_needs_ocr(text: &str) -> bool {
    text.trim().is_empty()
        || text.chars().any(|character| {
            matches!(character, '\0' | '\u{fffd}')
                || (character.is_control()
                    && !matches!(
                        character,
                        '\t' | '\n' | '\u{000b}' | '\u{000c}' | '\r' | '\u{0085}'
                    ))
        })
}

fn vision_bounding_box_to_top_left(vision_bounding_box: BoundingBox) -> io::Result<BoundingBox> {
    if !is_unit_square_box(&vision_bounding_box) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Vision returned an invalid normalized bounding box",
        ));
    }
    let top_left_bounding_box = BoundingBox {
        x: vision_bounding_box.x,
        y: 1.0 - vision_bounding_box.y - vision_bounding_box.height,
        width: vision_bounding_box.width,
        height: vision_bounding_box.height,
    };
    if !is_unit_square_box(&top_left_bounding_box) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Vision bounding box could not be converted to top-left coordinates",
        ));
    }
    Ok(top_left_bounding_box)
}

fn is_unit_square_box(bounding_box: &BoundingBox) -> bool {
    bounding_box.x.is_finite()
        && bounding_box.y.is_finite()
        && bounding_box.width.is_finite()
        && bounding_box.height.is_finite()
        && bounding_box.x >= 0.0
        && bounding_box.y >= 0.0
        && bounding_box.width > 0.0
        && bounding_box.height > 0.0
        && bounding_box.x + bounding_box.width <= 1.0
        && bounding_box.y + bounding_box.height <= 1.0
}

#[cfg(target_os = "macos")]
struct VisionOcrEngine;

#[cfg(target_os = "macos")]
impl PdfOcrEngine for VisionOcrEngine {
    fn engine(&self) -> &'static str {
        "apple-vision"
    }

    fn engine_version(&self) -> &'static str {
        "vnrecognizetextrequest-revision-3-accurate"
    }

    fn recognize_page(
        &mut self,
        _page: u32,
        rendered_page_png: &[u8],
    ) -> io::Result<Vec<OcrTextBlock>> {
        use objc2::{AllocAnyThread, ClassType};
        use objc2_foundation::{NSArray, NSData, NSDictionary};
        use objc2_vision::{
            VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizeTextRequestRevision3,
            VNRequestTextRecognitionLevel,
        };

        let image_data = NSData::with_bytes(rendered_page_png);
        let options = NSDictionary::new();
        let handler = VNImageRequestHandler::initWithData_options(
            VNImageRequestHandler::alloc(),
            &image_data,
            &options,
        );
        let request = VNRecognizeTextRequest::new();
        unsafe {
            request
                .as_super()
                .as_super()
                .setRevision(VNRecognizeTextRequestRevision3);
        }
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setUsesLanguageCorrection(true);
        let requests = NSArray::from_slice(&[request.as_super().as_super()]);
        handler
            .performRequests_error(&requests)
            .map_err(|_| io::Error::other("local Vision text recognition failed"))?;

        let results = require_vision_results(request.results())?;
        let blocks = results
            .to_vec()
            .into_iter()
            .filter_map(|observation| {
                let candidate = observation.topCandidates(1).firstObject()?;
                let bounding_box = unsafe { observation.as_super().as_super().boundingBox() };
                Some(OcrTextBlock {
                    text: candidate.string().to_string(),
                    confidence: f64::from(candidate.confidence()),
                    vision_bounding_box: BoundingBox {
                        x: bounding_box.origin.x,
                        y: bounding_box.origin.y,
                        width: bounding_box.size.width,
                        height: bounding_box.size.height,
                    },
                })
            })
            .collect::<Vec<_>>();
        Ok(blocks)
    }
}

fn require_vision_results<T>(results: Option<T>) -> io::Result<T> {
    results.ok_or_else(|| io::Error::other("local Vision returned no result collection"))
}

#[cfg(not(target_os = "macos"))]
fn extract_pdf_observations(
    _plaintext: &[u8],
    _password: Option<&[u8]>,
) -> io::Result<Vec<SourceObservation>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native PDF extraction requires macOS",
    ))
}

#[cfg(not(target_os = "macos"))]
fn extract_image_observations(
    _plaintext: &[u8],
    _mime_type: &str,
) -> io::Result<Vec<SourceObservation>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "local image OCR requires macOS",
    ))
}

#[cfg(test)]
mod tests {
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

    fn native_text_observation(page: u32, text: &str) -> SourceObservation {
        SourceObservation {
            id: format!("pdf-page-{page}-native-text"),
            kind: SourceObservationKind::NativeText,
            page: Some(page),
            row: None,
            column: None,
            text: text.to_owned(),
            text_span: Some(TextSpan {
                start: 0,
                end: text.encode_utf16().count() as u64,
            }),
            bounding_box: None,
            engine: PDF_ENGINE.to_owned(),
            engine_version: PDF_ENGINE_VERSION.to_owned(),
            confidence: None,
        }
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
        let native = native_text_observation(1, "Opening balance\n1,234.56 SGD");
        let mut engine = FakeOcrEngine {
            blocks: Vec::new(),
            calls: Vec::new(),
            fails: false,
        };

        let observations = extract_pdf_observations_with_ocr(
            vec![native],
            |_| panic!("reliable native text must not render a page for Vision"),
            &mut engine,
        )
        .expect("preserve reliable native text without OCR");

        assert!(engine.calls.is_empty());
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].kind, SourceObservationKind::NativeText);
    }

    #[test]
    fn broken_native_text_keeps_native_and_appends_transformed_ocr_observation() {
        let native = native_text_observation(1, "Statement\u{fffd}");
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
            vec![native],
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
        let ocr = &observations[1];
        assert_eq!(ocr.id, "pdf-page-1-ocr-text-1");
        assert_eq!(ocr.kind, SourceObservationKind::OcrText);
        assert_eq!(ocr.page, Some(1));
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
    fn vision_failure_fails_closed_without_relabeling_native_text() {
        let native = native_text_observation(1, "\0");
        let mut engine = FakeOcrEngine {
            blocks: Vec::new(),
            calls: Vec::new(),
            fails: true,
        };

        let error = extract_pdf_observations_with_ocr(
            vec![native],
            |_| Ok(Zeroizing::new(vec![0x89, b'P', b'N', b'G'])),
            &mut engine,
        )
        .expect_err("OCR failure must reject the extraction bundle");

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(engine.calls, vec![1]);
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
        assert!(observations.iter().all(|observation| {
            observation.page.is_none() && observation.bounding_box.is_none()
        }));
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
            write!(&mut pdf, "{} 0 obj\n{}\nendobj\n", index + 1, object)
                .expect("write PDF object");
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
}
