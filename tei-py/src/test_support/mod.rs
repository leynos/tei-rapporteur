//! Test-only helpers shared across Rust unit tests and Python BDD suites.
//!
//! They use `PyO3`'s embedding API with the supported `PyO3` `0.28.x`
//! minor series to interact with an embedded Python interpreter. Their
//! primary job is bootstrapping `msgspec>=0.19,<0.20` with `uv` or `pip`
//! so Rust and Python BDD tests can import it.

mod bootstrap;

pub use bootstrap::{RunWithKwargsArgs, ensure_msgspec_available, run_with_kwargs};
pub(super) use bootstrap::ensure_msgspec_installed;

#[cfg(feature = "test-support")]
use std::sync::{Mutex, MutexGuard};

#[cfg(feature = "test-support")]
static PYTHON_IMPORT_STATE_LOCK: Mutex<()> = Mutex::new(());

/// Returns an RAII guard that serializes all operations touching the embedded
/// Python interpreter's import state (i.e. `sys.modules` or `sys.meta_path`).
///
/// Prefer `with_python` over calling this directly. Use this only when you need
/// the guard to outlive a single `Python::attach` block, e.g. when testing the
/// lock behaviour itself.
#[cfg(feature = "test-support")]
pub(super) fn python_import_state_lock() -> MutexGuard<'static, ()> {
    PYTHON_IMPORT_STATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn with_python<F, R>(f: F) -> R
where
    F: for<'py> FnOnce(pyo3::Python<'py>) -> R,
{
    let _guard = python_import_state_lock();
    pyo3::Python::attach(f)
}
mod tests;
