//! `msgspec` bootstrap helpers for embedded Python tests.
//!
//! This private module backs the parent `test_support` API used by Rust unit
//! tests and Python BDD integration tests. It centralizes the embedded
//! interpreter bootstrap path so callers can ask the parent module to make
//! `msgspec` available without duplicating subprocess, version, or import-state
//! checks in individual tests.

#[cfg(not(test))]
use pyo3::sync::OnceExt;
use pyo3::{
    Bound, PyResult, Python,
    exceptions::PyImportError,
    types::{PyAny, PyAnyMethods, PyDict, PyTuple},
};
#[cfg(test)]
use std::sync::Mutex;
#[cfg(not(test))]
use std::sync::Once;

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
static MSGSPEC_INIT: Once = Once::new();
#[cfg(test)]
static MSGSPEC_INIT: Mutex<bool> = Mutex::new(false);

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

pub(super) fn has_uv(py: Python<'_>) -> bool {
    py.import("shutil")
        .ok()
        .and_then(|shutil| shutil.call_method1("which", ("uv",)).ok())
        // `which` returns a path string or `None`; treat only a concrete path as present.
        .and_then(|path| path.extract::<Option<String>>().ok())
        .flatten()
        .is_some()
}

#[doc(hidden)]
pub fn run_with_kwargs<'py, A>(
    run: &Bound<'py, PyAny>,
    args: A,
    kwargs: &Bound<'py, PyDict>,
) -> bool
where
    A: RunWithKwargsArgs<'py>,
{
    // Best-effort: subprocess.run may fail (e.g., missing network); the final
    // `py.import("msgspec")?` is the authoritative error path for callers.
    run.call(args, Some(kwargs)).is_ok()
}

pub(super) fn install_msgspec<'py>(
    run: &Bound<'py, PyAny>,
    executable: &Bound<'py, PyAny>,
    kwargs: &Bound<'py, PyDict>,
    use_uv: bool,
) {
    if use_uv && let Ok(executable_path) = executable.extract::<String>() {
        let mut args = vec![
            "uv".to_owned(),
            "pip".to_owned(),
            "install".to_owned(),
            "--python".to_owned(),
            executable_path,
        ];
        args.extend(UV_COMMON_FLAGS.iter().map(ToString::to_string));
        args.push(MSGSPEC_REQUIREMENT.to_owned());
        if let Ok(args_tuple) = PyTuple::new(run.py(), args)
            && run_with_kwargs(run, (args_tuple,), kwargs)
        {
            return;
        }
    }

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

/// Constructs a `subprocess.run` keyword-argument dict with `check=True`
/// and `timeout=30`. Returns `None` if either key cannot be set.
fn make_subprocess_kwargs(py: Python<'_>) -> Option<Bound<'_, PyDict>> {
    let kwargs = PyDict::new(py);
    kwargs.set_item("check", true).ok()?;
    kwargs.set_item("timeout", 30u64).ok()?;
    Some(kwargs)
}

/// Performs a best-effort `msgspec` installation inside an already-held GIL
/// token. Returns `None` on any setup failure; callers treat absence of
/// `msgspec` as a soft skip rather than a hard error.
fn try_install_msgspec(py: Python<'_>) -> Option<()> {
    let subprocess = py.import("subprocess").ok()?;
    let sys = py.import("sys").ok()?;
    let executable = sys.getattr("executable").ok()?;
    let run = subprocess.getattr("run").ok()?;

    let kwargs = make_subprocess_kwargs(py)?;
    run_with_kwargs(
        &run,
        ((executable.clone(), "-m", "ensurepip", "--upgrade"),),
        &kwargs,
    );

    let install_kwargs = make_subprocess_kwargs(py)?;
    install_msgspec(&run, &executable, &install_kwargs, has_uv(py));
    Some(())
}

fn msgspec_satisfies_requirement(py: Python<'_>) -> bool {
    let Ok(metadata) = py.import("importlib.metadata") else {
        return false;
    };
    metadata
        .call_method1("version", ("msgspec",))
        .and_then(|version| version.extract::<String>())
        .is_ok_and(|version| msgspec_version_satisfies_requirement(&version))
        && py.import("msgspec").is_ok()
}

pub(super) fn msgspec_version_satisfies_requirement(version: &str) -> bool {
    let Some((release, suffix)) = pep440_release_segments(version) else {
        return false;
    };
    if !is_allowed_pep440_suffix(suffix) {
        return false;
    }
    compare_release(&release, &[0, 19]).is_ge() && compare_release(&release, &[0, 20]).is_lt()
}

fn pep440_release_segments(version: &str) -> Option<(Vec<u64>, &str)> {
    let stripped = version.trim().trim_start_matches(['v', 'V']);
    let normalized = stripped
        .split_once('!')
        .map_or(stripped, |(_, release)| release);
    let release_end = normalized
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .unwrap_or(normalized.len());
    let (release_text, suffix) = normalized.split_at(release_end);
    let release = release_text.trim_matches('.');
    let parts: Option<Vec<_>> = release
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect();
    parts
        .filter(|segments| !segments.is_empty())
        .map(|segments| (segments, suffix))
}

fn is_allowed_pep440_suffix(suffix: &str) -> bool {
    suffix.is_empty()
        || suffix.starts_with('+')
        || suffix.starts_with(".post")
        || suffix.starts_with("post")
}

fn compare_release(actual: &[u64], expected: &[u64]) -> std::cmp::Ordering {
    let len = actual.len().max(expected.len());
    for index in 0..len {
        let actual_part = actual.get(index).copied().unwrap_or_default();
        let expected_part = expected.get(index).copied().unwrap_or_default();
        match actual_part.cmp(&expected_part) {
            std::cmp::Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(not(test))]
fn bootstrap_msgspec_once(py: Python<'_>) {
    MSGSPEC_INIT.call_once_py_attached(py, || {
        try_install_msgspec(py);
    });
}

#[cfg(test)]
fn bootstrap_msgspec_once(py: Python<'_>) {
    let mut initialized = MSGSPEC_INIT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !*initialized {
        try_install_msgspec(py);
        *initialized = true;
    }
}

#[cfg(test)]
pub(crate) fn reset_msgspec_bootstrap_for_tests() {
    let mut initialized = MSGSPEC_INIT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *initialized = false;
}

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
pub(super) fn ensure_msgspec_installed(py: Python<'_>) -> PyResult<()> {
    if msgspec_satisfies_requirement(py) {
        return Ok(());
    }

    bootstrap_msgspec_once(py);

    if msgspec_satisfies_requirement(py) {
        Ok(())
    } else {
        Err(PyImportError::new_err(MSGSPEC_REQUIREMENT))
    }
}

/// Bootstraps `msgspec` for callers that already hold the shared Python lock.
#[must_use]
pub fn bootstrap_msgspec_attached(py: Python<'_>) -> bool {
    ensure_msgspec_installed(py).is_ok()
}

/// Bootstraps `msgspec` for the embedded interpreter when it is not already
/// importable.
///
/// Returns `true` only when importing succeeds after the bootstrap attempt.
/// This helper may run subprocess installers and mutate Python import state,
/// so it attaches to the Python interpreter through the shared import-state
/// lock; callers must not hold the GIL or that lock when calling it.
#[must_use]
pub fn bootstrap_msgspec() -> bool {
    super::with_python(|py| ensure_msgspec_installed(py).is_ok())
}
