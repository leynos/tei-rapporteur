//! Behaviour-driven scenarios for benchmark fixture generation.
//!
//! These tests verify that generated benchmark fixtures are valid TEI documents
//! that round-trip correctly through both the full-document and streaming parsers.

#![cfg(feature = "streaming")]

use anyhow::ensure;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{TeiDocument, TeiError};
use tei_test_helpers::expect_validated_state;
use tei_xml::fixtures::{BenchFixtureConfig, generate_benchmark_document, generate_benchmark_xml};
use tei_xml::parse_xml;
use tei_xml::streaming::{TeiEvent, TeiPullParser};

// Force Cargo to recompile the test binary when the feature file changes so the
// embedded scenarios stay in sync with expectations.
const _: &str = include_str!("features/benchmark_fixtures.feature");

#[derive(Default)]
struct BenchmarkState {
    config: RefCell<Option<BenchFixtureConfig>>,
    document: RefCell<Option<TeiDocument>>,
    xml: RefCell<Option<String>>,
    streaming_events: RefCell<Vec<Result<TeiEvent, TeiError>>>,
    streaming_error: RefCell<Option<TeiError>>,
    parsed_document: RefCell<Option<TeiDocument>>,
}

impl BenchmarkState {
    fn set_config(&self, config: BenchFixtureConfig) {
        *self.config.borrow_mut() = Some(config);
    }

    fn config(&self) -> anyhow::Result<BenchFixtureConfig> {
        self.config
            .borrow()
            .ok_or_else(|| anyhow::anyhow!("scenario must set a configuration"))
    }

    fn set_document(&self, document: TeiDocument) {
        *self.document.borrow_mut() = Some(document);
    }

    fn document(&self) -> anyhow::Result<TeiDocument> {
        self.document
            .borrow()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("scenario must generate a document"))
    }

    fn set_xml(&self, xml: String) {
        *self.xml.borrow_mut() = Some(xml);
    }

    fn xml(&self) -> anyhow::Result<String> {
        self.xml
            .borrow()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("scenario must generate XML"))
    }

    fn push_event(&self, event: Result<TeiEvent, TeiError>) {
        self.streaming_events.borrow_mut().push(event);
    }

    fn streaming_events(&self) -> Vec<Result<TeiEvent, TeiError>> {
        self.streaming_events.borrow().clone()
    }

    fn set_streaming_error(&self, error: TeiError) {
        *self.streaming_error.borrow_mut() = Some(error);
    }

    fn has_streaming_error(&self) -> bool {
        self.streaming_error.borrow().is_some()
    }

    fn set_parsed_document(&self, document: TeiDocument) {
        *self.parsed_document.borrow_mut() = Some(document);
    }

    fn parsed_document(&self) -> anyhow::Result<TeiDocument> {
        self.parsed_document
            .borrow()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("scenario must parse a document"))
    }
}

#[fixture]
fn validated_state_result() -> anyhow::Result<BenchmarkState> {
    let state = BenchmarkState::default();
    ensure!(
        state.config.borrow().is_none(),
        "config slot must start empty"
    );
    ensure!(
        state.document.borrow().is_none(),
        "document slot must start empty"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> BenchmarkState {
    match validated_state_result() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialise benchmark state: {error}"),
    }
}

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given("a small benchmark configuration")]
fn a_small_benchmark_configuration(#[from(validated_state)] state: &BenchmarkState) {
    state.set_config(BenchFixtureConfig::SMALL);
}

#[given("a medium benchmark configuration")]
fn a_medium_benchmark_configuration(#[from(validated_state)] state: &BenchmarkState) {
    state.set_config(BenchFixtureConfig::MEDIUM);
}

#[given("a large benchmark configuration")]
fn a_large_benchmark_configuration(#[from(validated_state)] state: &BenchmarkState) {
    state.set_config(BenchFixtureConfig::LARGE);
}

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

#[when("a benchmark document is generated")]
fn a_benchmark_document_is_generated(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    let config = state.config()?;
    let document = generate_benchmark_document(&config)
        .map_err(|e| anyhow::anyhow!("generation failed: {e}"))?;
    state.set_document(document);
    Ok(())
}

#[when("benchmark XML is generated")]
fn benchmark_xml_is_generated(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    let config = state.config()?;
    let xml = generate_benchmark_xml(&config)
        .map_err(|e| anyhow::anyhow!("XML generation failed: {e}"))?;
    state.set_xml(xml);
    Ok(())
}

#[when("the XML is parsed with the streaming parser")]
fn the_xml_is_parsed_with_streaming_parser(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    let xml = state.xml()?;
    let parser = TeiPullParser::from_str(&xml);

    for event in parser {
        match &event {
            Ok(_) => state.push_event(event),
            Err(e) => {
                state.set_streaming_error(e.clone());
                state.push_event(event);
                break;
            }
        }
    }
    Ok(())
}

