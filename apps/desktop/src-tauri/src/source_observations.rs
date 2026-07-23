use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, io};
use zeroize::Zeroize;

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

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BoundingBox {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
