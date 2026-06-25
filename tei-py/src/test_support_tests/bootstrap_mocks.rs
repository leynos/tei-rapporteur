//! Mocks for exercising the `msgspec` bootstrap path without network access.
//!
//! This module provides the Python `subprocess.run` replacement used by the
//! property tests in `properties`. It also owns the helpers that inspect calls
//! recorded by both the bootstrap properties and the `run_with_kwargs` unit
//! tests orchestrated from `mod`. The setup functions depend on
//! `test_helpers` for shared subprocess monkeypatch serialization.

use pyo3::{
    Bound, Python, pyclass, pymethods,
    types::{PyAny, PyAnyMethods, PyDict, PyTuple},
};
use std::{
    ffi::CString,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use super::test_helpers::SubprocessPatchGuard;

/// Python-callable counter used as a `subprocess.run` mock.
#[pyclass]
pub(super) struct BootstrapRunCounter {
    count: Arc<AtomicUsize>,
}

/// Owns a Python bootstrap monkeypatch and the lock protecting it.
pub(super) struct BootstrapRunPatch {
    pub(super) globals: pyo3::Py<PyDict>,
    pub(super) patch_guard: SubprocessPatchGuard,
}

#[pymethods]
impl BootstrapRunCounter {
    fn __call__(&self, _args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

pub(super) fn setup_bootstrap_run_counter(
    py: Python<'_>,
    count: Arc<AtomicUsize>,
    patch_guard: SubprocessPatchGuard,
) -> BootstrapRunPatch {
    let globals = PyDict::new(py);
    globals
        .set_item(
            "_counter",
            pyo3::Py::new(py, BootstrapRunCounter { count }).expect("build run counter"),
        )
        .expect("install run counter");
    // Count the forced bootstrap path:
    //   * `force_msgspec_bootstrap_for_tests` makes
    //     `ensure_msgspec_installed` enter the `Once`-guarded installer even
    //     when msgspec is installed on disk;
    //   * the mocked `subprocess.run` counts each call without invoking
    //     package installation, while the property preloads real `msgspec` so
    //     the installer's final `import msgspec` does not need a stub.
    // The counter therefore proves the installer path executed, and the
    // assertions fail if the bootstrap is ever skipped. The returned patch
    // guard serializes subprocess mutation until the caller restores it.
    let patch = CString::new(
        r"
import subprocess
import sys

_original_run = subprocess.run
_calls = []


def _mock_run(*args, **kwargs):
    _calls.append((args, kwargs))
    _counter(args, kwargs)


subprocess.run = _mock_run
",
    )
    .expect("CString build");
    py.run(patch.as_c_str(), Some(&globals), None)
        .expect("monkeypatch subprocess.run");

    BootstrapRunPatch {
        globals: globals.unbind(),
        patch_guard,
    }
}

/// Returns how many mocked `subprocess.run` calls were recorded.
pub(super) fn recorded_call_count(globals: &Bound<'_, PyDict>) -> usize {
    globals
        .get_item("_calls")
        .expect("read recorded subprocess.run calls")
        .len()
        .expect("count recorded subprocess.run calls")
}

/// Returns the positional arguments from the first recorded `subprocess.run`.
pub(super) fn recorded_args<'py>(globals: &Bound<'py, PyDict>) -> Bound<'py, PyAny> {
    let expr = CString::new("_calls[0][0]").expect("CString build");
    globals
        .py()
        .eval(expr.as_c_str(), Some(globals), None)
        .expect("read recorded subprocess.run positional arguments")
}
