//! Dictionary exchange unit tests covering the `from_dict` and `to_dict` helpers.

use pyo3::Python;
use pyo3::types::{PyAnyMethods, PyModule};
use pyo3_serde::{from_pyobject, to_pyobject};
use rstest::{fixture, rstest};
use serde_json::{Value, json, to_value};
use tei_core::TeiDocument;

use crate::{Document, from_dict, to_dict};

#[fixture]
fn wolf_document() -> TeiDocument {
    TeiDocument::from_title_str("Wolf 359").expect("valid title should construct fixture")
}

#[fixture]
fn wolf_payload(wolf_document: TeiDocument) -> Value {
    to_value(&wolf_document).expect("serialising fixture to JSON should succeed")
}

#[fixture]
fn bridgewater_document() -> Document {
    Document::try_from_title("Bridgewater").expect("valid document should construct")
}

fn get_title_field_mut(payload: &mut Value) -> Option<&mut Value> {
    payload
        .as_object_mut()?
        .get_mut("teiHeader")?
        .as_object_mut()?
        .get_mut("fileDesc")?
        .as_object_mut()?
        .get_mut("title")
}

#[rstest]
fn from_dict_decodes_documents(wolf_payload: Value) {
    Python::with_gil(|py| {
        let py_payload =
            to_pyobject(py, &wolf_payload).expect("converting fixture to PyObject should succeed");

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
        let mut payload = to_value(
            TeiDocument::from_title_str("Wolf 359").expect("valid title should construct fixture"),
        )
        .expect("serialising fixture to JSON should succeed");

        if let Some(title) = get_title_field_mut(&mut payload) {
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

#[test]
fn to_dict_rejects_non_document_inputs() {
    Python::with_gil(|py| {
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        crate::bindings::py_exports::tei_rapporteur(py, &module)
            .expect("module registration should succeed");

        let to_dict = module
            .getattr("to_dict")
            .expect("to_dict should be registered");

        let error = to_dict
            .call1((py.None(),))
            .expect_err("passing non-Document should fail with a Python type error");
        assert!(error.is_instance_of::<pyo3::exceptions::PyTypeError>(py));
    });
}
