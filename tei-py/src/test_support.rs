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
    Python::attach(|py| ensure_msgspec_installed(py).is_ok())
}

#[cfg(test)]
mod tests {
    //! Unit tests for Python-side test support helpers.

    use super::*;
    use pyo3::{pyclass, pymethods};
    use rstest::rstest;
    use std::{
        ffi::CString,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
    };

    struct RunAndKwargs<'py> {
        run: Bound<'py, PyAny>,
        kwargs: Bound<'py, PyDict>,
        globals: Bound<'py, PyDict>,
    }

    #[derive(Clone, Copy)]
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
        // Restore `subprocess.run` and remove the bootstrap `meta_path` blocker
        // if one was installed. The blocker removal is a no-op for callers that
        // never installed it (e.g. the `run_with_kwargs` tests).
        let restore = CString::new(
            r"
import subprocess
import sys

subprocess.run = _original_run
_calls = []

try:
    sys.meta_path.remove(_bootstrap_msgspec_blocker)
except (ValueError, NameError):
    pass
",
        )
        .expect("CString build");
        py.run(restore.as_c_str(), Some(globals), None).ok();
    }

    struct SubprocessRestoreGuard<'py> {
        py: Python<'py>,
        globals: Bound<'py, PyDict>,
    }

    impl Drop for SubprocessRestoreGuard<'_> {
        fn drop(&mut self) {
            restore_subprocess_run(self.py, &self.globals);
        }
    }

    #[pyclass]
    struct BootstrapRunCounter {
        count: Arc<AtomicUsize>,
    }

    #[pymethods]
    impl BootstrapRunCounter {
        fn __call__(&self, _args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn setup_bootstrap_run_counter(py: Python<'_>, count: Arc<AtomicUsize>) -> pyo3::Py<PyDict> {
        let globals = PyDict::new(py);
        globals
            .set_item(
                "_counter",
                pyo3::Py::new(py, BootstrapRunCounter { count }).expect("build run counter"),
            )
            .expect("install run counter");
        // Force the bootstrap to actually run, then count and satisfy it:
        //   * a `meta_path` blocker makes `import msgspec` fail, so
        //     `ensure_msgspec_installed` cannot short-circuit and must enter the
        //     `Once`-guarded installer (even if msgspec is installed on disk);
        //   * the mocked `subprocess.run` counts each call and registers an
        //     importable stub `msgspec` module, so the installer's final
        //     `import msgspec` resolves without network access.
        // The counter therefore proves the installer path executed, and the
        // assertions fail if the bootstrap is ever skipped. Each test runs in
        // its own process under nextest, so the process-wide `Once` is fresh and
        // no cross-test reset seam or mutex is required.
        let patch = CString::new(
            r#"
import subprocess
import sys
import types


class _BlockMsgspecBootstrap:
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "msgspec" or fullname.startswith("msgspec."):
            raise ModuleNotFoundError("msgspec blocked for bootstrap test", name="msgspec")
        return None


_bootstrap_msgspec_blocker = _BlockMsgspecBootstrap()
sys.meta_path.insert(0, _bootstrap_msgspec_blocker)

_original_run = subprocess.run


def _mock_run(*args, **kwargs):
    _counter(args, kwargs)
    sys.modules.setdefault("msgspec", types.ModuleType("msgspec"))


subprocess.run = _mock_run
"#,
        )
        .expect("CString build");
        py.run(patch.as_c_str(), Some(&globals), None)
            .expect("monkeypatch subprocess.run");

        globals.unbind()
    }

    fn remove_msgspec_from_modules(py: Python<'_>) {
        let remove =
            CString::new("import sys\nsys.modules.pop('msgspec', None)\n").expect("CString build");
        py.run(remove.as_c_str(), None, None)
            .expect("remove msgspec from sys.modules");
    }

    fn recorded_call_count(globals: &Bound<'_, PyDict>) -> usize {
        globals
            .get_item("_calls")
            .expect("read recorded subprocess.run calls")
            .len()
            .expect("count recorded subprocess.run calls")
    }

    fn recorded_args<'py>(globals: &Bound<'py, PyDict>) -> Bound<'py, PyAny> {
        let expr = CString::new("_calls[0][0]").expect("CString build");
        globals
            .py()
            .eval(expr.as_c_str(), Some(globals), None)
            .expect("read recorded subprocess.run positional arguments")
    }

    #[rstest]
    #[case::unit_tuple(RunWithKwargsArgShape::Unit)]
    #[case::one_tuple_of_pytuple(RunWithKwargsArgShape::NestedPyTuple)]
    #[case::bound_pytuple(RunWithKwargsArgShape::DirectPyTuple)]
    fn run_with_kwargs_accepts_supported_arg_shapes(#[case] arg_shape: RunWithKwargsArgShape) {
        Python::attach(|py| {
            let RunAndKwargs {
                run,
                kwargs,
                globals,
            } = setup_run_and_kwargs(py);
            let _restore_guard = SubprocessRestoreGuard {
                py,
                globals: globals.clone(),
            };

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

            assert_eq!(call_count, 1);

            let args = recorded_args(&globals);
            match arg_shape {
                RunWithKwargsArgShape::Unit => {
                    assert_eq!(args.len().expect("count positional arguments"), 0);
                }
                RunWithKwargsArgShape::NestedPyTuple => {
                    let first_arg = args.get_item(0).expect("read first positional argument");
                    assert_eq!(
                        first_arg
                            .extract::<(String,)>()
                            .expect("extract nested tuple argument"),
                        ("true".to_owned(),)
                    );
                }
                RunWithKwargsArgShape::DirectPyTuple => {
                    assert_eq!(args.len().expect("count positional arguments"), 1);
                    let first_arg = args.get_item(0).expect("read first positional argument");
                    assert_eq!(
                        first_arg
                            .extract::<Vec<String>>()
                            .expect("extract direct list argument"),
                        vec!["true".to_owned()]
                    );
                }
            }
        });
    }

    #[test]
    fn ensure_msgspec_installed_invokes_subprocess_at_most_once_across_repeated_calls() {
        let run_count = Arc::new(AtomicUsize::new(0));

        let globals = Python::attach(|py| {
            let g = setup_bootstrap_run_counter(py, Arc::clone(&run_count));
            remove_msgspec_from_modules(py);
            g
        });

        assert!(Python::attach(ensure_msgspec_installed).is_ok());
        assert!(Python::attach(ensure_msgspec_installed).is_ok());

        assert!(
            Python::attach(|py| {
                restore_subprocess_run(py, globals.bind(py));
                ensure_msgspec_installed(py)
            })
            .is_ok()
        );

        // The blocker forces the installer to run exactly once: subprocess.run
        // is invoked for ensurepip and for the msgspec install. Asserting the
        // exact count proves the bootstrap path executed (it would be 0 if
        // skipped) and that the `Once` guard prevented a second bootstrap across
        // both repeated calls.
        assert_eq!(run_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn msgspec_available_reports_true_only_when_msgspec_is_importable() {
        // Call the function under test first; it may bootstrap msgspec as a
        // side-effect, so the importability check must come *after* the call.
        let reported_available = msgspec_available();
        let importable_after_check = Python::attach(|py| py.import("msgspec").is_ok());

        assert_eq!(reported_available, importable_after_check);
    }

    #[test]
    fn ensure_msgspec_installed_is_safe_under_concurrent_access() {
        let run_count = Arc::new(AtomicUsize::new(0));
        let globals = Python::attach(|py| {
            let globals = setup_bootstrap_run_counter(py, Arc::clone(&run_count));
            remove_msgspec_from_modules(py);

            globals
        });

        let handles: Vec<_> = (0..8)
            .map(|_| thread::spawn(move || Python::attach(ensure_msgspec_installed)))
            .collect();

        for handle in handles {
            assert!(handle.join().expect("bootstrap thread panicked").is_ok());
        }

        assert!(
            Python::attach(|py| {
                restore_subprocess_run(py, globals.bind(py));
                ensure_msgspec_installed(py)
            })
            .is_ok()
        );

        // The blocker forces the installer to run, and the `Once` guard fires
        // exactly once across all threads, making exactly two subprocess.run
        // calls (ensurepip + msgspec install). The exact count proves the
        // bootstrap path executed rather than being skipped.
        assert_eq!(run_count.load(Ordering::SeqCst), 2);
    }

    #[rstest]
    #[case::none_means_absent("None", false)]
    #[case::path_means_present("'/usr/bin/uv'", true)]
    fn has_uv_reflects_which_return_value(#[case] which_return_expr: &str, #[case] expected: bool) {
        Python::attach(|py| {
            let globals = PyDict::new(py);
            let patch = CString::new(format!(
                "import shutil\n\
                 orig = shutil.which\n\
                 shutil.which = lambda name: {which_return_expr}\n"
            ))
            .expect("CString build");
            py.run(patch.as_c_str(), Some(&globals), None)
                .expect("monkeypatch shutil.which");
            let _restore_guard = ShutilRestoreGuard {
                py,
                globals: globals.clone(),
            };

            assert_eq!(has_uv(py), expected);
        });
    }

    fn restore_shutil_which(py: Python<'_>, globals: &Bound<'_, PyDict>) {
        let restore = CString::new("import shutil\nshutil.which = orig\n").expect("CString build");
        py.run(restore.as_c_str(), Some(globals), None).ok();
    }

    struct ShutilRestoreGuard<'py> {
        py: Python<'py>,
        globals: Bound<'py, PyDict>,
    }

    impl Drop for ShutilRestoreGuard<'_> {
        fn drop(&mut self) {
            restore_shutil_which(self.py, &self.globals);
        }
    }
}
