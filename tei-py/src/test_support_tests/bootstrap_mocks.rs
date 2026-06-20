//! Mocks for exercising the `msgspec` bootstrap path without network access.
//!
//! This module provides the Python `subprocess.run` replacement used by the
//! property tests in `properties`. It also owns the helpers that inspect calls
//! recorded by both the bootstrap properties and the `run_with_kwargs` unit
//! tests orchestrated from `mod`. The setup functions depend on
//! `test_helpers` for shared subprocess monkeypatch serialisation.

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

use super::test_helpers::{SubprocessPatchGuard, acquire_subprocess_patch_lock};

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
) -> BootstrapRunPatch {
    let patch_guard = acquire_subprocess_patch_lock();
    let globals = PyDict::new(py);
    globals
        .set_item(
            "_counter",
            pyo3::Py::new(py, BootstrapRunCounter { count }).expect("build run counter"),
        )
        .expect("install run counter");
    // Force the bootstrap to actually run, then count and satisfy it:
    //   * a `meta_path` blocker makes `import msgspec` fail, so
    //     `ensure_msgspec_installed` cannot short-circuit and must enter the
    //     `Once`-guarded installer (even if msgspec is installed on disk);
    //   * the mocked `subprocess.run` counts each call and registers an
    //     importable stub `msgspec` module, so the installer's final
    //     `import msgspec` resolves without network access.
    // The counter therefore proves the installer path executed, and the
    // assertions fail if the bootstrap is ever skipped. The returned patch
    // guard serialises subprocess mutation until the caller restores it.
    let patch = CString::new(
        r#"
import subprocess
import sys
import types


class _BlockMsgspecBootstrap:
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "msgspec" or fullname.startswith("msgspec."):
            raise ModuleNotFoundError("msgspec blocked for bootstrap test", name="msgspec")
        return None


_bootstrap_msgspec_blocker = _BlockMsgspecBootstrap()
sys.meta_path.insert(0, _bootstrap_msgspec_blocker)

_original_run = subprocess.run
_calls = []


def _mock_run(*args, **kwargs):
    _calls.append((args, kwargs))
    _counter(args, kwargs)
    sys.modules.setdefault("msgspec", types.ModuleType("msgspec"))


subprocess.run = _mock_run
"#,
    )
    .expect("CString build");
    py.run(patch.as_c_str(), Some(&globals), None)
        .expect("monkeypatch subprocess.run");

    BootstrapRunPatch {
        globals: globals.unbind(),
        patch_guard,
    }
}

/// Removes any stubbed `msgspec` module left by a bootstrap mock.
pub(super) fn remove_msgspec_from_modules(py: Python<'_>) {
    let remove =
        CString::new("import sys\nsys.modules.pop('msgspec', None)\n").expect("CString build");
    py.run(remove.as_c_str(), None, None)
        .expect("remove msgspec from sys.modules");
}

/// Restores the Python import hook that forces the bootstrap path to execute.
pub(super) fn restore_msgspec_blocker(py: Python<'_>, globals: &Bound<'_, PyDict>) {
    let restore = CString::new(
        r"
import sys

try:
    sys.meta_path.remove(_bootstrap_msgspec_blocker)
except (ValueError, NameError):
    pass
",
    )
    .expect("CString build");
    py.run(restore.as_c_str(), Some(globals), None)
        .expect("restore msgspec bootstrap blocker");
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
