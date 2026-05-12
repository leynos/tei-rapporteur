//! Test-only helpers shared across Rust unit tests and Python BDD suites.
//! They use `PyO3`'s embedding API (`pyo3::sync::OnceExt`,
//! `pyo3::call::PyCallArgs`, and `Bound<PyAny>`) with the supported `PyO3`
//! `0.24.x` minor series to interact with an embedded Python interpreter.
//! Their primary job is bootstrapping `msgspec>=0.19,<0.20` with `uv` or `pip`
//! via `subprocess.run` so Rust and Python BDD tests can import it.
//! Only [`ensure_msgspec_installed`] and [`msgspec_available`] are exported;
//! `run_with_kwargs`, `install_msgspec`, and `has_uv` are private details.
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
    use rstest::rstest;
    use std::{ffi::CString, thread};

    struct RunAndKwargs<'py> {
        run: Bound<'py, PyAny>,
        kwargs: Bound<'py, PyDict>,
        globals: Bound<'py, PyDict>,
    }

    enum RunWithKwargsArgShape {
        Unit,
        NestedPyTuple,
        DirectPyTuple,
    }

    fn setup_run_and_kwargs(py: Python<'_>) -> RunAndKwargs<'_> {
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

    fn restore_subprocess_run(py: Python<'_>, globals: &Bound<'_, PyDict>) {
        let restore =
            CString::new("import subprocess\nsubprocess.run = _original_run\n_calls = []\n")
                .expect("CString build");
        py.run(restore.as_c_str(), Some(globals), None)
            .expect("restore subprocess.run");
    }

    fn recorded_call_count(globals: &Bound<'_, PyDict>) -> usize {
        globals
            .get_item("_calls")
            .expect("read recorded subprocess.run calls")
            .len()
            .expect("count recorded subprocess.run calls")
    }

    #[rstest]
    #[case::unit_tuple(RunWithKwargsArgShape::Unit)]
    #[case::one_tuple_of_pytuple(RunWithKwargsArgShape::NestedPyTuple)]
    #[case::bound_pytuple(RunWithKwargsArgShape::DirectPyTuple)]
    fn run_with_kwargs_accepts_supported_arg_shapes(#[case] arg_shape: RunWithKwargsArgShape) {
        Python::with_gil(|py| {
            let RunAndKwargs {
                run,
                kwargs,
                globals,
            } = setup_run_and_kwargs(py);

            match arg_shape {
                RunWithKwargsArgShape::Unit => run_with_kwargs(&run, (), &kwargs),
                RunWithKwargsArgShape::NestedPyTuple => {
                    let args_tuple = PyTuple::new(py, ["true"]).expect("build argument tuple");

                    run_with_kwargs(&run, (args_tuple,), &kwargs);
                }
                RunWithKwargsArgShape::DirectPyTuple => {
                    let args_tuple = PyTuple::new(py, [["true"]]).expect("build subprocess args");

                    run_with_kwargs(&run, args_tuple, &kwargs);
                }
            }

            let call_count = recorded_call_count(&globals);
            restore_subprocess_run(py, &globals);

            assert!(call_count > 0);
        });
    }

    #[test]
    fn msgspec_available_returns_bool() {
        assert_eq!(msgspec_available(), msgspec_available());
    }

    #[test]
    fn ensure_msgspec_installed_is_idempotent() {
        let first_result = Python::with_gil(ensure_msgspec_installed).is_ok();
        let second_result = Python::with_gil(ensure_msgspec_installed).is_ok();

        assert_eq!(first_result, second_result);
    }

    #[test]
    fn msgspec_available_matches_ensure_installed() {
        let ensure_result = Python::with_gil(|py| ensure_msgspec_installed(py).is_ok());

        assert_eq!(msgspec_available(), ensure_result);
    }

    #[test]
    fn ensure_msgspec_installed_is_safe_under_concurrent_access() {
        let handles: Vec<_> = (0..8)
            .map(|_| thread::spawn(|| Python::with_gil(ensure_msgspec_installed)))
            .collect();

        for handle in handles {
            assert!(handle.join().is_ok());
        }
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
