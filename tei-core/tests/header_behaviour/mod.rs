//! Behaviour-driven tests for TEI header assembly and validation.

mod fixtures;
mod helpers;
mod state;
mod steps;

use anyhow::Result;
use rstest_bdd_macros::scenario;
use tei_test_helpers::expect_validated_state;

pub(crate) use fixtures::{validated_state, validated_state_result};
pub(crate) use state::HeaderState;

#[scenario(path = "tests/features/header.feature", index = 0)]
fn assembles_a_header(
    #[from(validated_state_result)] validated_state: Result<HeaderState>,
) {
    expect_validated_state(validated_state, "header");
}

#[scenario(path = "tests/features/header.feature", index = 1)]
fn rejects_blank_revision_notes(
    #[from(validated_state_result)] validated_state: Result<HeaderState>,
) {
    expect_validated_state(validated_state, "header");
}
