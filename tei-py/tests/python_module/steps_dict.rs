//! Dictionary-based steps and scenarios for the Python module.

use super::state::{PythonModuleState, python_state};
use anyhow::{Context, Result, bail, ensure};
use pyo3::prelude::*;
use pyo3_serde::{from_pyobject, to_pyobject};
use rstest_bdd_macros::{given, scenario, then, when};
use tei_core::{P, ProfileDesc, TeiDocument, Utterance};
use tei_serde::json::Value;
use tei_serde::serde_json::json;

use tei_py::projection::document_to_value;

const _: fn() -> PythonModuleState = python_state;

#[given("I provide a dictionary payload titled \"{title}\"")]
pub(super) fn i_provide_a_dictionary_payload(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    let document = TeiDocument::from_title_str(title.as_str())
        .context("dictionary fixtures must construct valid documents")?;
    let payload =
        document_to_value(&document).context("serialising fixtures to JSON should succeed")?;
    state.store_dict_payload(payload);
    Ok(())
}

#[given("I provide an invalid dictionary payload missing required fields")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "rstest-bdd step signatures stay uniform even when storing literals"
)]
pub(super) fn i_provide_an_invalid_dictionary_payload(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    state.store_dict_payload(json!({ "text": {} }));
    Ok(())
}

#[given("I provide a dictionary payload with a blank title")]
pub(super) fn i_provide_a_dictionary_payload_with_a_blank_title(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let document = TeiDocument::from_title_str("placeholder")
        .context("placeholder title should construct a fixture")?;
    let mut payload =
        document_to_value(&document).context("serialising placeholder document should succeed")?;

    if let Some(Value::String(title)) = payload
        .get_mut("header")
        .and_then(|header| header.get_mut("file_desc"))
        .and_then(|file_desc| file_desc.get_mut("title"))
    {
        title.clear();
        title.push_str("   ");
    }
    state.store_dict_payload(payload);
    Ok(())
}

#[given("I provide a dictionary payload with duplicate identifiers")]
pub(super) fn i_provide_a_dictionary_payload_with_duplicate_identifiers(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let document =
        TeiDocument::from_title_str("Test").context("test title should construct a fixture")?;
    let mut text = document.text().clone();

    let mut p1 = P::from_text_segments(["Hello"]).context("paragraph should accept content")?;
    p1.set_id("dup").context("identifier should validate")?;

    let mut p2 = P::from_text_segments(["World"]).context("paragraph should accept content")?;
    p2.set_id("dup").context("identifier should validate")?;

    text.body_mut().push_paragraph(p1);
    text.body_mut().push_paragraph(p2);

    let invalid_doc = TeiDocument::new(document.header().clone(), text);
    let payload =
        document_to_value(&invalid_doc).context("serialising fixture to JSON should succeed")?;
    state.store_dict_payload(payload);
    Ok(())
}

#[given("I provide a dictionary payload with an unknown speaker")]
pub(super) fn i_provide_a_dictionary_payload_with_unknown_speaker(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let document =
        TeiDocument::from_title_str("Test").context("test title should construct a fixture")?;

    let mut profile = ProfileDesc::new();
    profile
        .add_speaker("host")
        .context("speaker should validate")?;
    let header = document.header().clone().with_profile_desc(profile);

    let mut text = document.text().clone();
    let utterance = Utterance::from_text_segments(Some("guest"), ["Hi"])
        .context("utterance should accept content")?;
    text.body_mut().push_utterance(utterance);

    let invalid_doc = TeiDocument::new(header, text);
    let payload =
        document_to_value(&invalid_doc).context("serialising fixture to JSON should succeed")?;
    state.store_dict_payload(payload);
    Ok(())
}

