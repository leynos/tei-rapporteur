//! `shutil.which` monkeypatch restore helpers for test-support unit tests.
//!
//! The parent test module owns the assertions for `has_uv`; this module keeps
//! the Python teardown guard colocated with the specific process-wide monkeypatch
//! it restores.

use pyo3::{Bound, Python, types::PyDict};
use std::ffi::CString;

fn restore_shutil_which(py: Python<'_>, globals: &Bound<'_, PyDict>) {
    let restore = CString::new("import shutil\nshutil.which = orig\n").expect("CString build");
    py.run(restore.as_c_str(), Some(globals), None).ok();
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
