//! Integration-style tests for the `PyO3` bindings that require module wiring.

use crate::test_support::ensure_msgspec_installed;
use pyo3::types::{PyAnyMethods, PyList};
use pyo3::{Python, types::PyModule};

#[test]
fn to_dict_rejects_non_document_inputs() {
    Python::with_gil(|py| {
        if ensure_msgspec_installed(py).is_err() {
            return;
        }
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

#[test]
fn spoken_text_segments_return_msgspec_structs() {
    Python::with_gil(|py| {
        if ensure_msgspec_installed(py).is_err() {
            return;
        }
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        crate::bindings::py_exports::tei_rapporteur(py, &module)
            .expect("module registration should succeed");
        let extractor = module
            .getattr("spoken_text_segments")
            .expect("spoken_text_segments should be registered");
        let xml = concat!(
            "<TEI>",
            "<teiHeader><fileDesc><title>Example</title></fileDesc></teiHeader>",
            "<text><body>",
            "<sp><speaker>Host</speaker><p xml:id=\"line-1\">Hello <seg>there</seg>.</p></sp>",
            "</body></text>",
            "</TEI>"
        );

        let result = extractor
            .call1((xml,))
            .expect("spoken extraction should succeed");
        let segments = result
            .downcast::<PyList>()
            .expect("spoken extraction should return a list");
        assert_eq!(segments.len().expect("list length should be available"), 1);
        let segment = segments.get_item(0).expect("segment should be present");

        let text: String = segment
            .getattr("text")
            .expect("segment should expose text")
            .extract()
            .expect("text should be a string");
        let locator: String = segment
            .getattr("locator")
            .expect("segment should expose locator")
            .extract()
            .expect("locator should be a string");
        let xml_id: Option<String> = segment
            .getattr("xml_id")
            .expect("segment should expose xml_id")
            .extract()
            .expect("xml_id should be optional string");

        assert_eq!(text, "Hello there.");
        assert_eq!(locator, "/TEI/text/body/sp[1]/p[1]");
        assert_eq!(xml_id.as_deref(), Some("line-1"));
    });
}
