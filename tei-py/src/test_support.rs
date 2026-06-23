//! Test-only helpers shared across Rust unit tests and Python BDD suites.
//! They bootstrap `msgspec>=0.19,<0.20` through `PyO3`'s embedded interpreter so
//! tests can import it consistently. [`ensure_msgspec_installed`],
//! [`try_ensure_msgspec_installed`], and [`msgspec_available`] are public
//! exports; `run_with_kwargs` is hidden-public for compile-fail UI tests.
//! Bootstrap execution is serialized with `OnceExt::call_once_py_attached`.
//! Child `test_support_tests` modules provide subprocess mocks, Python-state
//! restoration guards, and property coverage for bootstrap invariants.
const MSGSPEC_REQUIREMENT: &str = "msgspec>=0.19,<0.20";
const PIP_COMMON_FLAGS: [&str; 6] = [
    "--no-input",
    "--disable-pip-version-check",
    "--default-timeout",
    "15",
    "--retries",
    "1",
];
const UV_COMMON_FLAGS: [&str; 1] = ["--quiet"];

#[cfg(not(test))]
use pyo3::sync::OnceExt;
use pyo3::{
    Bound, PyResult, Python,
    exceptions::PyRuntimeError,
    types::{PyAny, PyAnyMethods, PyDict, PyTuple},
};
#[cfg(not(test))]
use std::sync::Once;
#[cfg(test)]
use std::sync::{
    Mutex, MutexGuard, TryLockError,
    atomic::{AtomicBool, Ordering as AtomicOrdering},
};

/// Crate-owned wrapper for Python call argument diagnostics.
///
/// This trait intentionally has no reuse strategy beyond `run_with_kwargs`.
/// Its purpose is to anchor the compile-fail contract at a `tei-py` symbol so
/// the committed trybuild snapshot is driven by this crate's
/// `#[diagnostic::on_unimplemented]` text, not by PyO3's `PyCallArgs`
/// diagnostic, which may change across PyO3 minor releases.
///
/// The notes below are the source of the expected UI-test output. Keep them in
/// sync with `tei-py/tests/ui/non_pycallargs_rejected.stderr`.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a Python `call` argument",
    note = "`PyCallArgs` is implemented for Rust tuples, `Bound<'py, PyTuple>` and `Py<PyTuple>`",
    note = "if your type is convertible to `PyTuple` via `IntoPyObject`, call `<arg>.into_pyobject(py)` manually",
    note = "if you meant to pass the type as a single argument, wrap it in a 1-tuple, `(<arg>,)`"
)]
pub trait RunWithKwargsArgs<'py>: pyo3::call::PyCallArgs<'py> {}

impl<'py, A> RunWithKwargsArgs<'py> for A where A: pyo3::call::PyCallArgs<'py> {}

fn has_uv(py: Python<'_>) -> bool {
    py.import("shutil")
        .ok()
        .and_then(|shutil| shutil.call_method1("which", ("uv",)).ok())
        // `which` returns a path string or `None`; treat only a concrete path as present.
        .and_then(|path| path.extract::<Option<String>>().ok())
        .flatten()
        .is_some()
}

/// Calls a Python callable with positional arguments and keyword arguments.
///
/// This helper intentionally discards any error returned by the Python call.
/// It is used only for best-effort setup paths where a later import check is
/// the authoritative failure signal.
#[doc(hidden)]
pub fn run_with_kwargs<'py, A>(run: &Bound<'py, PyAny>, args: A, kwargs: &Bound<'py, PyDict>)
where
    A: RunWithKwargsArgs<'py>,
{
    run.call(args, Some(kwargs)).ok();
}

fn install_msgspec<'py>(
    run: &Bound<'py, PyAny>,
    executable: &Bound<'py, PyAny>,
    kwargs: &Bound<'py, PyDict>,
    use_uv: bool,
) {
    if use_uv {
        let mut args = vec!["uv", "pip", "install"];
        args.extend_from_slice(&UV_COMMON_FLAGS);
        args.push(MSGSPEC_REQUIREMENT);
        if let Ok(args_tuple) = PyTuple::new(run.py(), args) {
            run_with_kwargs(run, (args_tuple,), kwargs);
        }
    } else {
        run_with_kwargs(
            run,
            ((
                executable.clone(),
                "-m",
                "pip",
                "install",
                PIP_COMMON_FLAGS[0],
                PIP_COMMON_FLAGS[1],
                PIP_COMMON_FLAGS[2],
                PIP_COMMON_FLAGS[3],
                PIP_COMMON_FLAGS[4],
                PIP_COMMON_FLAGS[5],
                "--break-system-packages",
                MSGSPEC_REQUIREMENT,
            ),),
            kwargs,
        );
    }
}

