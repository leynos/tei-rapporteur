//! Integration-style tests for the `PyO3` bindings that require module wiring.

use pyo3::types::PyAnyMethods;
use pyo3::{Python, types::PyModule};

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
