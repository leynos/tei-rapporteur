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
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(super) struct RunAndKwargs<'py> {
    pub(super) run: Bound<'py, PyAny>,
    pub(super) kwargs: Bound<'py, PyDict>,
    pub(super) globals: Bound<'py, PyDict>,
}

/// Installs the recording `subprocess.run` monkeypatch.
///
/// Arrangement can fail, so the caller decides whether a failure is the test
/// verdict.
pub(super) fn setup_run_and_kwargs(py: Python<'_>) -> PyResult<RunAndKwargs<'_>> {
    let globals = PyDict::new(py);
    py.run(
        cr"
import subprocess

_original_run = subprocess.run
_calls = []
subprocess.run = lambda *a, **kw: _calls.append((a, kw))
",
        Some(&globals),
        None,
    )?;
    let subprocess = py.import("subprocess")?;
    let run = subprocess.getattr("run")?;
    let kwargs = PyDict::new(py);

    Ok(RunAndKwargs {
        run,
        kwargs,
        globals,
    })
}

fn restore_subprocess_run(py: Python<'_>, globals: &Bound<'_, PyDict>) -> PyResult<()> {
    // Restore `subprocess.run` and remove the bootstrap `meta_path` blocker if
    // one was installed. The blocker removal is a no-op for callers that never
    // installed it (e.g. the `run_with_kwargs` tests).
    py.run(
        cr#"
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
        Some(globals),
        None,
    )
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

/// Installs a `subprocess.run` mock that counts bootstrap invocations.
///
/// Arrangement can fail, so the caller decides whether a failure is the test
/// verdict.
pub(super) fn setup_bootstrap_run_counter(
    py: Python<'_>,
    count: Arc<AtomicUsize>,
) -> PyResult<Py<PyDict>> {
    let globals = PyDict::new(py);
    globals.set_item("_counter", Py::new(py, BootstrapRunCounter { count })?)?;
    // Count bootstrap subprocess calls without forcing a process-wide import
    // failure. When `msgspec` is already installed the helper should
    // short-circuit cleanly; when it is absent the mocked installer satisfies
    // the final import without touching the network.
    py.run(
        cr#"
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
        Some(&globals),
        None,
    )?;

    Ok(globals.unbind())
}

/// Returns the number of `subprocess.run` calls the mock recorded.
pub(super) fn recorded_call_count(globals: &Bound<'_, PyDict>) -> PyResult<usize> {
    globals.get_item("_calls")?.len()
}

/// Returns the positional arguments of the first recorded `subprocess.run` call.
pub(super) fn recorded_args<'py>(globals: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    globals.py().eval(c"_calls[0][0]", Some(globals), None)
}