fn make_subprocess_kwargs(py: Python<'_>) -> Option<Bound<'_, PyDict>> {
    let kwargs = PyDict::new(py);
    kwargs.set_item("check", true).ok()?;
    kwargs.set_item("timeout", 30u64).ok()?;
    Some(kwargs)
}

fn do_bootstrap(py: Python<'_>) {
    let Some(subprocess) = py.import("subprocess").ok() else {
        return;
    };
    let Some(sys) = py.import("sys").ok() else {
        return;
    };
    let Some(executable) = sys.getattr("executable").ok() else {
        return;
    };
    let Ok(run) = subprocess.getattr("run") else {
        return;
    };
    let Some(kwargs) = make_subprocess_kwargs(py) else {
        return;
    };
    run_with_kwargs(
        &run,
        ((executable.clone(), "-m", "ensurepip", "--upgrade"),),
        &kwargs,
    );
    let Some(install_kwargs) = make_subprocess_kwargs(py) else {
        return;
    };
    install_msgspec(&run, &executable, &install_kwargs, has_uv(py));
}

fn msgspec_satisfies_requirement(py: Python<'_>) -> PyResult<bool> {
    py.import("msgspec")?;
    let metadata = py.import("importlib.metadata")?;
    let version: String = metadata.call_method1("version", ("msgspec",))?.extract()?;
    Ok(msgspec_version_satisfies_requirement(&version))
}

fn msgspec_version_satisfies_requirement(version: &str) -> bool {
    let mut parts = version.split('.').map(|value| {
        value
            .chars()
            .take_while(std::primitive::char::is_ascii_digit)
            .collect::<String>()
            .parse::<u64>()
            .ok()
    });
    parts.next() == Some(Some(0)) && parts.next() == Some(Some(19))
}

#[cfg(not(test))]
static MSGSPEC_INIT: Once = Once::new();

#[cfg(test)]
struct ResettableMsgspecInit {
    state: Mutex<MsgspecInitState>,
}

#[cfg(test)]
struct RunningMsgspecInitGuard<'a> {
    init: &'a ResettableMsgspecInit,
    is_complete: bool,
}

#[cfg(test)]
#[derive(Clone, Copy, Eq, PartialEq)]
enum MsgspecInitState {
    Incomplete,
    Running,
    Complete,
}

#[cfg(test)]
impl ResettableMsgspecInit {
    const fn new() -> Self {
        Self {
            state: Mutex::new(MsgspecInitState::Incomplete),
        }
    }

    fn call_once_py_attached<F>(&self, py: Python<'_>, bootstrap: F)
    where
        F: FnOnce(),
    {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while *state == MsgspecInitState::Running {
            drop(state);
            py.detach(|| std::thread::sleep(std::time::Duration::from_millis(1)));
            state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        if *state == MsgspecInitState::Complete {
            return;
        }
        *state = MsgspecInitState::Running;
        drop(state);

        let mut running_guard = RunningMsgspecInitGuard {
            init: self,
            is_complete: false,
        };
        bootstrap();
        self.set_state(MsgspecInitState::Complete);
        running_guard.is_complete = true;
    }

    fn reset(&self) {
        self.set_state(MsgspecInitState::Incomplete);
    }

    fn set_state(&self, next_state: MsgspecInitState) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *state = next_state;
    }
}

#[cfg(test)]
impl Drop for RunningMsgspecInitGuard<'_> {
    fn drop(&mut self) {
        if !self.is_complete {
            self.init.set_state(MsgspecInitState::Incomplete);
        }
    }
}

#[cfg(test)]
static MSGSPEC_INIT: ResettableMsgspecInit = ResettableMsgspecInit::new();

#[cfg(test)]
static MSGSPEC_BOOTSTRAP_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
static FORCE_MSGSPEC_BOOTSTRAP: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static PYTHON_MODULE_REGISTRATION_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub(super) struct ForcedMsgspecBootstrapGuard;

#[cfg(test)]
impl Drop for ForcedMsgspecBootstrapGuard {
    fn drop(&mut self) {
        FORCE_MSGSPEC_BOOTSTRAP.store(false, AtomicOrdering::SeqCst);
    }
}

