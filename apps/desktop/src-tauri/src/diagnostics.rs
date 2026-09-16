//! Spec 0015 operational diagnostics boundary.
//!
//! Two things live here:
//!
//! * [`RedactedText`] and [`redact`] — the single guarantee that the durable
//!   operational log and `jobs.error_json` never carry document or email
//!   content, extracted text, raw financial fields, amounts, filenames or
//!   filesystem paths, mailbox addresses, OAuth tokens, API keys, statement
//!   passwords, model payloads, or unbounded identifiers that would reveal
//!   them.
//! * The entry types ([`OperationalLogEntry`], [`JobFailureDetail`]) shared by
//!   the store, which persists them, and the runtime, which captures them at
//!   the point where a technical error is collapsed into a static user-facing
//!   code.
//!
//! Redaction is structural rather than best-effort: every string that reaches
//! a persisted diagnostics field is produced by [`redact`], and callers can
//! only build the persisted types through those constructors.

use crate::database::intake::is_control_or_bidi;
use serde::Serialize;
use std::{borrow::Cow, error::Error, fmt::Write as _, io};

/// Operational log entries older than this are deleted when the Vault opens.
pub(crate) const OPERATIONAL_LOG_RETENTION_DAYS: i64 = 30;
/// The app version recorded on every entry.
pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The longest redacted technical text kept per entry or error layer.
const MAX_REDACTED_CHARS: usize = 320;
/// A quoted span longer than this counts as unterminated, not as a value.
const MAX_QUOTED_SPAN_CHARS: usize = 120;
/// Suffixes that make a bare token a document name rather than prose.
const DOCUMENT_SUFFIXES: &[&str] = &[
    "csv", "dat", "db", "doc", "docx", "eml", "heic", "htm", "html", "jpeg", "jpg", "json", "msg",
    "ofx", "pdf", "png", "qbo", "qif", "sqlite", "tif", "tiff", "txt", "xls", "xlsx", "xml", "zip",
];

/// Redacted, bounded, single-line technical text.
///
/// The inner value is only reachable through [`Self::as_str`]; there is no
/// constructor that stores raw input, so a caller cannot persist unredacted
/// text by accident.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RedactedText(String);

impl RedactedText {
    /// Redacts arbitrary text. Used for values that did not come from an
    /// [`Error`], such as a static code that is being recorded as the message
    /// of a technical layer.
    pub(crate) fn from_text(raw: &str) -> Self {
        Self(redact(raw))
    }

