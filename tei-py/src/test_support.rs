//! Test-only helpers shared across Rust unit tests and Python BDD suites.
//! They use `PyO3`'s embedding API (`pyo3::sync::OnceExt`,
//! `pyo3::call::PyCallArgs`, and `Bound<PyAny>`) with the supported `PyO3`
//! `0.28.x` minor series to interact with an embedded Python interpreter.
//! Their primary job is bootstrapping `msgspec>=0.19,<0.20` with `uv` or `pip`
//! via `subprocess.run` so Rust and Python BDD tests can import it.
//! [`ensure_msgspec_installed`] and [`msgspec_available`] are public exports.
//! `run_with_kwargs` is a hidden-public helper for compile-fail UI tests, while
//! `install_msgspec` and `has_uv` are private details.
//! The bootstrap is serialized with `Once` via `OnceExt::call_once_py_attached`
//! to prevent races when tests run in parallel.
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

use pyo3::{
    Bound, PyResult, Python,
    sync::OnceExt,
    types::{PyAny, PyAnyMethods, PyDict, PyTuple},
};
use std::sync::Once;

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
static MSGSPEC_INIT: Once = Once::new();

/// Ensures `msgspec` is importable by the embedded Python interpreter.
///
/// A `Once` guarded by `OnceExt::call_once_py_attached` serialises the
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
    if py.import("msgspec").is_ok() {
        return Ok(());
    }

    MSGSPEC_INIT.call_once_py_attached(py, || do_bootstrap(py));

    py.import("msgspec")?;
    Ok(())
}

/// Reports whether `msgspec` is available to the embedded interpreter.
///
/// The helper calls [`ensure_msgspec_installed`] behind the GIL and returns
/// `true` only when importing succeeds after the best-effort bootstrap.
#[must_use]
pub fn msgspec_available() -> bool {
    Python::attach(|py| ensure_msgspec_installed(py).is_ok())
}

#[cfg(test)]
#[path = "test_support_tests/mod.rs"]
mod tests;
