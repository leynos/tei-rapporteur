//! `subprocess.run` monkeypatch helpers for test-support unit tests.
//!
//! The parent test module asserts bootstrap behaviour; this module owns the
//! Python mocks and RAII guards that keep process-wide subprocess and
//! import-state mutations contained during those assertions.

use super::super::bootstrap::reset_msgspec_bootstrap_for_tests;
use pyo3::{
    Bound, Py, PyResult, Python, pyclass, pymethods,
    types::{PyAny, PyAnyMethods, PyDict, PyTuple},
};
use std::{
    ffi::CString,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub(super) struct RunAndKwargs<'py> {
    pub(super) run: Bound<'py, PyAny>,
    pub(super) kwargs: Bound<'py, PyDict>,
    pub(super) globals: Bound<'py, PyDict>,
}

pub(super) fn setup_run_and_kwargs(py: Python<'_>) -> RunAndKwargs<'_> {
    let globals = PyDict::new(py);
    let patch = CString::new(concat!(
        "import subprocess\n",
        "_original_run = subprocess.run\n",
        "_calls = []\n",
        "subprocess.run = lambda *a, **kw: _calls.append((a, kw))\n",
    ))
    .expect("CString build");
    py.run(patch.as_c_str(), Some(&globals), None)
        .expect("monkeypatch subprocess.run");
    let subprocess = py.import("subprocess").expect("import subprocess");
    let run = subprocess.getattr("run").expect("get subprocess.run");
    let kwargs = PyDict::new(py);

    RunAndKwargs {
        run,
        kwargs,
        globals,
    }
}

fn restore_subprocess_run(py: Python<'_>, globals: &Bound<'_, PyDict>) -> PyResult<()> {
    // Restore `subprocess.run` and remove the bootstrap `meta_path` blocker if
    // one was installed. The blocker removal is a no-op for callers that never
    // installed it (e.g. the `run_with_kwargs` tests).
    let restore = CString::new(
        r#"
import subprocess
import sys
import importlib.metadata

subprocess.run = _original_run
_calls = []

try:
    _original_metadata_version
except NameError:
    pass
else:
    importlib.metadata.version = _original_metadata_version

try:
    sys.meta_path.remove(_bootstrap_msgspec_blocker)
except (ValueError, NameError):
    pass

try:
    _original_msgspec
except NameError:
    pass
else:
    if _original_msgspec is _msgspec_missing:
        sys.modules.pop("msgspec", None)
    else:
        sys.modules["msgspec"] = _original_msgspec
"#,
    )
    .expect("CString build");
    py.run(restore.as_c_str(), Some(globals), None)
}

fn report_restore_failure(py: Python<'_>, error: &pyo3::PyErr) {
    if std::thread::panicking() {
        if let Ok(stderr) = py.import("sys").and_then(|sys| sys.getattr("stderr")) {
            stderr
                .call_method1(
                    "write",
                    (format!(
                        "failed to restore subprocess.run monkeypatch: {error}\n"
                    ),),
                )
                .ok();
        }
    } else {
        panic!("failed to restore subprocess.run monkeypatch: {error}");
    }
}

pub(super) struct SubprocessRestoreGuard<'py> {
    py: Python<'py>,
    globals: Bound<'py, PyDict>,
}

impl<'py> SubprocessRestoreGuard<'py> {
    pub(super) fn new(py: Python<'py>, globals: Bound<'py, PyDict>) -> Self {
        Self { py, globals }
    }
}

impl Drop for SubprocessRestoreGuard<'_> {
    fn drop(&mut self) {
        if let Err(error) = restore_subprocess_run(self.py, &self.globals) {
            report_restore_failure(self.py, &error);
        }
    }
}

pub(super) struct BootstrapRestoreGuard {
    globals: Py<PyDict>,
}

impl BootstrapRestoreGuard {
    pub(super) fn new(globals: Py<PyDict>) -> Self {
        Self { globals }
    }
}

impl Drop for BootstrapRestoreGuard {
    fn drop(&mut self) {
        Python::attach(
            |py| match restore_subprocess_run(py, self.globals.bind(py)) {
                Ok(()) => reset_msgspec_bootstrap_for_tests(),
                Err(error) => report_restore_failure(py, &error),
            },
        );
    }
}

#[pyclass]
struct BootstrapRunCounter {
    count: Arc<AtomicUsize>,
}

#[pymethods]
impl BootstrapRunCounter {
    fn __call__(&self, _args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

pub(super) fn setup_bootstrap_run_counter(py: Python<'_>, count: Arc<AtomicUsize>) -> Py<PyDict> {
    let globals = PyDict::new(py);
    globals
        .set_item(
            "_counter",
            Py::new(py, BootstrapRunCounter { count }).expect("build run counter"),
        )
        .expect("install run counter");
    // Count bootstrap subprocess calls without forcing a process-wide import
    // failure. When `msgspec` is already installed the helper should
    // short-circuit cleanly; when it is absent the mocked installer satisfies
    // the final import without touching the network.
    let patch = CString::new(
        r#"
import subprocess
import sys
import types
import importlib
import importlib.metadata


class _BlockMsgspecBootstrap:
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "msgspec" or fullname.startswith("msgspec."):
            raise ModuleNotFoundError("msgspec blocked for bootstrap test", name="msgspec")
        return None


_bootstrap_msgspec_blocker = _BlockMsgspecBootstrap()
_msgspec_missing = object()
_original_msgspec = sys.modules.get("msgspec", _msgspec_missing)

_original_run = subprocess.run
_original_metadata_version = importlib.metadata.version


def _mock_metadata_version(package_name):
    if package_name == "msgspec":
        return "0.19.99"
    return _original_metadata_version(package_name)


def _mock_run(*args, **kwargs):
    _counter(args, kwargs)
    try:
        sys.meta_path.remove(_bootstrap_msgspec_blocker)
    except ValueError:
        pass
    try:
        sys.modules["msgspec"] = importlib.import_module("msgspec")
    except ModuleNotFoundError:
        sys.modules.setdefault("msgspec", types.ModuleType("msgspec"))
    finally:
        if _bootstrap_msgspec_blocker not in sys.meta_path:
            sys.meta_path.insert(0, _bootstrap_msgspec_blocker)


subprocess.run = _mock_run
importlib.metadata.version = _mock_metadata_version
"#,
    )
    .expect("CString build");
    py.run(patch.as_c_str(), Some(&globals), None)
        .expect("monkeypatch subprocess.run");

    globals.unbind()
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
