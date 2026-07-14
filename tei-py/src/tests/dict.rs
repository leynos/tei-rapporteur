//! Dictionary exchange unit tests covering the `from_dict` and `to_dict` helpers.

use crate::test_support::with_python;
use pyo3_serde::{from_pyobject, to_pyobject};
use rstest::{fixture, rstest};
use tei_core::TeiDocument;
use tei_serde::json::Value;
use tei_serde::serde_json::json;

use crate::{Document, from_dict, projection::document_to_value, to_dict};

#[fixture]
fn wolf_document() -> TeiDocument {
    TeiDocument::from_title_str("Wolf 359").expect("valid title should construct fixture")
}

#[fixture]
fn wolf_payload(wolf_document: TeiDocument) -> Value {
    document_to_value(&wolf_document)
        .expect("serializing projection fixture to JSON should succeed")
}

#[fixture]
fn bridgewater_document() -> Document {
    Document::try_from_title("Bridgewater").expect("valid document should construct")
}

#[rstest]
fn from_dict_decodes_documents(wolf_payload: Value) {
    with_python(|py| {
        let py_payload =
            to_pyobject(py, &wolf_payload).expect("converting fixture to PyObject should succeed");

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
fn from_dict_rejects_blank_title(wolf_document: TeiDocument) {
    with_python(|py| {
        let mut payload =
            document_to_value(&wolf_document).expect("serializing fixture to JSON should succeed");

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
fn to_dict_serializes_documents(bridgewater_document: Document) {
    with_python(|py| {
        let py_payload = to_dict(py, &bridgewater_document)
            .expect("serializing document to dict should succeed");
        let value: Value = from_pyobject(py_payload)
            .expect("converting PyObject back to JSON value should succeed");
        let title = value
            .pointer("/header/file_desc/title")
            .and_then(Value::as_str)
            .expect("title should be present in dictionary output");
        assert_eq!(title, "Bridgewater");
    });
}
