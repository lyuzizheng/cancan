//! Turning a failed sidecar run into a durable failure record.
//!
//! The protocol exchange in `sidecar.rs` is the only place that knows *why* a
//! normalizer or review-core run broke — a timeout, a nonzero exit, an
//! unparsable message, a code the sidecar reported — and the static code the
//! renderer receives cannot express any of it. This module keeps that reason
//! attached to the failure and stops the child, so the owning job row records
//! the failure once, with the attempt it belongs to.

use super::*;
use crate::diagnostics::JobFailureDetail;
use std::fmt;

/// Why a sidecar run ended before it produced a result.
enum SidecarFailure {
    /// A protocol violation: the same exchange would fail the same way again.
    Protocol(String),
    /// Transport trouble, a timeout, or an exit: a later attempt can succeed.
    Transient(String),
    /// A code the sidecar reported on the wire. The wire's retryability
    /// decides the retry, not the transport that carried it.
    Reported {
        reason: String,
        retryable: bool,
        reported_code: Option<&'static str>,
    },
}

impl SidecarFailure {
    fn detail(&self, provider: &'static str) -> JobFailureDetail {
        match self {
            Self::Protocol(reason) => JobFailureDetail::from_message(Some(provider), reason),
            Self::Transient(reason) => {
                JobFailureDetail::from_transient_message(Some(provider), reason)
            }
            Self::Reported {
                reason,
                retryable,
                reported_code,
            } => JobFailureDetail::from_reported_code(
                Some(provider),
                reason,
                *retryable,
                *reported_code,
            ),
        }
    }
}

/// The static half of one sidecar exchange's failures: the component that owns
/// the run and the code its caller publishes.
#[derive(Clone, Copy)]
pub(super) struct SidecarExchange {
    provider: &'static str,
    code: &'static str,
}

impl SidecarExchange {
    pub(super) const fn new(provider: &'static str, code: &'static str) -> Self {
        Self { provider, code }
    }

    /// Ends the run on a protocol violation.
    pub(super) fn protocol(self, child: CommandChild, reason: impl fmt::Display) -> RuntimeError {
        self.failure(child, SidecarFailure::Protocol(reason.to_string()))
    }

    /// Ends the run on a code the sidecar reported on the wire. The label is
    /// the component name the exchange already uses (`the normalizer`, `the
    /// review core`), so the record reads the same from either mode.
    pub(super) fn reported(
        self,
        child: CommandChild,
        label: &str,
        code: &str,
        wire_retryable: Option<bool>,
    ) -> RuntimeError {
        let (retryable, reported_code) = reported_code_retryable(code, wire_retryable);
        self.failure(
            child,
            SidecarFailure::Reported {
                reason: format!("{label} reported {code}"),
                retryable,
                reported_code,
            },
        )
    }

    /// Ends the run on transport trouble, a timeout, or an exit.
    pub(super) fn transient(self, child: CommandChild, reason: impl fmt::Display) -> RuntimeError {
        self.failure(child, SidecarFailure::Transient(reason.to_string()))
    }

