use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, io};
use zeroize::{Zeroize, Zeroizing};

#[cfg(target_os = "macos")]
use crate::viewer::{
    pdf_password_text, render_image_preview_png, render_pdf_page_png_with_password,
};

const CSV_ENGINE: &str = "rust-csv";
const CSV_ENGINE_VERSION: &str = "1.4.0";
const EXTRACTION_VERSION: &str = "native-observations-v2";
const PDF_ENGINE: &str = "pdfkit";
const PDF_ENGINE_VERSION: &str = "macos-page-string-v2";
const DEFAULT_OCR_ROW_Y_OVERLAP_THRESHOLD: f64 = 0.5;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SourceObservationKind {
    NativeText,
    OcrText,
    TableCell,
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

/// Observation text is pinned to Unicode NFC at the extraction boundary.
///
/// PDFKit and Vision can return canonically equivalent strings in different
/// normalization forms depending on the macOS release (for example a
/// decomposed "e" + combining accent instead of a precomposed "é"), which
/// would otherwise make byte-exact downstream matching depend on the host
/// macOS version.
#[cfg(target_os = "macos")]
fn nfc_normalized_text(value: &objc2_foundation::NSString) -> String {
    value.precomposedStringWithCanonicalMapping().to_string()
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
            .map(pdf_password_text)
            .transpose()?
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
    let mut observations = Vec::new();
    for page_index in 0..page_count {
        let page = unsafe { document.pageAtIndex(page_index) }
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "PDF page is unavailable"))?;
        let text = unsafe { page.string() }
            .map(|value| nfc_normalized_text(&value))
            .unwrap_or_default();
        let page = u32::try_from(page_index + 1)
            .map_err(|_| io::Error::other("PDF page index does not fit u32"))?;

        let trimmed_text = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .unwrap_or(&text);
        let lines: Vec<&str> = trimmed_text
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .collect();
        for (line_index, line) in lines.iter().enumerate() {
            let row = u64::try_from(line_index + 1)
                .map_err(|_| io::Error::other("PDF line row index does not fit u64"))?;
            // JSON consumers use JavaScript string offsets, so spans are UTF-16 code units.
            let end = u64::try_from(line.encode_utf16().count())
                .map_err(|_| io::Error::other("PDF text length does not fit u64"))?;
            observations.push(SourceObservation {
                id: format!("pdf-page-{page}-native-text-{row}"),
                kind: SourceObservationKind::NativeText,
                page: Some(page),
                row: Some(row),
                column: None,
                text: (*line).to_owned(),
                text_span: Some(TextSpan { start: 0, end }),
                bounding_box: None,
                engine: PDF_ENGINE.to_owned(),
                engine_version: PDF_ENGINE_VERSION.to_owned(),
                confidence: None,
            });
        }
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
    let mut page_groups: BTreeMap<u32, Vec<&SourceObservation>> = BTreeMap::new();
    for observation in &observations {
        if observation.kind == SourceObservationKind::NativeText {
            let page = observation.page.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "native PDF text observation is missing a page number",
                )
            })?;
            page_groups.entry(page).or_default().push(observation);
        }
    }

    let mut pages_needing_ocr = Vec::new();
    for (page, page_obs) in page_groups {
        let any_broken = page_obs
            .iter()
            .any(|obs| obs.text.chars().any(is_broken_native_character));
        let all_blank = page_obs.iter().all(|obs| obs.text.trim().is_empty());
        if any_broken || all_blank {
            let has_usable = page_obs
                .iter()
                .any(|obs| native_text_has_usable_content(&obs.text));
            pages_needing_ocr.push((page, !has_usable));
        }
    }

    for (page, ocr_required) in pages_needing_ocr {
        let ocr_observations = (|| {
            let rendered_page_png = render_page(page)?;
            let recognized_text = engine.recognize_page(page, &rendered_page_png)?;
            cluster_pdf_ocr_blocks(
                page,
                recognized_text,
                engine.engine(),
                engine.engine_version(),
                DEFAULT_OCR_ROW_Y_OVERLAP_THRESHOLD,
            )
        })();
        match ocr_observations {
            Ok(ocr_observations) => observations.extend(ocr_observations),
            Err(error) if ocr_required => return Err(error),
            Err(_) => {}
        }
    }

    Ok(observations)
}

fn y_overlap_ratio(box_a: &BoundingBox, box_b: &BoundingBox) -> f64 {
    let top_a = box_a.y;
    let bottom_a = box_a.y + box_a.height;
    let top_b = box_b.y;
    let bottom_b = box_b.y + box_b.height;
    let overlap = (bottom_a.min(bottom_b) - top_a.max(top_b)).max(0.0);
    if overlap <= 0.0 {
        return 0.0;
    }
    let min_height = box_a.height.min(box_b.height);
    if min_height <= 0.0 {
        0.0
    } else {
        overlap / min_height
    }
}

struct ValidatedOcrBlock {
    text: String,
    confidence: f64,
    bounding_box: BoundingBox,
}

