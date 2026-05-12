//! Test-only helpers shared across Rust unit tests and Python BDD suites.
//!
//! The helpers use `PyO3`'s embedding API, including `pyo3::sync::OnceExt`,
//! `pyo3::call::PyCallArgs`, and `Bound<PyAny>`, against the supported `PyO3`
//! `0.24.x` minor series. The primary job is bootstrapping
//! `msgspec>=0.19,<0.20` into the embedded interpreter with `uv` or `pip` via
//! `subprocess.run`, so both Rust and Python BDD tests can import it. Only
//! [`ensure_msgspec_installed`] and [`msgspec_available`] are exported;
//! `run_with_kwargs`, `install_msgspec`, and `has_uv` are private details.
//! The bootstrap is serialised with `Once` via `OnceExt::call_once_py_attached`
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

fn has_uv(py: Python<'_>) -> bool {
    py.import("shutil")
        .ok()
        .and_then(|shutil| shutil.call_method1("which", ("uv",)).ok())
        // `which` returns a path string or `None`; treat only a concrete path as present.
        .and_then(|path| path.extract::<Option<String>>().ok())
        .flatten()
        .is_some()
}

fn run_with_kwargs<'py, A>(run: &Bound<'py, PyAny>, args: A, kwargs: &Bound<'py, PyDict>)
where
    A: pyo3::call::PyCallArgs<'py>,
{
    // Best-effort: subprocess.run may fail (e.g., missing network); the final
    // `py.import("msgspec")?` is the authoritative error path for callers.
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

    MSGSPEC_INIT.call_once_py_attached(py, || {
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

        let kwargs = PyDict::new(py);
        if kwargs.set_item("check", true).is_err() || kwargs.set_item("timeout", 30u64).is_err() {
            return;
        }
        run_with_kwargs(
            &run,
            ((executable.clone(), "-m", "ensurepip", "--upgrade"),),
            &kwargs,
        );

        let install_kwargs = PyDict::new(py);
        if install_kwargs.set_item("check", true).is_err()
            || install_kwargs.set_item("timeout", 30u64).is_err()
        {
            return;
        }

        install_msgspec(&run, &executable, &install_kwargs, has_uv(py));
    });

    py.import("msgspec")?;
    Ok(())
}

/// Reports whether `msgspec` is available to the embedded interpreter.
///
/// The helper calls [`ensure_msgspec_installed`] behind the GIL and returns
/// `true` only when importing succeeds after the best-effort bootstrap.
#[must_use]
pub fn msgspec_available() -> bool {
    Python::with_gil(|py| ensure_msgspec_installed(py).is_ok())
}

#[cfg(test)]
mod tests {
    //! Unit tests for Python-side test support helpers.

    use super::*;
    use std::ffi::CString;

    #[test]
    fn run_with_kwargs_accepts_unit_tuple() {
        Python::with_gil(|py| {
            let subprocess = py.import("subprocess").expect("import subprocess");
            let run = subprocess.getattr("run").expect("get subprocess.run");
            let kwargs = PyDict::new(py);

            run_with_kwargs(&run, (), &kwargs);
        });
    }

    #[test]
    fn run_with_kwargs_accepts_one_tuple_of_pybytes() {
        Python::with_gil(|py| {
            let subprocess = py.import("subprocess").expect("import subprocess");
            let run = subprocess.getattr("run").expect("get subprocess.run");
            let kwargs = PyDict::new(py);
            let args_tuple = PyTuple::new(py, ["true"]).expect("build argument tuple");

            run_with_kwargs(&run, (args_tuple,), &kwargs);
        });
    }

    #[test]
    fn run_with_kwargs_accepts_bound_pytuple() {
        Python::with_gil(|py| {
            let subprocess = py.import("subprocess").expect("import subprocess");
            let run = subprocess.getattr("run").expect("get subprocess.run");
            let kwargs = PyDict::new(py);
            let args_tuple = PyTuple::new(py, [["true"]]).expect("build subprocess args");

            run_with_kwargs(&run, args_tuple, &kwargs);
        });
    }

    #[test]
    fn has_uv_reports_absence_when_which_returns_none() {
        Python::with_gil(|py| {
            let code = CString::new(
                "import shutil\norig = shutil.which\nshutil.which = lambda name: None\n",
            )
            .expect("CString build");
            py.run(code.as_c_str(), None, None)
                .expect("monkeypatch shutil.which");

            assert!(!has_uv(py), "uv should be absent when which returns None");

            let restore =
                CString::new("import shutil\nshutil.which = orig\n").expect("CString build");
            py.run(restore.as_c_str(), None, None)
                .expect("restore shutil.which");
        });
    }

    #[test]
    fn has_uv_reports_presence_when_which_returns_path() {
        Python::with_gil(|py| {
            let code = CString::new(
                "import shutil\norig = shutil.which\nshutil.which = lambda name: '/usr/bin/uv'\n",
            )
            .expect("CString build");
            py.run(code.as_c_str(), None, None)
                .expect("monkeypatch shutil.which");

            assert!(
                has_uv(py),
                "uv should be detected when which returns a path"
            );

            let restore =
                CString::new("import shutil\nshutil.which = orig\n").expect("CString build");
            py.run(restore.as_c_str(), None, None)
                .expect("restore shutil.which");
        });
    }
}
