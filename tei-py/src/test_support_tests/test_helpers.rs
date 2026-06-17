//! Fixtures for unit tests around low-level Python call helpers.

use pyo3::{
    Bound, Python,
    types::{PyAny, PyAnyMethods, PyDict},
};
use std::ffi::CString;

pub(super) struct RunAndKwargs<'py> {
    pub(super) run: Bound<'py, PyAny>,
    pub(super) kwargs: Bound<'py, PyDict>,
    pub(super) globals: Bound<'py, PyDict>,
}

#[derive(Clone, Copy)]
pub(super) enum RunWithKwargsArgShape {
    Unit,
    NestedPyTuple,
    DirectPyTuple,
}

pub(super) fn setup_run_and_kwargs(py: Python<'_>) -> RunAndKwargs<'_> {
    let globals = PyDict::new(py);
    let patch = CString::new(
        "import subprocess\n\
         _original_run = subprocess.run\n\
         _calls = []\n\
         subprocess.run = lambda *a, **kw: _calls.append((a, kw))\n",
    )
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

pub(super) fn restore_subprocess_run(py: Python<'_>, globals: &Bound<'_, PyDict>) {
    // Restore `subprocess.run` and remove the bootstrap `meta_path` blocker
    // if one was installed. The blocker removal is a no-op for callers that
    // never installed it (e.g. the `run_with_kwargs` tests).
    let restore = CString::new(
        r"
import subprocess
import sys

subprocess.run = _original_run
_calls = []

try:
    sys.meta_path.remove(_bootstrap_msgspec_blocker)
except (ValueError, NameError):
    pass
",
    )
    .expect("CString build");
    py.run(restore.as_c_str(), Some(globals), None).ok();
}

pub(super) struct SubprocessRestoreGuard<'py> {
    pub(super) py: Python<'py>,
    pub(super) globals: Bound<'py, PyDict>,
}

impl Drop for SubprocessRestoreGuard<'_> {
    fn drop(&mut self) {
        restore_subprocess_run(self.py, &self.globals);
    }
}

pub(super) struct ShutilRestoreGuard<'py> {
    pub(super) py: Python<'py>,
    pub(super) globals: Bound<'py, PyDict>,
}

impl Drop for ShutilRestoreGuard<'_> {
    fn drop(&mut self) {
        super::restore_shutil_which(self.py, &self.globals);
    }
}
