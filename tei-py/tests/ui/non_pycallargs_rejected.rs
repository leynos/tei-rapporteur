//! Ensures `run_with_kwargs` rejects argument shapes outside `PyCallArgs`.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use tei_py::test_support::run_with_kwargs;

fn rejects_plain_string<'py>(run: &Bound<'py, PyAny>, kwargs: &Bound<'py, PyDict>) {
    run_with_kwargs(run, String::from("not call args"), kwargs);
}

fn main() {}
