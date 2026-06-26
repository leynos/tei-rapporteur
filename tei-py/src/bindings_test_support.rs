//! Test-only helpers for registering the `tei_rapporteur` Python module.
//!
//! This module keeps registration fixtures available to crate tests without
//! growing the production binding module beyond the repository line limit.

use pyo3::{Bound, PyResult, Python, types::PyModule};

pub(crate) fn register_tei_rapporteur_module_for_tests(
    py_context: Python<'_>,
    py_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let _registration_guard =
        crate::test_support::lock_python_module_registration_attached_for_tests(py_context);
    crate::bindings::py_exports::register_tei_rapporteur_module(py_context, py_module)
}