    /// Redacts an error's `Display` text.
    pub(crate) fn from_error(error: &(dyn Error + 'static)) -> Self {
        Self(redact(&error.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The error class recorded next to a redacted message, so a report can tell a
/// database failure from a filesystem or serialization failure without reading
/// the message.
pub(crate) fn classify_error(error: &(dyn Error + 'static)) -> &'static str {
    let mut current = Some(error);
    while let Some(error) = current {
        if error.is::<io::Error>() {
            return "io";
        }
        if error.is::<rusqlite::Error>() {
            return "sqlite";
        }
        if error.is::<serde_json::Error>() {
            return "json";
        }
        current = error.source();
    }
    "unknown"
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperationalLogLevel {
    /// The operation was retried or parked; nothing is broken yet.
    Warning,
    /// The operation failed and needs attention.
    Error,
}

impl OperationalLogLevel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// One entry of the durable operational log: a static code plus the redacted
/// technical context the user-facing code drops.
///
/// Every field is fixed-shape or redacted, so the entry can be written from a
/// failure path without a decision about what is safe to keep.
#[derive(Clone, Debug)]
pub(crate) struct OperationalLogEntry {
    pub(crate) level: OperationalLogLevel,
    pub(crate) component: &'static str,
    pub(crate) error_code: Option<Cow<'static, str>>,
    pub(crate) error_kind: Option<&'static str>,
    pub(crate) detail: RedactedText,
    pub(crate) cause: Option<RedactedText>,
    pub(crate) job_type: Option<Cow<'static, str>>,
    pub(crate) job_status: Option<Cow<'static, str>>,
    pub(crate) attempt: Option<i64>,
    pub(crate) duration_ms: Option<i64>,
}

impl OperationalLogEntry {
    /// A failed step, with the error it collapsed into a static code.
    pub(crate) fn failure(
        component: &'static str,
        error_code: &str,
        error: &(dyn Error + 'static),
    ) -> Self {
        Self {
            level: OperationalLogLevel::Error,
            component,
            error_code: Some(Cow::Owned(error_code.to_owned())),
            error_kind: Some(classify_error(error)),
            detail: RedactedText::from_error(error),
            cause: error
                .source()
                .map(|source| RedactedText::from_text(&source.to_string())),
            job_type: None,
            job_status: None,
            attempt: None,
            duration_ms: None,
        }
    }

    /// A failed step that has no error object left, only the static code.
    pub(crate) fn failure_code(component: &'static str, error_code: &str) -> Self {
        Self {
            level: OperationalLogLevel::Error,
            component,
            error_code: Some(Cow::Owned(error_code.to_owned())),
            error_kind: Some("static"),
            detail: RedactedText::from_text(error_code),
            cause: None,
            job_type: None,
            job_status: None,
            attempt: None,
            duration_ms: None,
        }
    }

    pub(crate) fn with_level(mut self, level: OperationalLogLevel) -> Self {
        self.level = level;
        self
    }

    /// Adds the job state the failure belongs to: type, status, attempt count,
    /// and how long the attempt ran.
    pub(crate) fn with_job(
        mut self,
        job_type: impl Into<Cow<'static, str>>,
        job_status: impl Into<Cow<'static, str>>,
        attempt: i64,
        duration_ms: Option<i64>,
    ) -> Self {
        self.job_type = Some(job_type.into());
        self.job_status = Some(job_status.into());
        self.attempt = Some(attempt);
        self.duration_ms = duration_ms;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JobFailureDetail {
    pub(crate) error_kind: &'static str,
    pub(crate) message: RedactedText,
    pub(crate) cause: Option<RedactedText>,
    pub(crate) retryable: bool,
    /// The component that reported the failure when it is not the local store,
    /// for example the document normalizer sidecar.
    pub(crate) provider: Option<&'static str>,
}

impl JobFailureDetail {
    pub(crate) fn from_code(provider: Option<&'static str>, code: &str) -> Self {
        Self {
            error_kind: "static",
            message: RedactedText::from_text(code),
            cause: None,
            retryable: false,
            provider,
        }
    }
}

/// One operational log row read back for preview or export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OperationalLogRow {
    pub(crate) created_at: String,
    pub(crate) level: String,
    pub(crate) component: String,
    pub(crate) error_code: Option<String>,
    pub(crate) error_kind: Option<String>,
    pub(crate) detail: Option<String>,
    pub(crate) cause: Option<String>,
    pub(crate) job_type: Option<String>,
    pub(crate) job_status: Option<String>,
    pub(crate) attempt: Option<i64>,
    pub(crate) duration_ms: Option<i64>,
    pub(crate) app_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct OperationalDiagnosticsCategory {
    pub(crate) label: String,
    pub(crate) count: u32,
}

/// What a diagnostic export would contain, shown to the user before the file
/// is written. It never contains unredacted text: `sample_lines` are the same
/// redacted lines the export writes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct OperationalDiagnosticsPreview {
    pub(crate) app_version: String,
    pub(crate) retention_days: i64,
    /// Retained entries in the Vault.
    pub(crate) entry_count: u32,
    /// The most entries one export can contain.
    pub(crate) export_limit: u32,
    pub(crate) oldest_entry_at: Option<String>,
    pub(crate) newest_entry_at: Option<String>,
    pub(crate) components: Vec<OperationalDiagnosticsCategory>,
    pub(crate) error_codes: Vec<OperationalDiagnosticsCategory>,
    pub(crate) sample_lines: Vec<String>,
}

/// The export file: a header that states the boundary, then one redacted line
/// per entry.
pub(crate) struct OperationalLogExport<'a> {
    pub(crate) app_version: &'a str,
    pub(crate) generated_at: &'a str,
    pub(crate) retention_days: i64,
    /// Retained entries behind this export.
    pub(crate) entry_count: u32,
    pub(crate) rows: &'a [OperationalLogRow],
}

pub(crate) fn render_export_lines(rows: &[OperationalLogRow]) -> Vec<String> {
    rows.iter().map(render_row).collect()
}

pub(crate) fn render_export(export: &OperationalLogExport<'_>) -> String {
    let mut file = String::new();
    let _ = writeln!(file, "CanCan operational diagnostics export");
    let _ = writeln!(file, "app_version: {}", export.app_version);
    let _ = writeln!(file, "generated_at: {}", export.generated_at);
    let _ = writeln!(file, "retention_days: {}", export.retention_days);
    let _ = writeln!(file, "entries_retained: {}", export.entry_count);
    let _ = writeln!(file, "entries_exported: {}", export.rows.len());
    let _ = writeln!(
        file,
        "oldest_exported_at: {}",
        export
            .rows
            .first()
            .map(|row| row.created_at.as_str())
            .unwrap_or("none")
    );
    let _ = writeln!(
        file,
        "newest_exported_at: {}",
        export
            .rows
            .last()
            .map(|row| row.created_at.as_str())
            .unwrap_or("none")
    );
    let _ = writeln!(
        file,
        "truncated: {}",
        export.rows.len() < export.entry_count as usize
    );
    let _ = writeln!(
        file,
        "redaction: document and email content, extracted text, financial fields, \
         amounts, filenames, filesystem paths, mailbox addresses, credentials, and \
         quoted values are removed before this file is written"
    );
    let _ = writeln!(file, "--");
    for line in render_export_lines(export.rows) {
        let _ = writeln!(file, "{line}");
    }
    file
}

fn render_row(row: &OperationalLogRow) -> String {
    let mut line = format!(
        "{} {} component={}",
        row.created_at,
        row.level,
        redact(&row.component)
    );
    if let Some(error_code) = &row.error_code {
        let _ = write!(line, " code={}", redact(error_code));
    }
    if let Some(error_kind) = &row.error_kind {
        let _ = write!(line, " kind={}", redact(error_kind));
    }
    if let Some(job_type) = &row.job_type {
        let _ = write!(line, " job={}", redact(job_type));
    }
    if let Some(job_status) = &row.job_status {
        let _ = write!(line, " status={}", redact(job_status));
    }
    if let Some(attempt) = row.attempt {
        let _ = write!(line, " attempt={attempt}");
    }
    if let Some(duration_ms) = row.duration_ms {
        let _ = write!(line, " duration_ms={duration_ms}");
    }
    let _ = write!(line, " version={}", redact(&row.app_version));
    if let Some(detail) = &row.detail {
        // Redaction runs again on the way out so an entry written before a
        // redaction change cannot leak through an export.
        let _ = write!(
            line,
            " detail={:?}",
            RedactedText::from_text(detail).as_str()
        );
    }
    if let Some(cause) = &row.cause {
        let _ = write!(line, " cause={:?}", RedactedText::from_text(cause).as_str());
    }
    line
}

/// Redacts one technical string. Rules, in order:
///
/// * control and bidirectional characters become spaces and whitespace runs
///   collapse, so one entry is always one line;
/// * quoted spans become `'<value>'` / `"<value>"` because third-party
///   messages (for example serde's `invalid type: integer "12"`) can embed
///   values;
/// * any token that looks like a path or URL, a mailbox address, a document
///   name, a credential or content hash, a bare account number, or a monetary
///   amount becomes a placeholder;
/// * the result is truncated to [`MAX_REDACTED_CHARS`].
pub(crate) fn redact(raw: &str) -> String {
    let collapsed = raw
        .chars()
        .map(|character| {
            if is_control_or_bidi(character) {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let quoted = redact_quoted_spans(&collapsed);
    let tokens = quoted
        .split(' ')
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ");
    truncate(tokens)
}

fn redact_token(token: &str) -> Cow<'_, str> {
    let core = token.trim_matches(|character: char| {
        matches!(
            character,
            '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';' | ':' | '\'' | '"' | '='
        )
    });
    if core.is_empty() {
        return Cow::Borrowed(token);
    }
    if core.contains('/') || core.contains('\\') {
        return Cow::Borrowed("<path>");
    }
    if core.contains('@') {
        return Cow::Borrowed("<email>");
    }
    if is_document_name(core) {
        return Cow::Borrowed("<file>");
    }
    if is_opaque_identifier(core) {
        return Cow::Borrowed("<token>");
    }
    if is_bare_number(core) {
        return Cow::Borrowed("<number>");
    }
    if is_amount(core) {
        return Cow::Borrowed("<amount>");
    }
    Cow::Borrowed(token)
}

fn redact_quoted_spans(text: &str) -> String {
    let characters = text.chars().collect::<Vec<_>>();
    let mut redacted = String::with_capacity(text.len());
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '\'' || character == '"' {
            let limit = (index + 1 + MAX_QUOTED_SPAN_CHARS).min(characters.len());
            let closing = (index + 1..limit).find(|probe| characters[*probe] == character);
            if let Some(closing) = closing {
                redacted.push(character);
                redacted.push_str("<value>");
                redacted.push(character);
                index = closing + 1;
                continue;
            }
        }
        redacted.push(character);
        index += 1;
    }
    redacted
}

fn is_document_name(core: &str) -> bool {
    let lower = core.to_ascii_lowercase();
    lower
        .rsplit_once('.')
        .is_some_and(|(stem, suffix)| !stem.is_empty() && DOCUMENT_SUFFIXES.contains(&suffix))
}

/// A credential, content hash, or other opaque identifier: long, unbroken, and
/// mixing letters with digits.
fn is_opaque_identifier(core: &str) -> bool {
    if core.len() < 24 {
        return false;
    }
    let mut has_digit = false;
    let mut has_letter = false;
    for byte in core.bytes() {
        match byte {
            b'0'..=b'9' => has_digit = true,
            b'a'..=b'z' | b'A'..=b'Z' => has_letter = true,
            b'_' | b'-' | b'+' | b'=' => {}
            _ => return false,
        }
    }
    has_digit && has_letter
}

/// An account, card, or reference number: digits with separators only.
fn is_bare_number(core: &str) -> bool {
    let mut digits = 0_u32;
    for byte in core.bytes() {
        match byte {
            b'0'..=b'9' => digits += 1,
            b'-' | b'_' | b'.' => {}
            _ => return false,
        }
    }
    digits >= 9
}

fn is_amount(core: &str) -> bool {
    let Some(separator) = core.rfind(['.', ',']) else {
        return false;
    };
    let (whole, fraction) = (&core[..separator], &core[separator + 1..]);
    if fraction.len() != 2 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let mut whole = whole.chars();
    match whole.next() {
        Some('-' | '+') => {}
        Some(first) if first.is_ascii_digit() => {}
        _ => return false,
    }
    let mut digits = 0_u32;
    for character in whole {
        if character.is_ascii_digit() {
            digits += 1;
        } else if !matches!(character, ',' | '_' | '.') {
            return false;
        }
    }
    digits > 0
}

fn truncate(text: String) -> String {
    if text.chars().count() <= MAX_REDACTED_CHARS {
        return text;
    }
    let mut truncated = text.chars().take(MAX_REDACTED_CHARS).collect::<String>();
    truncated.push('…');
    truncated
}
