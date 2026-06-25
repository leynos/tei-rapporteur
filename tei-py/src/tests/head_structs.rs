//! Unit tests covering `Head` validation in the `tei_rapporteur.structs`
//! submodule.

use super::*;
use crate::test_support::ensure_msgspec_installed_for_tests;
use pyo3::{
    Py, Python,
    exceptions::PyValueError,
    types::{PyAnyMethods, PyModule},
};
use rstest::fixture;

#[fixture]
fn registered_module() -> Option<Py<PyModule>> {
    Python::attach(|py| {
        if ensure_msgspec_installed_for_tests(py).is_err() {
            return None;
        }
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        tei_rapporteur(py, &module).expect("module registration");
        Some(module.unbind())
    })
}

#[rstest::rstest]
fn head_rejects_empty_content(#[from(registered_module)] module: Option<Py<PyModule>>) {
    let Some(registered_module) = module else {
        return;
    };

    Python::attach(|py| {
        let bound_module = registered_module.bind(py);
        let structs = bound_module.getattr("structs").expect("structs module");
        let head_type = structs.getattr("Head").expect("Head class");

        let error = head_type
            .call0()
            .expect_err("Head should reject empty content");
        assert!(
            error.is_instance_of::<PyValueError>(py),
            "empty Head should raise ValueError"
        );
        assert!(
            error
                .to_string()
                .contains("Head must contain at least one Inline node"),
            "error should explain the Head invariant"
        );
    });
}
