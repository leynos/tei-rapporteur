//! Behaviour-driven scenarios that cover the streaming TEI pull parser.
//!
//! The shared harness (captured state, named fixtures, and step definitions)
//! lives in the [`streaming`] module. This file binds the core feature
//! scenarios; namespace-aware coverage lives in [`streaming::namespace`], and
//! the harness-free structural tests live in `streaming_structure.rs`.

#![cfg(feature = "streaming")]

mod streaming;

use rstest_bdd_macros::scenario;
use streaming::support::{StreamingState, validated_state, validated_state_result};
use tei_test_helpers::expect_validated_state;

// Force Cargo to recompile the test binary when the feature file changes so the
// embedded scenarios stay in sync with expectations.
const _: &str = include_str!("features/streaming.feature");

#[scenario(path = "tests/features/streaming.feature", index = 0)]
fn parse_minimal_incrementally(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 1)]
fn yield_paragraphs_as_body_blocks(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 2)]
fn yield_utterances_with_speaker(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 3)]
fn parse_inline_emphasis(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 4)]
fn parse_pause_markers(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 5)]
fn header_accessible_after_parsing(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 6)]
fn report_malformed_xml(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 7)]
fn report_missing_header(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 8)]
fn handle_cdata_in_body(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 9)]
fn handle_eof_after_body(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 10)]
fn header_is_none_before_header_event(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}
