//! Test-only helpers shared across Rust and Python BDD suites.
use pyo3::{PyResult, Python, types::PyAnyMethods};
use std::sync::Once;

static MSGSPEC_INIT: Once = Once::new();

/// Ensures `msgspec` is importable by the embedded Python interpreter.
///
/// The helper bootstraps `pip` via `ensurepip` when necessary and performs a
/// best-effort installation of `msgspec==0.19.0`. It is thread-safe: install
/// attempts run at most once even when tests execute in parallel. It returns an
/// error only when importing `msgspec` still fails after the attempted install.
///
/// # Errors
///
/// Returns a `PyErr` when importing or installing `msgspec` fails, for example
/// when `pip` is unavailable in the embedded interpreter.
pub fn ensure_msgspec_installed(py: Python<'_>) -> PyResult<()> {
    if py.import("msgspec").is_ok() {
        return Ok(());
    }

    MSGSPEC_INIT.call_once(|| {
        Python::with_gil(|gil| {
            let Ok(subprocess) = gil.import("subprocess") else {
                return;
            };
            let Ok(sys) = gil.import("sys") else {
                return;
            };
            let Ok(executable) = sys.getattr("executable") else {
                return;
            };

            if let Ok(check_call) = subprocess.getattr("check_call") {
                drop(check_call.call1(((executable.clone(), "-m", "ensurepip", "--upgrade"),)));
            }

            if let Ok(check_call) = subprocess.getattr("check_call") {
                drop(check_call.call1(((
                    executable,
                    "-m",
                    "pip",
                    "install",
                    "--break-system-packages",
                    "msgspec==0.19.0",
                ),)));
            }
        });
    });

    py.import("msgspec")?;
    Ok(())
}