#[cfg(test)]
pub(super) fn force_msgspec_bootstrap_for_tests() -> ForcedMsgspecBootstrapGuard {
    FORCE_MSGSPEC_BOOTSTRAP.store(true, AtomicOrdering::SeqCst);
    ForcedMsgspecBootstrapGuard
}

#[cfg(test)]
fn lock_msgspec_bootstrap_for_tests() -> MutexGuard<'static, ()> {
    MSGSPEC_BOOTSTRAP_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
fn lock_msgspec_bootstrap_attached_for_tests(py: Python<'_>) -> MutexGuard<'static, ()> {
    lock_attached_for_tests(py, &MSGSPEC_BOOTSTRAP_TEST_LOCK)
}

#[cfg(test)]
fn lock_attached_for_tests(py: Python<'_>, mutex: &'static Mutex<()>) -> MutexGuard<'static, ()> {
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

#[cfg(test)]
pub(super) fn acquire_msgspec_bootstrap_lock_for_tests() -> MutexGuard<'static, ()> {
    lock_msgspec_bootstrap_for_tests()
}

#[cfg(test)]
pub(crate) fn acquire_python_module_registration_lock_for_tests() -> MutexGuard<'static, ()> {
    PYTHON_MODULE_REGISTRATION_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
pub(crate) fn lock_python_module_registration_attached_for_tests(
    py: Python<'_>,
) -> MutexGuard<'static, ()> {
    lock_attached_for_tests(py, &PYTHON_MODULE_REGISTRATION_TEST_LOCK)
}

#[cfg(test)]
pub(super) fn reset_msgspec_init_for_tests() {
    MSGSPEC_INIT.reset();
}

/// Ensures `msgspec` is importable by the embedded Python interpreter.
///
/// A `Once` guarded by `OnceExt::call_once_py_attached` serializes the
/// bootstrap so only one thread runs the installer, avoiding the race
/// reported in CI while detaching from Python when blocked.
///
/// The helper bootstraps `pip` via `ensurepip` when necessary and performs a
/// best-effort installation of `msgspec>=0.19,<0.20`. It is thread-safe:
/// install attempts run at most once even when tests execute in parallel. It
/// returns an error only when importing `msgspec` still fails after the
/// attempted install.
///
/// # Errors
///
/// Returns a `PyErr` when importing or installing `msgspec` fails, for example
/// when `pip` is unavailable in the embedded interpreter.
pub fn ensure_msgspec_installed(py: Python<'_>) -> PyResult<()> {
    #[cfg(test)]
    {
        let _guard = lock_msgspec_bootstrap_attached_for_tests(py);
        ensure_msgspec_installed_inner(py)
    }

    #[cfg(not(test))]
    ensure_msgspec_installed_inner(py)
}

fn ensure_msgspec_installed_inner(py: Python<'_>) -> PyResult<()> {
    #[cfg(test)]
    let should_force_bootstrap = FORCE_MSGSPEC_BOOTSTRAP.load(AtomicOrdering::SeqCst);
    #[cfg(not(test))]
    let should_force_bootstrap = false;

    if !should_force_bootstrap && msgspec_satisfies_requirement(py).unwrap_or(false) {
        return Ok(());
    }

    MSGSPEC_INIT.call_once_py_attached(py, || do_bootstrap(py));

    if msgspec_satisfies_requirement(py)? {
        Ok(())
    } else {
        Err(PyRuntimeError::new_err(format!(
            "installed msgspec does not satisfy {MSGSPEC_REQUIREMENT}"
        )))
    }
}

#[cfg(test)]
pub(super) fn ensure_msgspec_installed_unlocked_for_tests(py: Python<'_>) -> PyResult<()> {
    ensure_msgspec_installed_inner(py)
}

/// Attempts to make `msgspec` available to the embedded interpreter.
#[must_use]
pub fn try_ensure_msgspec_installed() -> bool {
    Python::attach(|py| ensure_msgspec_installed(py).is_ok())
}

/// Reports whether the required `msgspec` version is already available.
///
/// This query helper does not run the bootstrap installer.
#[must_use]
pub fn msgspec_available() -> bool {
    Python::attach(|py| msgspec_satisfies_requirement(py).unwrap_or(false))
}

#[cfg(test)]
#[path = "test_support_tests/mod.rs"]
mod tests;
