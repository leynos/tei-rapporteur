//! Fixtures for unit tests around low-level Python call helpers.
//!
//! This module provides shared setup and restoration guards for tests in
//! `mod`, `properties`, and `bootstrap_mocks`. Its subprocess lock serialises
//! Python global-state mutation while still allowing unrelated tests to run
//! normally.

use pyo3::{
    Bound, Py, PyResult, Python,
    types::{PyAny, PyAnyMethods, PyDict},
};
use std::{
    ffi::CString,
    sync::{Mutex, MutexGuard},
};

static SUBPROCESS_PATCH_LOCK: Mutex<()> = Mutex::new(());

/// Python `subprocess.run` mock plus keyword arguments for call-helper tests.
pub(super) struct RunAndKwargs<'py> {
    pub(super) run: Bound<'py, PyAny>,
    pub(super) kwargs: Bound<'py, PyDict>,
    pub(super) globals: Bound<'py, PyDict>,
    pub(super) patch_guard: SubprocessPatchGuard,
}

/// Shapes accepted by the private `run_with_kwargs` helper.
#[derive(Clone, Copy)]
pub(super) enum RunWithKwargsArgShape {
    Unit,
    NestedPyTuple,
    DirectPyTuple,
}

/// Lock guard held for the full lifetime of a subprocess monkeypatch.
pub(super) struct SubprocessPatchGuard {
    _guard: MutexGuard<'static, ()>,
}

/// Acquires the subprocess monkeypatch lock.
pub(super) fn acquire_subprocess_patch_lock() -> SubprocessPatchGuard {
    SubprocessPatchGuard {
        _guard: SUBPROCESS_PATCH_LOCK
            .lock()
            .expect("lock subprocess patch mutex"),
    }
}

/// Installs a recording `subprocess.run` mock for call-helper tests.
pub(super) fn setup_run_and_kwargs(py: Python<'_>) -> RunAndKwargs<'_> {
    let patch_guard = acquire_subprocess_patch_lock();
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
        patch_guard,
    }
}

/// Restores Python subprocess state for a previously installed mock.
pub(super) fn restore_subprocess_run(py: Python<'_>, globals: &Bound<'_, PyDict>) -> PyResult<()> {
    super::bootstrap_mocks::restore_msgspec_blocker(py, globals);
    let restore = CString::new(
        r"
import subprocess

subprocess.run = _original_run
_calls = []
",
    )
    .expect("CString build");
    py.run(restore.as_c_str(), Some(globals), None)
}

/// Restores subprocess state for tests that remain inside one Python scope.
pub(super) struct SubprocessRestoreGuard<'py> {
    pub(super) py: Python<'py>,
    pub(super) globals: Bound<'py, PyDict>,
    pub(super) _patch_guard: SubprocessPatchGuard,
}

impl Drop for SubprocessRestoreGuard<'_> {
    fn drop(&mut self) {
        restore_subprocess_run(self.py, &self.globals).expect("restore subprocess.run");
    }
}

/// Restores subprocess state for property tests that leave the setup scope.
pub(super) struct OwnedSubprocessRestoreGuard {
    pub(super) globals: Py<PyDict>,
    pub(super) _patch_guard: SubprocessPatchGuard,
}

impl Drop for OwnedSubprocessRestoreGuard {
    fn drop(&mut self) {
        Python::attach(|py| {
            restore_subprocess_run(py, self.globals.bind(py)).expect("restore subprocess.run");
        });
    }
}

/// Restores `shutil.which` after tests that mock `uv` discovery.
pub(super) struct ShutilRestoreGuard<'py> {
    pub(super) py: Python<'py>,
    pub(super) globals: Bound<'py, PyDict>,
}

impl Drop for ShutilRestoreGuard<'_> {
    fn drop(&mut self) {
        super::restore_shutil_which(self.py, &self.globals);
    }
}
