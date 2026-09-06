//! Dictionary exchange unit tests covering the `from_dict` and `to_dict` helpers.

use crate::test_support::with_python;
use anyhow::Result;
use pyo3_serde::{from_pyobject, to_pyobject};
use rstest::{fixture, rstest};
use tei_core::TeiDocument;
use tei_serde::json::Value;
use tei_serde::serde_json::json;

use crate::{Document, from_dict, projection::document_to_value, to_dict};

// Fixtures arrange state rather than assert it, so each one propagates its
// construction failure and the consuming test body decides the verdict.

#[fixture]
fn wolf_document() -> Result<TeiDocument> {
    Ok(TeiDocument::from_title_str("Wolf 359")?)
}

#[fixture]
fn wolf_payload(#[from(wolf_document)] document_result: Result<TeiDocument>) -> Result<Value> {
    Ok(document_to_value(&document_result?)?)
}

#[fixture]
fn bridgewater_document() -> Result<Document> {
    Ok(Document::try_from_title("Bridgewater")?)
}

#[rstest]
fn from_dict_decodes_documents(#[from(wolf_payload)] payload_result: Result<Value>) {
    let payload = payload_result.expect("wolf payload fixture should build");
    with_python(|py| {
        let py_payload =
            to_pyobject(py, &payload).expect("converting fixture to PyObject should succeed");

        let document = from_dict(py_payload).expect("dictionary payload should decode");
        assert_eq!(document.title(), "Wolf 359");
    });
}

#[test]
fn from_dict_rejects_missing_fields() {
    with_python(|py| {
        let payload = json!({ "text": {} });
        let py_payload =
            to_pyobject(py, &payload).expect("serializing malformed payload should succeed");

        let error = from_dict(py_payload).expect_err("missing header should fail");
        assert!(
            error.to_string().contains("missing field"),
            "error should mention missing fields"
        );
    });
}

#[rstest]
fn from_dict_rejects_blank_title(#[from(wolf_document)] document_result: Result<TeiDocument>) {
    let document = document_result.expect("wolf document fixture should build");
    with_python(|py| {
        let mut payload =
            document_to_value(&document).expect("serializing fixture to JSON should succeed");

        if let Some(title) = payload.pointer_mut("/header/file_desc/title") {
            *title = Value::String("   ".to_owned());
        }

        let py_payload = to_pyobject(py, &payload)
            .expect("converting mutated fixture to PyObject should succeed");

        let error =
            from_dict(py_payload).expect_err("blank title in otherwise valid payload should fail");
        let message = error.to_string();
        assert!(
            message.contains("document title may not be empty"),
            "error should mention blank titles, got: {message}"
        );
    });
}

#[rstest]
fn to_dict_serializes_documents(#[from(bridgewater_document)] document_result: Result<Document>) {
    let document = document_result.expect("bridgewater document fixture should build");
    with_python(|py| {
        let py_payload =
            to_dict(py, &document).expect("serializing document to dict should succeed");
        let value: Value = from_pyobject(py_payload)
            .expect("converting PyObject back to JSON value should succeed");
        let title = value
            .pointer("/header/file_desc/title")
            .and_then(Value::as_str)
            .expect("title should be present in dictionary output");
        assert_eq!(title, "Bridgewater");
    });
}