#[when("I decode the dictionary payload")]
pub(super) fn i_decode_the_dictionary_payload(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let payload = state.dict_payload()?;
    Python::attach(|py| {
        state.with_module(py, |module| {
            let decoder = module
                .getattr("from_dict")
                .context("from_dict must be registered")?;
            let py_payload =
                to_pyobject(py, &payload).context("converting payload to Python should succeed")?;
            match decoder.call1((py_payload,)) {
                Ok(document) => state.store_document(document.unbind()),
                Err(error) => state.store_error(error.to_string()),
            }
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[when("I encode the constructed Document to a dictionary")]
pub(super) fn i_encode_the_constructed_document_to_a_dictionary(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    Python::attach(|py| {
        state.with_module(py, |module| {
            let encoder = module
                .getattr("to_dict")
                .context("to_dict must be registered")?;
            state.with_document(py, |document| {
                match encoder.call1((document,)) {
                    Ok(payload) => {
                        let value: Value = from_pyobject(payload)?;
                        state.store_dict_output(value.clone());
                        state.store_dict_payload(value);
                    }
                    Err(error) => state.store_error(error.to_string()),
                }
                Ok::<(), anyhow::Error>(())
            })
        })
    })?;
    Ok(())
}

#[when("I encode a dictionary without providing a Document")]
pub(super) fn i_encode_a_dictionary_without_providing_a_document(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    Python::attach(|py| {
        state.with_module(py, |module| {
            let encoder = module
                .getattr("to_dict")
                .context("to_dict must be registered")?;
            match encoder.call1(("not a document",)) {
                Ok(_) => bail!("encoding without a Document should fail"),
                Err(error) => state.store_error(error.to_string()),
            }
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

fn text_from_content(value: &Value) -> Result<&str> {
    value
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|inline| inline.get("value"))
        .and_then(Value::as_str)
        .context("value should include first inline text")
}

#[then("the div structure is preserved")]
pub(super) fn the_div_structure_is_preserved(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    if let Ok(error) = state.error() {
        bail!("{error}");
    }
    let payload = Python::attach(|py| {
        state.with_document(py, |document| {
            let decoded: tei_py::Document = document.extract().map_err(|error| {
                anyhow::anyhow!("decoded document should be a Document: {error}")
            })?;
            document_to_value(&decoded).context("decoded document should project to a dictionary")
        })
    })?;
    let div = payload
        .get("text")
        .and_then(|text| text.get("body"))
        .and_then(|body| body.get("blocks"))
        .and_then(Value::as_array)
        .and_then(|blocks| blocks.first())
        .context("dictionary payload should contain a body division")?;

    ensure!(
        div.get("type").and_then(Value::as_str) == Some("div"),
        "top-level body block should be a division"
    );
    ensure!(
        div.get("div_type").and_then(Value::as_str) == Some("show-notes"),
        "division type should survive dictionary round-trip"
    );
    ensure!(
        div.get("subtype").and_then(Value::as_str) == Some("chapter-markers"),
        "division subtype should survive dictionary round-trip"
    );
    ensure!(
        div.get("head").map(text_from_content).transpose()? == Some("Chapter markers"),
        "division head should survive dictionary round-trip"
    );

    let nested_div = div
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.get(1))
        .context("dictionary payload should contain a nested division")?;
    ensure!(
        nested_div.get("type").and_then(Value::as_str) == Some("div"),
        "nested block should be a division"
    );
    ensure!(
        nested_div.get("head").map(text_from_content).transpose()? == Some("Guest bios"),
        "nested division head should survive dictionary round-trip"
    );

    let item = nested_div
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|list| list.get("items"))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .context("dictionary payload should contain a list item")?;
    ensure!(
        item.get("label").map(text_from_content).transpose()? == Some("1."),
        "list item label should survive dictionary round-trip"
    );
    ensure!(
        text_from_content(item)? == "Transcript",
        "list item content should survive dictionary round-trip"
    );
    Ok(())
}

/// Scenario: Decode a `Document` from a dictionary payload.
#[scenario(path = "tests/features/python_module.feature", index = 13)]
pub fn decodes_dictionary_payloads(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Reject dictionary payloads missing required fields.
#[scenario(path = "tests/features/python_module.feature", index = 14)]
pub fn rejects_incomplete_dictionary_payloads(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Reject dictionary payloads with invalid titles.
#[scenario(path = "tests/features/python_module.feature", index = 15)]
pub fn rejects_blank_titles_in_dictionary_payloads(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Encode a constructed `Document` to a dictionary.
#[scenario(path = "tests/features/python_module.feature", index = 16)]
pub fn encodes_documents_to_dictionaries(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Surface errors when `to_dict` is called without a `Document`.
#[scenario(path = "tests/features/python_module.feature", index = 17)]
pub fn rejects_to_dict_without_document(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Round-trip a div-containing `Document` through a dictionary payload.
#[scenario(path = "tests/features/python_module.feature", index = 20)]
#[expect(
    unused_variables,
    reason = "rstest-bdd injects state through scenario signatures"
)]
pub fn round_trips_div_blocks_via_dictionary(python_state: PythonModuleState) {}
