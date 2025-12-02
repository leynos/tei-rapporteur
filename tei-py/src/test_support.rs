//! Test-only helpers shared across Rust and Python BDD suites.
use pyo3::{
    Bound, PyResult, Python,
    sync::OnceExt,
    types::{PyAny, PyAnyMethods, PyDict},
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
    A: pyo3::IntoPyObject<'py, Target = pyo3::types::PyTuple>,
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
        run_with_kwargs(
            run,
            (("uv", "pip", "install", "--quiet", "msgspec>=0.19,<0.20"),),
            kwargs,
        );
    } else {
        run_with_kwargs(
            run,
            ((
                executable.clone(),
                "-m",
                "pip",
                "install",
                "--no-input",
                "--disable-pip-version-check",
                "--default-timeout",
                "15",
                "--retries",
                "1",
                "--break-system-packages",
                "msgspec>=0.19,<0.20",
            ),),
            kwargs,
        );
    }
}
use pyo3::sync::GILOnceCell;

static MSGSPEC_INIT: GILOnceCell<()> = GILOnceCell::new();

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn has_uv_reports_absence_when_which_returns_none() {
        Python::with_gil(|py| {
            let code = CString::new("import shutil\nshutil.which = lambda name: None\n")
                .expect("CString build");
            py.run(code.as_c_str(), None, None)
                .expect("monkeypatch shutil.which");

            assert!(!has_uv(py), "uv should be absent when which returns None");
        });
    }

    #[test]
    fn has_uv_reports_presence_when_which_returns_path() {
        Python::with_gil(|py| {
            let code = CString::new("import shutil\nshutil.which = lambda name: '/usr/bin/uv'\n")
                .expect("CString build");
            py.run(code.as_c_str(), None, None)
                .expect("monkeypatch shutil.which");

            assert!(
                has_uv(py),
                "uv should be detected when which returns a path"
            );
        });
    }
}
