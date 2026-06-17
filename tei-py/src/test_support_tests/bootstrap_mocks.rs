//! Mocks for exercising the `msgspec` bootstrap path without network access.

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

#[pyclass]
pub(super) struct BootstrapRunCounter {
    count: Arc<AtomicUsize>,
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
) -> pyo3::Py<PyDict> {
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
    // assertions fail if the bootstrap is ever skipped. Each test runs in
    // its own process under nextest, so the process-wide `Once` is fresh and
    // no cross-test reset seam or mutex is required.
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


def _mock_run(*args, **kwargs):
    _counter(args, kwargs)
    sys.modules.setdefault("msgspec", types.ModuleType("msgspec"))


subprocess.run = _mock_run
"#,
    )
    .expect("CString build");
    py.run(patch.as_c_str(), Some(&globals), None)
        .expect("monkeypatch subprocess.run");

    globals.unbind()
}

pub(super) fn remove_msgspec_from_modules(py: Python<'_>) {
    let remove =
        CString::new("import sys\nsys.modules.pop('msgspec', None)\n").expect("CString build");
    py.run(remove.as_c_str(), None, None)
        .expect("remove msgspec from sys.modules");
}

pub(super) fn recorded_call_count(globals: &Bound<'_, PyDict>) -> usize {
    globals
        .get_item("_calls")
        .expect("read recorded subprocess.run calls")
        .len()
        .expect("count recorded subprocess.run calls")
}

pub(super) fn recorded_args<'py>(globals: &Bound<'py, PyDict>) -> Bound<'py, PyAny> {
    let expr = CString::new("_calls[0][0]").expect("CString build");
    globals
        .py()
        .eval(expr.as_c_str(), Some(globals), None)
        .expect("read recorded subprocess.run positional arguments")
}
