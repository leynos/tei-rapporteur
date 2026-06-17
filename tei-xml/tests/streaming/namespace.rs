//! Namespace-aware streaming coverage (issue #49). Kept in its own focused
//! module so further namespace cases can be added here rather than enlarging
//! the shared behaviour file. The fixture asserts that a `<TEI>` root carrying
//! the standard TEI default namespace still yields the expected event sequence
//! and a decodable header title.

use anyhow::{bail, ensure};
use rstest_bdd_macros::{scenario, then};
use tei_xml::streaming::TeiEvent;

use super::support::{StreamingState, validated_state, validated_state_result};
use tei_test_helpers::expect_validated_state;

#[then("the header title is \"{title}\"")]
fn the_header_title_is(
    #[from(validated_state)] state: &StreamingState,
    title: String,
) -> anyhow::Result<()> {
    let events = state.events();
    for event in events {
        if let Ok(TeiEvent::Header(header)) = event {
            let actual = header.file_desc().title().as_str();
            ensure!(
                actual == title,
                "header title mismatch: expected {title:?}, found {actual:?}"
            );
            return Ok(());
        }
    }
    bail!("no header event found");
}

#[scenario(path = "tests/features/streaming.feature", index = 11)]
fn parse_namespaced_tei_document_correctly(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}
