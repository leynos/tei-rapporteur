//! Compile-fail fixture: verifies that `run_with_kwargs` rejects a plain `String`.
//!
//! This file is a [`trybuild`] compile-fail fixture. It is compiled by
//! `tei-py/tests/ui.rs` as an independent external crate; it must **not**
//! compile successfully. The companion `non_pycallargs_rejected.stderr` snapshot
//! records the expected `E0277` diagnostic produced when `String` — which does
//! not implement `RunWithKwargsArgs<'py>` — is passed to `run_with_kwargs`.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use tei_py::test_support::run_with_kwargs;

fn rejects_plain_string<'py>(run: &Bound<'py, PyAny>, kwargs: &Bound<'py, PyDict>) {
    run_with_kwargs(run, String::from("not call args"), kwargs);
}

fn main() {}
