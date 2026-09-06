//! BDD scenarios for the Python-facing streaming iterator.

use super::*;
use crate::test_support::{bootstrap_msgspec_attached, with_python};
use anyhow::{Result, ensure};
use pyo3::types::PyModule;
use pyo3_serde::from_pyobject;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use tei_serde::json::Value;

const _: &str = include_str!("../../tests/features/python_streaming.feature");

const MINIMAL_TEI: &str = concat!(
    "<TEI>",
    "<teiHeader><fileDesc><title>Wolf 359</title></fileDesc></teiHeader>",
    "<text><body/></text>",
    "</TEI>"
);
const PARAGRAPH_TEI: &str = concat!(
    "<TEI>",
    "<teiHeader><fileDesc><title>Test</title></fileDesc></teiHeader>",
    "<text><body>",
    "<p xml:id=\"p1\">Hi <hi rend=\"stress\">there</hi></p>",
    "</body></text>",
    "</TEI>"
);
const UTTERANCE_TEI: &str = concat!(
    "<TEI>",
    "<teiHeader><fileDesc><title>Test</title></fileDesc></teiHeader>",
    "<text><body>",
    "<u xml:id=\"u1\" n=\"1\" who=\"host\" source=\"#src1\" resp=\"#ann1\" cert=\"high\" corresp=\"#sp1\" ana=\"#tag1\">",
    "Welcome <pause dur=\"PT1S\"/>back",
    "</u>",
    "</body></text>",
    "</TEI>"
);
const MALFORMED_TEI: &str = "<TEI><teiHeader><fileDesc><title>Bad</title></fileDesc></teiHeader>";
const HEADERLESS_TEI: &str = "<TEI><text><body/></text></TEI>";

#[derive(Default)]
struct StreamingState {
    xml: Option<&'static str>,
    events: Vec<Value>,
    error: Option<String>,
    exhausted: bool,
}

#[fixture]
fn state() -> StreamingState {
    StreamingState::default()
}

#[given("the tei_rapporteur Python module is initialized for streaming")]
fn module_initialized(state: &mut StreamingState) {
    // BDD fixture hook: state is intentionally untouched; the binding ensures
    // the shared `StreamingState` is registered before scenarios execute.
    let _ = state;
}

fn parse_with_iterator(state: &mut StreamingState) -> Result<()> {
    let Some(xml) = state.xml else {
        state.error = Some("fixture missing".to_owned());
        return Ok(());
    };
    let mut iterator = crate::streaming::iter_parse_py(xml);

    with_python(|py| {
        loop {
            match iterator.__next__(py) {
                Ok(Some(obj)) => {
                    let value: Value = from_pyobject(obj.into_bound(py))
                        .context("streaming events must convert to JSON values")?;
                    state.events.push(value);
                }
                Ok(None) => {
                    state.exhausted = true;
                    break;
                }
                Err(err) => {
                    state.error = Some(err.to_string());
                    state.exhausted = true;
                    break;
                }
            }
        }
        Ok(())
    })
}

/// Returns the first recorded event carrying the given discriminator.
///
/// Steps query the recorded event stream rather than panicking on a missing
/// event, so a missing event surfaces as a step failure with context.
fn find_event<'a>(state: &'a StreamingState, kind: &str) -> Result<&'a Value> {
    state
        .events
        .iter()
        .find(|event| event.get("type").and_then(Value::as_str) == Some(kind))
        .with_context(|| format!("{kind} event missing"))
}

/// Returns the first entry of a pointer-list valued event field.
fn first_pointer<'a>(event: &'a Value, field: &str) -> Option<&'a str> {
    event
        .get(field)
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
}

/// Returns the discriminators of an event's inline content nodes.
fn inline_types<'a>(event: &'a Value, field: &str) -> Result<Vec<&'a str>> {
    let content = event
        .get(field)
        .and_then(Value::as_array)
        .with_context(|| format!("event {field} missing"))?;
    Ok(content
        .iter()
        .filter_map(|item| item.get("type").and_then(Value::as_str))
        .collect())
}

#[given("the minimal TEI fixture")]
fn minimal_fixture(#[from(state)] state: &mut StreamingState) {
    state.xml = Some(MINIMAL_TEI);
}

#[given("the paragraph TEI fixture")]
fn paragraph_fixture(#[from(state)] state: &mut StreamingState) {
    state.xml = Some(PARAGRAPH_TEI);
}

#[given("the utterance TEI fixture")]
fn utterance_fixture(#[from(state)] state: &mut StreamingState) {
    state.xml = Some(UTTERANCE_TEI);
}

#[given("the malformed TEI fixture")]
fn malformed_fixture(#[from(state)] state: &mut StreamingState) {
    state.xml = Some(MALFORMED_TEI);
}

#[given("the headerless TEI fixture")]
fn headerless_fixture(#[from(state)] state: &mut StreamingState) {
    state.xml = Some(HEADERLESS_TEI);
}

#[when("I stream parse the events")]
fn stream_parse(#[from(state)] state: &mut StreamingState) -> Result<()> {
    parse_with_iterator(state)
}

