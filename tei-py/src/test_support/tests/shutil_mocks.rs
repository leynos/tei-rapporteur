//! `shutil.which` monkeypatch restore helpers for test-support unit tests.
//!
//! The parent test module owns the assertions for `has_uv`; this module keeps
//! the Python teardown guard colocated with the specific process-wide monkeypatch
//! it restores.

use pyo3::{
    Bound, Python,
    types::{PyAnyMethods, PyDict},
};
use std::ffi::CString;

fn restore_shutil_which(py: Python<'_>, globals: &Bound<'_, PyDict>) {
    let restore = CString::new("import shutil\nshutil.which = orig\n").expect("CString build");
    if let Err(error) = py.run(restore.as_c_str(), Some(globals), None)
        && let Ok(stderr) = py.import("sys").and_then(|sys| sys.getattr("stderr"))
    {
        stderr
            .call_method1(
                "write",
                (format!(
                    "failed to restore shutil.which monkeypatch: {error}\n"
                ),),
            )
            .ok();
    }
}

pub(super) struct ShutilRestoreGuard<'py> {
    py: Python<'py>,
    globals: Bound<'py, PyDict>,
}

impl<'py> ShutilRestoreGuard<'py> {
    pub(super) fn new(py: Python<'py>, globals: Bound<'py, PyDict>) -> Self {
        Self { py, globals }
    }
}

impl Drop for ShutilRestoreGuard<'_> {
    fn drop(&mut self) {
        restore_shutil_which(self.py, &self.globals);
    }
}