impl Zeroize for ValidatedOcrBlock {
    fn zeroize(&mut self) {
        self.text.zeroize();
    }
}

impl Drop for ValidatedOcrBlock {
    fn drop(&mut self) {
        self.zeroize();
    }
}

fn cluster_pdf_ocr_blocks(
    page: u32,
    recognized_text: Vec<OcrTextBlock>,
    engine: &str,
    engine_version: &str,
    y_overlap_threshold: f64,
) -> io::Result<Vec<SourceObservation>> {
    if recognized_text.is_empty() {
        return Ok(Vec::new());
    }

    let mut validated_blocks = Vec::with_capacity(recognized_text.len());
    for mut block in recognized_text {
        let bounding_box = vision_bounding_box_to_top_left(block.vision_bounding_box)?;
        validated_blocks.push(ValidatedOcrBlock {
            text: std::mem::take(&mut block.text),
            confidence: block.confidence,
            bounding_box,
        });
    }

    let n = validated_blocks.len();
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }

    fn union(parent: &mut [usize], i: usize, j: usize) {
        let root_i = find(parent, i);
        let root_j = find(parent, j);
        if root_i != root_j {
            parent[root_i] = root_j;
        }
    }

    for i in 0..n {
        for j in (i + 1)..n {
            if y_overlap_ratio(
                &validated_blocks[i].bounding_box,
                &validated_blocks[j].bounding_box,
            ) >= y_overlap_threshold
            {
                union(&mut parent, i, j);
            }
        }
    }

    let mut groups: BTreeMap<usize, Vec<ValidatedOcrBlock>> = BTreeMap::new();
    for (i, block) in validated_blocks.into_iter().enumerate() {
        let root = find(&mut parent, i);
        groups.entry(root).or_default().push(block);
    }

    struct Cluster {
        min_y: f64,
        min_x: f64,
        bounding_box: BoundingBox,
        avg_confidence: f64,
        text: String,
    }

    impl Zeroize for Cluster {
        fn zeroize(&mut self) {
            self.text.zeroize();
        }
    }

    impl Drop for Cluster {
        fn drop(&mut self) {
            self.zeroize();
        }
    }

    let mut clusters = Vec::with_capacity(groups.len());
    for (_, mut cluster_blocks) in groups {
        cluster_blocks.sort_by(|a, b| {
            a.bounding_box
                .x
                .total_cmp(&b.bounding_box.x)
                .then_with(|| a.bounding_box.y.total_cmp(&b.bounding_box.y))
        });

        let min_x = cluster_blocks
            .iter()
            .map(|b| b.bounding_box.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = cluster_blocks
            .iter()
            .map(|b| b.bounding_box.y)
            .fold(f64::INFINITY, f64::min);
        let max_x = cluster_blocks
            .iter()
            .map(|b| b.bounding_box.x + b.bounding_box.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = cluster_blocks
            .iter()
            .map(|b| b.bounding_box.y + b.bounding_box.height)
            .fold(f64::NEG_INFINITY, f64::max);

        let bounding_box = BoundingBox {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        };

        let avg_confidence = cluster_blocks.iter().map(|b| b.confidence).sum::<f64>()
            / (cluster_blocks.len() as f64);

        let text = cluster_blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        clusters.push(Cluster {
            min_y,
            min_x,
            bounding_box,
            avg_confidence,
            text,
        });
    }

    clusters.sort_by(|a, b| {
        a.min_y
            .total_cmp(&b.min_y)
            .then_with(|| a.min_x.total_cmp(&b.min_x))
    });

    let mut observations = Vec::with_capacity(clusters.len());
    for (index, mut cluster) in clusters.into_iter().enumerate() {
        let row = u64::try_from(index + 1)
            .map_err(|_| io::Error::other("OCR row index does not fit u64"))?;
        observations.push(SourceObservation {
            id: format!("pdf-page-{page}-ocr-text-{row}"),
            kind: SourceObservationKind::OcrText,
            page: Some(page),
            row: Some(row),
            column: None,
            text: std::mem::take(&mut cluster.text),
            text_span: None,
            bounding_box: Some(cluster.bounding_box),
            engine: engine.to_owned(),
            engine_version: engine_version.to_owned(),
            confidence: Some(cluster.avg_confidence),
        });
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

#[cfg(test)]
fn native_text_needs_ocr(text: &str) -> bool {
    text.trim().is_empty() || text.chars().any(is_broken_native_character)
}

fn native_text_has_usable_content(text: &str) -> bool {
    text.chars()
        .any(|character| !character.is_whitespace() && !is_broken_native_character(character))
}

fn is_broken_native_character(character: char) -> bool {
    matches!(character, '\0' | '\u{fffd}')
        || (character.is_control()
            && !matches!(
                character,
                '\t' | '\n' | '\u{000b}' | '\u{000c}' | '\r' | '\u{0085}'
            ))
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
                    text: nfc_normalized_text(&candidate.string()),
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
mod tests;
