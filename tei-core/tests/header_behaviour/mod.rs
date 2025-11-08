//! Behaviour-driven tests for TEI header assembly and validation.

mod fixtures;
mod helpers;
mod state;
mod steps;

use anyhow::Result;
use rstest_bdd_macros::scenario;

pub(crate) use fixtures::{validated_state, validated_state_result};
pub(crate) use state::HeaderState;

fn expect_validated_header_state(result: Result<HeaderState>) {
    if let Err(error) = result {
        panic!(
            "header scenarios must initialise their state successfully: {error}"
        );
    }
}

#[scenario(path = "tests/features/header.feature", index = 0)]
fn assembles_a_header(
    #[from(validated_state_result)] validated_state: Result<HeaderState>,
) {
    expect_validated_header_state(validated_state);
}

#[scenario(path = "tests/features/header.feature", index = 1)]
fn rejects_blank_revision_notes(
    #[from(validated_state_result)] validated_state: Result<HeaderState>,
) {
    expect_validated_header_state(validated_state);
}
