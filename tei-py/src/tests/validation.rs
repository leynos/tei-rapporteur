//! Unit tests for the validate Python binding.

use crate::test_support::with_python;
use pyo3::types::{PyAnyMethods, PyModule};

#[test]
fn validate_returns_none_for_valid_document() {
    with_python(|py| {
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        crate::bindings::py_exports::tei_rapporteur(py, &module)
            .expect("module registration should succeed");

        let document_class = module.getattr("Document").expect("Document class");
        let document = document_class.call1(("Test",)).expect("document creation");

        let result = document.call_method0("validate");
        assert!(result.is_ok(), "validate should succeed for valid document");
    });
}

#[test]
fn validate_method_is_registered_on_document() {
    with_python(|py| {
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        crate::bindings::py_exports::tei_rapporteur(py, &module)
            .expect("module registration should succeed");

        let document_class = module.getattr("Document").expect("Document class");
        let document = document_class.call1(("Test",)).expect("document creation");

        let has_validate = document
            .hasattr("validate")
            .expect("hasattr should not raise");
        assert!(has_validate, "Document should have validate method");
    });
}
