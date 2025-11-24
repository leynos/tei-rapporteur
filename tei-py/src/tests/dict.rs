//! Dictionary exchange unit tests covering the `from_dict` and `to_dict` helpers.

use pyo3::Python;
use pyo3_serde::{from_pyobject, to_pyobject};
use serde_json::{Value, json, to_value};
use tei_core::TeiDocument;

use crate::{Document, from_dict, to_dict};

#[test]
fn from_dict_decodes_documents() {
    Python::with_gil(|py| {
        let fixture =
            TeiDocument::from_title_str("Wolf 359").expect("valid title should construct fixture");
        let payload = to_value(&fixture).expect("serialising fixture to JSON should succeed");
        let py_payload =
            to_pyobject(py, &payload).expect("converting fixture to PyObject should succeed");

        let document = from_dict(py_payload).expect("dictionary payload should decode");
        assert_eq!(document.title(), "Wolf 359");
    });
}

#[test]
fn from_dict_rejects_missing_fields() {
    Python::with_gil(|py| {
        let payload = json!({ "text": {} });
        let py_payload =
            to_pyobject(py, &payload).expect("serialising malformed payload should succeed");

        let error = from_dict(py_payload).expect_err("missing header should fail");
        assert!(
            error.to_string().contains("missing field"),
            "error should mention missing fields"
        );
    });
}

#[test]
fn from_dict_rejects_blank_title() {
    Python::with_gil(|py| {
        let fixture =
            TeiDocument::from_title_str("Wolf 359").expect("valid title should construct fixture");
        let mut payload = to_value(&fixture).expect("serialising fixture to JSON should succeed");

        let maybe_title = payload
            .as_object_mut()
            .and_then(|root| root.get_mut("teiHeader"))
            .and_then(Value::as_object_mut)
            .and_then(|header| header.get_mut("fileDesc"))
            .and_then(Value::as_object_mut)
            .and_then(|file_desc| file_desc.get_mut("title"));

        if let Some(title) = maybe_title {
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

#[test]
fn to_dict_serialises_documents() {
    Python::with_gil(|py| {
        let document =
            Document::try_from_title("Bridgewater").expect("valid document should construct");
        let py_payload =
            to_dict(py, &document).expect("serialising document to dict should succeed");
        let value: Value = from_pyobject(py_payload)
            .expect("converting PyObject back to JSON value should succeed");
        let title = value
            .get("teiHeader")
            .and_then(|header| header.get("fileDesc"))
            .and_then(|file_desc| file_desc.get("title"))
            .and_then(Value::as_str)
            .expect("title should be present in dictionary output");
        assert_eq!(title, "Bridgewater");
    });
}
