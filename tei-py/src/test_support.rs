//! Test-only helpers shared across Rust and Python BDD suites.
use pyo3::{PyResult, Python, types::PyAnyMethods};

/// Ensures `msgspec` is importable by the embedded Python interpreter.
///
/// The helper bootstraps `pip` via `ensurepip` when necessary and performs a
/// best-effort installation of `msgspec==0.19.0`. It returns an error only when
/// importing `msgspec` still fails after the attempted install.
///
/// # Errors
///
/// Returns a `PyErr` when importing or installing `msgspec` fails, for example
/// when `pip` is unavailable in the embedded interpreter.
pub fn ensure_msgspec_installed(py: Python<'_>) -> PyResult<()> {
    if py.import("msgspec").is_ok() {
        return Ok(());
    }

    let subprocess = py.import("subprocess")?;
    let sys = py.import("sys")?;
    let executable = sys.getattr("executable")?;

    if let Ok(check_call) = subprocess.getattr("check_call") {
        let _ = check_call
            .call1(((executable.clone(), "-m", "ensurepip", "--upgrade"),))
            .is_err();
    }

    if let Ok(check_call) = subprocess.getattr("check_call") {
        let _ = check_call
            .call1(((
                executable,
                "-m",
                "pip",
                "install",
                "--break-system-packages",
                "msgspec==0.19.0",
            ),))
            .is_err();
    }

    py.import("msgspec")?;
    Ok(())
}
