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
}

impl SidecarFailure {
    fn detail(&self, provider: &'static str) -> JobFailureDetail {
        match self {
            Self::Protocol(reason) => JobFailureDetail::from_message(Some(provider), reason),
            Self::Transient(reason) => {
                JobFailureDetail::from_transient_message(Some(provider), reason)
            }
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
