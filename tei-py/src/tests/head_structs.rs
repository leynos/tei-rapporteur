//! Unit tests covering `Head` validation in the `tei_rapporteur.structs`
//! submodule.

use super::*;
use crate::test_support::{ensure_msgspec_available, with_python};
use pyo3::{
    Py,
    exceptions::PyValueError,
    types::{PyAnyMethods, PyModule},
};
use rstest::fixture;

#[fixture]
fn registered_module() -> Py<PyModule> {
    assert!(
        ensure_msgspec_available(),
        "msgspec bootstrap should succeed for Head struct tests"
    );

    with_python(|py| {
        py.import("msgspec")
            .expect("msgspec should import after bootstrap");

        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        tei_rapporteur(py, &module).expect("module registration");
        module.unbind()
    })
}

#[rstest::rstest]
fn head_rejects_empty_content(#[from(registered_module)] module: Py<PyModule>) {
    with_python(|py| {
        let bound_module = module.bind(py);
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