#[then("the event sequence is \"{sequence}\"")]
fn event_sequence(#[from(state)] state: &StreamingState, sequence: String) -> Result<()> {
    let actual = state
        .events
        .iter()
        .filter_map(|ev| ev.get("type").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(", ");
    ensure!(
        actual == sequence,
        "event order mismatch: expected {sequence:?}, found {actual:?}"
    );
    Ok(())
}

#[then("a paragraph event is emitted with inline tags")]
fn paragraph_contains_inline(#[from(state)] state: &StreamingState) -> Result<()> {
    let paragraph = find_event(state, "paragraph")?;
    let tags = inline_types(paragraph, "content")?;
    ensure!(
        tags.contains(&"hi"),
        "inline content should include hi; found {tags:?}"
    );
    Ok(())
}

#[then("an utterance event is emitted with speaker \"{speaker}\"")]
fn utterance_has_speaker(#[from(state)] state: &StreamingState, speaker: String) -> Result<()> {
    let utterance = find_event(state, "utterance")?;
    let actual = utterance
        .get("speaker")
        .and_then(Value::as_str)
        .context("utterance speaker missing")?;
    ensure!(
        actual == speaker,
        "expected speaker {speaker:?}, found {actual:?}"
    );
    let tags = inline_types(utterance, "content")?;
    ensure!(tags.contains(&"pause"), "pause inline missing: {tags:?}");
    Ok(())
}

#[then("the utterance event exposes provenance metadata")]
fn utterance_exposes_provenance_metadata(#[from(state)] state: &StreamingState) -> Result<()> {
    let utterance = find_event(state, "utterance")?;
    for (field, expected) in [("n", "1"), ("cert", "high")] {
        let actual = utterance.get(field).and_then(Value::as_str);
        ensure!(
            actual == Some(expected),
            "utterance {field} should be {expected:?}, found {actual:?}"
        );
    }
    for (field, expected) in [
        ("source", "#src1"),
        ("resp", "#ann1"),
        ("corresp", "#sp1"),
        ("ana", "#tag1"),
    ] {
        let actual = first_pointer(utterance, field);
        ensure!(
            actual == Some(expected),
            "utterance {field} should be {expected:?}, found {actual:?}"
        );
    }
    Ok(())
}

#[then("the header event title equals \"{expected}\"")]
fn header_title(#[from(state)] state: &StreamingState, expected: String) -> Result<()> {
    let header = find_event(state, "header")?;
    let title = header
        .get("header")
        .and_then(|h| h.get("file_desc"))
        .and_then(|fd| fd.get("title"))
        .and_then(Value::as_str)
        .context("header title missing")?;
    ensure!(
        title == expected,
        "expected header title {expected:?}, found {title:?}"
    );
    Ok(())
}

#[then("streaming fails mentioning \"{snippet}\"")]
fn streaming_fails(#[from(state)] state: &StreamingState, snippet: String) -> Result<()> {
    let message = state.error.as_deref().context("expected an error")?;
    ensure!(
        message.contains(&snippet),
        "error should mention {snippet:?}, got {message:?}"
    );
    Ok(())
}

#[then("the iterator is exhausted after the error")]
fn exhausted_after_error(#[from(state)] state: &StreamingState) -> Result<()> {
    ensure!(state.exhausted, "iterator should be exhausted after error");
    Ok(())
}

#[then("all events decode into msgspec Event instances")]
fn events_decode(#[from(state)] state: &StreamingState) -> Result<()> {
    with_python(|py| {
        if !bootstrap_msgspec_attached(py) {
            rstest_bdd::skip!("msgspec is unavailable in this environment");
        }
        let module = PyModule::new(py, "tei_rapporteur").context("module allocation")?;
        crate::bindings::py_exports::tei_rapporteur(py, &module)
            .context("module registration should succeed")?;
        let event_type = module
            .getattr("structs")
            .context("structs module present")?
            .getattr("Event")
            .context("Event class present")?;
        let converter = py
            .import("msgspec")
            .context("msgspec import")?
            .getattr("convert")
            .context("convert function")?;

        for event in &state.events {
            let py_event = pyo3_serde::to_pyobject(py, event).context("conversion to PyObject")?;
            converter
                .call((py_event, event_type.clone()), None)
                .context("msgspec conversion should succeed")?;
        }
        Ok(())
    })
}

#[scenario(path = "tests/features/python_streaming.feature", index = 0)]
fn minimal_document_streams(state: StreamingState) {
    let _ = state;
}

#[scenario(path = "tests/features/python_streaming.feature", index = 1)]
fn paragraph_streams(state: StreamingState) {
    let _ = state;
}

#[scenario(path = "tests/features/python_streaming.feature", index = 2)]
fn utterance_streams(state: StreamingState) {
    let _ = state;
}

#[scenario(path = "tests/features/python_streaming.feature", index = 3)]
fn header_event_exposes_title(state: StreamingState) {
    let _ = state;
}

#[scenario(path = "tests/features/python_streaming.feature", index = 4)]
fn malformed_xml_errors(state: StreamingState) {
    let _ = state;
}

#[scenario(path = "tests/features/python_streaming.feature", index = 5)]
fn missing_header_errors(state: StreamingState) {
    let _ = state;
}

#[scenario(path = "tests/features/python_streaming.feature", index = 6)]
fn events_decode_with_msgspec(state: StreamingState) {
    let _ = state;
}
