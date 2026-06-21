//! Unit tests for Python-side test support helpers.

use super::{
    bootstrap::{has_uv, run_with_kwargs},
    ensure_msgspec_installed, msgspec_available,
};
use pyo3::{
    Bound, Py, Python, pyclass, pymethods,
    types::{PyAny, PyAnyMethods, PyDict, PyTuple},
};
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
    // Restore `subprocess.run` and remove the bootstrap `meta_path` blocker if
    // one was installed. The blocker removal is a no-op for callers that never
    // installed it (e.g. the `run_with_kwargs` tests).
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

fn setup_bootstrap_run_counter(py: Python<'_>, count: Arc<AtomicUsize>) -> Py<PyDict> {
    let globals = PyDict::new(py);
    globals
        .set_item(
            "_counter",
            Py::new(py, BootstrapRunCounter { count }).expect("build run counter"),
        )
        .expect("install run counter");
    // Count bootstrap subprocess calls without forcing a process-wide import
    // failure. When `msgspec` is already installed the helper should
    // short-circuit cleanly; when it is absent the mocked installer satisfies
    // the final import without touching the network.
    let patch = CString::new(
        r#"
import subprocess
import sys
import types
import importlib


class _BlockMsgspecBootstrap:
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "msgspec" or fullname.startswith("msgspec."):
            raise ModuleNotFoundError("msgspec blocked for bootstrap test", name="msgspec")
        return None


_bootstrap_msgspec_blocker = _BlockMsgspecBootstrap()

_original_run = subprocess.run


def _mock_run(*args, **kwargs):
    _counter(args, kwargs)
    try:
        sys.meta_path.remove(_bootstrap_msgspec_blocker)
    except ValueError:
        pass
    try:
        sys.modules["msgspec"] = importlib.import_module("msgspec")
    except ModuleNotFoundError:
        sys.modules.setdefault("msgspec", types.ModuleType("msgspec"))
    finally:
        if _bootstrap_msgspec_blocker not in sys.meta_path:
            sys.meta_path.insert(0, _bootstrap_msgspec_blocker)


subprocess.run = _mock_run
"#,
    )
    .expect("CString build");
    py.run(patch.as_c_str(), Some(&globals), None)
        .expect("monkeypatch subprocess.run");

    globals.unbind()
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

        assert_eq!(recorded_call_count(&globals), 1);

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

    let globals = Python::attach(|py| setup_bootstrap_run_counter(py, Arc::clone(&run_count)));

    assert!(Python::attach(ensure_msgspec_installed).is_ok());
    assert!(Python::attach(ensure_msgspec_installed).is_ok());

    assert!(
        Python::attach(|py| {
            restore_subprocess_run(py, globals.bind(py));
            ensure_msgspec_installed(py)
        })
        .is_ok()
    );

    // If `msgspec` is already importable, the bootstrap should not run. If it
    // is absent, the `Once` guard should limit the helper to the ensurepip call
    // plus one install attempt across both invocations.
    assert!(run_count.load(Ordering::SeqCst) <= 2);
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
    let globals = Python::attach(|py| setup_bootstrap_run_counter(py, Arc::clone(&run_count)));

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

    // If `msgspec` is already importable, the bootstrap should not run. If it
    // is absent, the `Once` guard should limit all threads to the ensurepip
    // call plus one install attempt.
    assert!(run_count.load(Ordering::SeqCst) <= 2);
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
