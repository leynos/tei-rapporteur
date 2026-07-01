//! Test-only helpers shared across Rust unit tests and Python BDD suites.
//!
//! They use `PyO3`'s embedding API with the supported `PyO3` `0.28.x`
//! minor series to interact with an embedded Python interpreter. Their
//! primary job is bootstrapping `msgspec>=0.19,<0.20` with `uv` or `pip`
//! so Rust and Python BDD tests can import it.

mod bootstrap;

pub use bootstrap::{
    RunWithKwargsArgs, bootstrap_msgspec, bootstrap_msgspec_attached, run_with_kwargs,
};

#[cfg(any(test, feature = "test-support"))]
use std::sync::TryLockError;
#[cfg(any(test, feature = "test-support"))]
use std::sync::{Mutex, MutexGuard};

#[cfg(any(test, feature = "test-support"))]
static PYTHON_IMPORT_STATE_LOCK: Mutex<()> = Mutex::new(());
#[cfg(any(test, feature = "test-support"))]
static PYTHON_MODULE_REGISTRATION_LOCK: Mutex<()> = Mutex::new(());

/// Returns an RAII guard that serializes all operations touching the embedded
/// Python interpreter's import state (i.e. `sys.modules` or `sys.meta_path`).
///
/// Prefer `with_python` over calling this directly. Use this only when you need
/// the guard to outlive a single `Python::attach` block, e.g. when testing the
/// lock behaviour itself.
#[cfg(any(test, feature = "test-support"))]
pub(super) fn python_import_state_lock() -> MutexGuard<'static, ()> {
    PYTHON_IMPORT_STATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(any(test, feature = "test-support"))]
fn lock_attached_for_tests(
    py: pyo3::Python<'_>,
    mutex: &'static Mutex<()>,
) -> MutexGuard<'static, ()> {
    loop {
        match mutex.try_lock() {
            Ok(guard) => return guard,
            Err(TryLockError::Poisoned(poisoned)) => return poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => {
                py.detach(|| std::thread::sleep(std::time::Duration::from_millis(1)));
            }
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn lock_python_module_registration_attached_for_tests(
    py: pyo3::Python<'_>,
) -> MutexGuard<'static, ()> {
    lock_attached_for_tests(py, &PYTHON_MODULE_REGISTRATION_LOCK)
}

/// Attaches to the embedded Python interpreter while holding the process-wide
/// Python import-state lock.
///
/// Use this in place of `Python::attach` in test code. The lock prevents
/// concurrent tests from racing on `sys.modules` or `sys.meta_path` mutations,
/// eliminating intermittent `msgspec` import failures.
///
/// # Panics
///
/// Panics if called re-entrantly from within the same thread, as that would
/// deadlock on the non-reentrant `Mutex`. Do not nest `with_python` calls.
pub fn with_python<F, R>(f: F) -> R
where
    F: for<'py> FnOnce(pyo3::Python<'py>) -> R,
{
    let _guard = python_import_state_lock();
    pyo3::Python::attach(f)
}

#[cfg(test)]
mod tests;