#[when("the XML is parsed with the full parser")]
fn the_xml_is_parsed_with_full_parser(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    let xml = state.xml()?;
    let document = parse_xml(&xml).map_err(|e| anyhow::anyhow!("parsing failed: {e}"))?;
    state.set_parsed_document(document);
    Ok(())
}

// ---------------------------------------------------------------------------
// Then steps
// ---------------------------------------------------------------------------

#[then("the document passes validation")]
fn the_document_passes_validation(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    let document = state.document()?;
    document
        .validate()
        .map_err(|e| anyhow::anyhow!("validation failed: {e}"))?;
    Ok(())
}

#[then("the document contains {count:usize} utterances")]
fn the_document_contains_n_utterances(
    #[from(validated_state)] state: &BenchmarkState,
    count: usize,
) -> anyhow::Result<()> {
    let document = state.document()?;
    let actual = document.text().body().utterances().count();
    ensure!(
        actual == count,
        "expected {count} utterances, found {actual}"
    );
    Ok(())
}

#[then("the document contains {count:usize} paragraphs")]
fn the_document_contains_n_paragraphs(
    #[from(validated_state)] state: &BenchmarkState,
    count: usize,
) -> anyhow::Result<()> {
    let document = state.document()?;
    let actual = document.text().body().paragraphs().count();
    ensure!(
        actual == count,
        "expected {count} paragraphs, found {actual}"
    );
    Ok(())
}

#[then("all events are yielded without errors")]
fn all_events_are_yielded_without_errors(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    ensure!(
        !state.has_streaming_error(),
        "streaming parser yielded an error"
    );
    let events = state.streaming_events();
    ensure!(!events.is_empty(), "no events were yielded");
    Ok(())
}

#[then("the body block count matches the configuration")]
fn the_body_block_count_matches_configuration(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    let config = state.config()?;
    let events = state.streaming_events();
    let body_block_count = events
        .iter()
        .filter(|e| matches!(e, Ok(TeiEvent::BodyBlock(_))))
        .count();
    let expected = config.total_blocks();
    ensure!(
        body_block_count == expected,
        "expected {expected} body blocks, found {body_block_count}"
    );
    Ok(())
}

#[then("the parsed document has the same utterance count")]
fn the_parsed_document_has_same_utterance_count(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    let config = state.config()?;
    let document = state.parsed_document()?;
    let actual = document.text().body().utterances().count();
    ensure!(
        actual == config.utterance_count,
        "expected {} utterances, found {actual}",
        config.utterance_count
    );
    Ok(())
}

#[then("the parsed document has the same paragraph count")]
fn the_parsed_document_has_same_paragraph_count(
    #[from(validated_state)] state: &BenchmarkState,
) -> anyhow::Result<()> {
    let config = state.config()?;
    let document = state.parsed_document()?;
    let actual = document.text().body().paragraphs().count();
    ensure!(
        actual == config.paragraph_count,
        "expected {} paragraphs, found {actual}",
        config.paragraph_count
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario wiring
// ---------------------------------------------------------------------------

#[scenario(path = "tests/features/benchmark_fixtures.feature", index = 0)]
fn generate_small_benchmark_fixture(
    #[from(validated_state)] _: BenchmarkState,
    #[from(validated_state_result)] result: anyhow::Result<BenchmarkState>,
) {
    expect_validated_state(result, "benchmark_fixtures");
}

#[scenario(path = "tests/features/benchmark_fixtures.feature", index = 1)]
fn generate_medium_benchmark_fixture(
    #[from(validated_state)] _: BenchmarkState,
    #[from(validated_state_result)] result: anyhow::Result<BenchmarkState>,
) {
    expect_validated_state(result, "benchmark_fixtures");
}

#[scenario(path = "tests/features/benchmark_fixtures.feature", index = 2)]
fn generate_large_benchmark_fixture(
    #[from(validated_state)] _: BenchmarkState,
    #[from(validated_state_result)] result: anyhow::Result<BenchmarkState>,
) {
    expect_validated_state(result, "benchmark_fixtures");
}

#[scenario(path = "tests/features/benchmark_fixtures.feature", index = 3)]
fn generated_xml_parses_with_streaming_parser(
    #[from(validated_state)] _: BenchmarkState,
    #[from(validated_state_result)] result: anyhow::Result<BenchmarkState>,
) {
    expect_validated_state(result, "benchmark_fixtures");
}

#[scenario(path = "tests/features/benchmark_fixtures.feature", index = 4)]
fn generated_xml_round_trips_through_full_parser(
    #[from(validated_state)] _: BenchmarkState,
    #[from(validated_state_result)] result: anyhow::Result<BenchmarkState>,
) {
    expect_validated_state(result, "benchmark_fixtures");
}