    /// A failure that happened before the protocol started: serializing the
    /// request, resolving the binary, or spawning it.
    pub(super) fn transport(
        self,
        error: &(dyn std::error::Error + Send + Sync + 'static),
    ) -> RuntimeError {
        deferred_failure(
            self.code,
            JobFailureDetail::from_error(Some(self.provider), error),
        )
    }

    /// Ends the run, recording why it failed.
    ///
    /// Nothing is logged here: the owning job row writes the code and this
    /// reason together with the attempt they belong to.
    fn failure(self, child: CommandChild, failure: SidecarFailure) -> RuntimeError {
        let _ = child.kill();
        deferred_failure(self.code, failure.detail(self.provider))
    }
}

/// The retry decision for a code the sidecar reported. Spec 0015: the three
/// deterministic parser rejections never retry; `command_failed` retries like
/// any transient failure; a missing or unknown flag falls back to the
/// pre-wire behavior (protocol reports do not retry). Only allowlisted codes
/// are kept structurally; anything else rides the redacted message alone.
fn reported_code_retryable(
    code: &str,
    wire_retryable: Option<bool>,
) -> (bool, Option<&'static str>) {
    match code {
        "normalizer_budget_exhausted" => (false, Some("normalizer_budget_exhausted")),
        "structured_proposal_invalid" => (false, Some("structured_proposal_invalid")),
        "evidence_grounding_failed" => (false, Some("evidence_grounding_failed")),
        "invalid_command" => (false, None),
        "command_failed" => (true, None),
        _ => (wire_retryable.unwrap_or(false), None),
    }
}

/// How a sidecar process stopped, for the record a failed run leaves.
pub(super) fn exit_reason(code: Option<i32>) -> String {
    match code {
        Some(code) => format!("exited with status {code}"),
        None => "was terminated by a signal".to_owned(),
    }
}

/// The last non-empty line a sidecar wrote to stderr, bounded so a runaway log
/// cannot dominate the record.
///
/// Stderr is the only place a crashed sidecar explains itself, and the text is
/// redacted like every other persisted text: paths, tokens, addresses, and
/// amounts never reach the log.
pub(super) fn stderr_tail(bytes: &[u8]) -> String {
    const MAX_STDERR_TAIL_CHARS: usize = 200;
    let text = String::from_utf8_lossy(bytes);
    let last = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    last.trim().chars().take(MAX_STDERR_TAIL_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sidecar_failure_keeps_its_reason_and_retryability() {
        // The review reader decides whether to retry from this field: a
        // protocol violation repeats, an exit or timeout can clear.
        let protocol =
            SidecarFailure::Protocol("the normalizer reported invalid_command".to_owned())
                .detail("normalizer");
        assert_eq!(protocol.provider, Some("normalizer"));
        assert!(!protocol.retryable);
        assert_eq!(
            protocol.message.as_str(),
            "the normalizer reported invalid_command"
        );

        let transient = SidecarFailure::Transient("the normalizer exited with status 1".to_owned())
            .detail("normalizer");
        assert!(transient.retryable);
        assert_eq!(
            transient.message.as_str(),
            "the normalizer exited with status 1"
        );
        assert_eq!(transient.reported_code, None);
    }

    #[test]
    fn a_reported_deterministic_code_never_retries_but_stays_structural() {
        // Spec 0015: the three deterministic parser rejections fail terminally
        // so the job engine never spends AI budget retrying them. The retry of
        // `reported_code_retryable` is a table, not the wire flag, so a lying
        // or stale worker cannot buy retries for a deterministic rejection.
        for (code, stored) in [
            ("normalizer_budget_exhausted", "normalizer_budget_exhausted"),
            ("structured_proposal_invalid", "structured_proposal_invalid"),
            ("evidence_grounding_failed", "evidence_grounding_failed"),
        ] {
            for wire in [None, Some(true), Some(false)] {
                let (retryable, reported) = reported_code_retryable(code, wire);
                assert!(!retryable, "{code} with wire {wire:?} must not retry");
                assert_eq!(reported, Some(stored));
                let detail = SidecarFailure::Reported {
                    reason: format!("the normalizer reported {code}"),
                    retryable,
                    reported_code: reported,
                }
                .detail("normalizer");
                assert!(!detail.retryable);
                assert_eq!(detail.reported_code, Some(stored));
            }
        }
    }

    #[test]
    fn a_reported_transient_code_retries_and_unknown_codes_stay_unstructural() {
        let (retryable, reported) = reported_code_retryable("command_failed", None);
        assert!(retryable);
        assert_eq!(reported, None);

        let (retryable, reported) = reported_code_retryable("some_future_code", Some(true));
        assert!(retryable);
        assert_eq!(reported, None);

        // Pre-wire behavior for protocol reports is preserved: an unknown code
        // without a wire flag does not retry.
        let (retryable, reported) = reported_code_retryable("some_future_code", None);
        assert!(!retryable);
        assert_eq!(reported, None);
    }

    #[test]
    fn the_stderr_tail_keeps_the_last_line_bounded() {
        assert_eq!(
            stderr_tail(b"warning: slow\n\n  fatal: no input  \n"),
            "fatal: no input"
        );
        assert_eq!(stderr_tail(b"\n \n"), "");
        assert_eq!(stderr_tail(&[b'x'; 400]).chars().count(), 200);
    }
}
