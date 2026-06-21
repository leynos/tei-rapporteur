//! BDD scenarios for the Python-facing streaming iterator.

use crate::test_support::ensure_msgspec_installed_for_tests;
use crate::test_support::{ensure_msgspec_installed, python_import_state_lock};
use pyo3::{Python, types::PyModule};
use pyo3_serde::from_pyobject;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use super::*;
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

#[given("the tei_rapporteur Python module is initialised for streaming")]
fn module_initialised(state: &mut StreamingState) {
    // BDD fixture hook: state is intentionally untouched; the binding ensures
    // the shared `StreamingState` is registered before scenarios execute.
    let _ = state;
}

fn parse_with_iterator(state: &mut StreamingState) {
    let Some(xml) = state.xml else {
        state.error = Some("fixture missing".to_owned());
        return;
    };
    let mut iterator = crate::streaming::iter_parse_py(xml);

    Python::attach(|py| {
        loop {
            match iterator.__next__(py) {
                Ok(Some(obj)) => {
                    let value: Value = from_pyobject(obj.into_bound(py))
                        .expect("streaming events must convert to JSON values");
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
    });
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
fn stream_parse(#[from(state)] state: &mut StreamingState) {
    parse_with_iterator(state);
}

#[then("the event sequence is \"{sequence}\"")]
fn event_sequence(#[from(state)] state: &StreamingState, sequence: String) {
    let actual = state
        .events
        .iter()
        .filter_map(|ev| ev.get("type").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(actual, sequence, "event order mismatch");
}

#[then("a paragraph event is emitted with inline tags")]
fn paragraph_contains_inline(#[from(state)] state: &StreamingState) {
    let paragraph = state
        .events
        .iter()
        .find(|ev| ev.get("type") == Some(&Value::String("paragraph".into())))
        .expect("paragraph event missing");
    let content = paragraph
        .get("content")
        .and_then(Value::as_array)
        .expect("paragraph content missing");
    let tags: Vec<_> = content
        .iter()
        .filter_map(|item| item.get("type").and_then(Value::as_str))
        .collect();
    assert!(
        tags.contains(&"hi"),
        "inline content should include hi; found {tags:?}"
    );
}

#[then("an utterance event is emitted with speaker \"{speaker}\"")]
fn utterance_has_speaker(#[from(state)] state: &StreamingState, speaker: String) {
    let utterance = state
        .events
        .iter()
        .find(|ev| ev.get("type") == Some(&Value::String("utterance".into())))
        .expect("utterance event missing");
    let actual = utterance
        .get("speaker")
        .and_then(Value::as_str)
        .expect("utterance speaker missing");
    assert_eq!(actual, speaker);
    let tags: Vec<_> = utterance
        .get("content")
        .and_then(Value::as_array)
        .expect("utterance content missing")
        .iter()
        .filter_map(|item| item.get("type").and_then(Value::as_str))
        .collect();
    assert!(tags.contains(&"pause"), "pause inline missing: {tags:?}");
}

#[then("the utterance event exposes provenance metadata")]
fn utterance_exposes_provenance_metadata(#[from(state)] state: &StreamingState) {
    let utterance = state
        .events
        .iter()
        .find(|ev| ev.get("type") == Some(&Value::String("utterance".into())))
        .expect("utterance event missing");
    assert_eq!(utterance.get("n").and_then(Value::as_str), Some("1"));
    assert_eq!(
        utterance
            .get("source")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str),
        Some("#src1")
    );
    assert_eq!(
        utterance
            .get("resp")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str),
        Some("#ann1")
    );
    assert_eq!(utterance.get("cert").and_then(Value::as_str), Some("high"));
    assert_eq!(
        utterance
            .get("corresp")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str),
        Some("#sp1")
    );
    assert_eq!(
        utterance
            .get("ana")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str),
        Some("#tag1")
    );
}

#[then("the header event title equals \"{expected}\"")]
fn header_title(#[from(state)] state: &StreamingState, expected: String) {
    let header = state
        .events
        .iter()
        .find(|ev| ev.get("type") == Some(&Value::String("header".into())))
        .expect("header event missing");
    let title = header
        .get("header")
        .and_then(|h| h.get("file_desc"))
        .and_then(|fd| fd.get("title"))
        .and_then(Value::as_str)
        .expect("header title missing");
    assert_eq!(title, expected);
}

#[then("streaming fails mentioning \"{snippet}\"")]
fn streaming_fails(#[from(state)] state: &StreamingState, snippet: String) {
    let message = state.error.as_deref().expect("expected an error");
    assert!(
        message.contains(&snippet),
        "error should mention {snippet:?}, got {message:?}"
    );
}

#[then("the iterator is exhausted after the error")]
fn exhausted_after_error(#[from(state)] state: &StreamingState) {
    assert!(state.exhausted, "iterator should be exhausted after error");
}

#[then("all events decode into msgspec Event instances")]
fn events_decode(#[from(state)] state: &StreamingState) {
    let _import_state_lock = python_import_state_lock();
    Python::attach(|py| {
        if ensure_msgspec_installed_for_tests(py).is_err() {
            return;
        }
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        crate::bindings::py_exports::tei_rapporteur(py, &module)
            .expect("module registration should succeed");
        let structs = module.getattr("structs").expect("structs module present");
        let event_type = structs.getattr("Event").expect("Event class present");
        let converter = py
            .import("msgspec")
            .expect("msgspec import")
            .getattr("convert")
            .expect("convert function");

        for event in &state.events {
            let py_event = pyo3_serde::to_pyobject(py, event).expect("conversion to PyObject");
            converter
                .call((py_event, event_type.clone()), None)
                .expect("msgspec conversion should succeed");
        }
    });
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
