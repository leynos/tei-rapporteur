//! Structured debug events for spoken-text extraction.

use std::{fmt::Display, time::Duration};

use super::document_state::DocumentPhase;

/// Logs that spoken text parsing has started.
pub(super) fn parse_started(input_bytes: usize) {
    tracing::debug!(input_bytes, "spoken_text_parse_started");
}

/// Logs a parser error without state context.
pub(super) fn parse_error(error: &dyn Display, input_bytes: usize) {
    tracing::debug!(error = %error, input_bytes, "spoken_text_parse_error");
}

/// Logs a parser error with state-machine context.
pub(super) fn parse_state_error(error: &dyn Display, phase: DocumentPhase, stack_depth: usize) {
    tracing::debug!(error = %error, ?phase, stack_depth, "spoken_text_parse_error");
}

/// Logs successful parser completion and latency fields.
pub(super) fn parse_finished(input_bytes: usize, segment_count: usize, elapsed: Duration) {
    tracing::debug!(
        input_bytes,
        segment_count,
        elapsed_microseconds = elapsed.as_micros(),
        "spoken_text_parse_finished"
    );
}

/// Logs element entry after local-name decoding.
pub(super) fn element_enter(
    element: &str,
    is_empty: bool,
    phase: DocumentPhase,
    stack_depth: usize,
) {
    tracing::debug!(
        element,
        is_empty,
        ?phase,
        stack_depth,
        "spoken_text_element_enter"
    );
}

/// Logs rejection of body markup outside the supported spoken profile.
pub(super) fn unsupported_body_element(element: &str, phase: DocumentPhase, stack_depth: usize) {
    tracing::debug!(
        element,
        ?phase,
        stack_depth,
        "spoken_text_unsupported_body_element"
    );
}

/// Logs a successful TEI-shell state-machine transition.
pub(super) fn phase_transition(from: DocumentPhase, to: DocumentPhase) {
    tracing::debug!(?from, ?to, "spoken_text_phase_transition");
}

/// Logs a rejected TEI-shell state-machine transition.
pub(super) fn phase_rejected(phase: DocumentPhase, next: DocumentPhase, error: &str) {
    tracing::debug!(?phase, ?next, error, "spoken_text_phase_rejected");
}
