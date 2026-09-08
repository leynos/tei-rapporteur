//! Unit tests for Python-side test support helpers.
//!
//! This module validates the private bootstrap machinery behind
//! `test_support`, including subprocess invocation shapes, `uv`/`pip`
//! fallback behaviour, and the import-state locking contract exposed through
//! the parent module's public helpers.

mod shutil_mocks;
mod subprocess_mocks;

use self::shutil_mocks::ShutilRestoreGuard;
use self::subprocess_mocks::{
    BootstrapRestoreGuard, RunAndKwargs, SubprocessRestoreGuard, recorded_args,
    recorded_call_count, setup_bootstrap_run_counter, setup_run_and_kwargs,
};
use super::{
    bootstrap::{ensure_msgspec_installed, install_msgspec},
    bootstrap::{has_uv, msgspec_version_satisfies_requirement, run_with_kwargs},
    bootstrap_msgspec, python_import_state_lock, with_python,
};
use pyo3::{
    Py, PyResult, Python,
    types::{PyAnyMethods, PyDict, PyTuple},
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

#[derive(Clone, Copy)]
enum RunWithKwargsArgShape {
    Unit,
    NestedPyTuple,
    DirectPyTuple,
}

struct BootstrapTestGuard {
    _restore_guard: BootstrapRestoreGuard,
    _import_state_lock: std::sync::MutexGuard<'static, ()>,
}

#[rstest]
#[case::unit_tuple(RunWithKwargsArgShape::Unit)]
#[case::one_tuple_of_pytuple(RunWithKwargsArgShape::NestedPyTuple)]
#[case::bound_pytuple(RunWithKwargsArgShape::DirectPyTuple)]
fn run_with_kwargs_accepts_supported_arg_shapes(#[case] arg_shape: RunWithKwargsArgShape) {
    with_python(|py| {
        let RunAndKwargs {
            run,
            kwargs,
            globals,
        } = setup_run_and_kwargs(py).expect("subprocess.run monkeypatch should install");
        let _restore_guard = SubprocessRestoreGuard::new(py, globals.clone());

        match arg_shape {
            RunWithKwargsArgShape::Unit => {
                run_with_kwargs(&run, (), &kwargs);
            }
            RunWithKwargsArgShape::NestedPyTuple => {
                let args_tuple = PyTuple::new(py, ["true"]).expect("build argument tuple");

                run_with_kwargs(&run, (args_tuple,), &kwargs);
            }
            RunWithKwargsArgShape::DirectPyTuple => {
                let args_tuple = PyTuple::new(py, [["true"]]).expect("build subprocess args");

                run_with_kwargs(&run, args_tuple, &kwargs);
            }
        }

        let call_count =
            recorded_call_count(&globals).expect("recorded call count should be readable");
        assert_eq!(call_count, 1);

        let args = recorded_args(&globals).expect("recorded arguments should be readable");
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
fn install_msgspec_falls_back_to_pip_when_uv_fails() {
    with_python(|py| {
        let globals = PyDict::new(py);
        let patch = CString::new(
            r#"
_calls = []


def _run(args, **kwargs):
    _calls.append(args)
    if args[0] == "uv":
        raise RuntimeError("uv failed")
"#,
        )
        .expect("CString build");
        py.run(patch.as_c_str(), Some(&globals), None)
            .expect("install failing uv mock");
        let run = globals.get_item("_run").expect("read mocked run");
        let executable = py
            .import("sys")
            .expect("import sys")
            .getattr("executable")
            .expect("read sys.executable");
        let kwargs = PyDict::new(py);

        install_msgspec(&run, &executable, &kwargs, true);

        let calls = globals.get_item("_calls").expect("read subprocess calls");
        assert_eq!(calls.len().expect("count subprocess calls"), 2);
        let first_call = calls.get_item(0).expect("read uv call");
        assert_eq!(
            first_call
                .get_item(0)
                .expect("read uv executable")
                .extract::<String>()
                .expect("extract uv executable"),
            "uv"
        );
        let second_call = calls.get_item(1).expect("read pip fallback call");
        assert_eq!(
            second_call
                .get_item(1)
                .expect("read pip module flag")
                .extract::<String>()
                .expect("extract pip module flag"),
            "-m"
        );
        assert_eq!(
            second_call
                .get_item(2)
                .expect("read pip module")
                .extract::<String>()
                .expect("extract pip module"),
            "pip"
        );
    });
}

#[rstest]
#[case::minor_only("0.19", true)]
#[case::v_prefixed("v0.19", true)]
#[case::patch_release("0.19.0", true)]
#[case::patch_update("0.19.7", true)]
#[case::local_version("0.19.0+local", true)]
#[case::post_release("0.19.post1", true)]
#[case::older_minor("0.18.9", false)]
#[case::newer_minor("0.20.0", false)]
#[case::release_candidate("0.19rc1", false)]
#[case::dev_release("0.19.dev0", false)]
fn msgspec_version_requirement_matches_declared_range(
    #[case] version: &str,
    #[case] expected: bool,
) {
    assert_eq!(msgspec_version_satisfies_requirement(version), expected);
}

fn setup_bootstrap_test_unlocked() -> PyResult<(Arc<AtomicUsize>, Py<PyDict>, BootstrapRestoreGuard)>
{
    let run_count = Arc::new(AtomicUsize::new(0));
    let globals = Python::attach(|py| setup_bootstrap_run_counter(py, Arc::clone(&run_count)))?;
    let restore_guard = BootstrapRestoreGuard::new(Python::attach(|py| globals.clone_ref(py)));
    Ok((run_count, globals, restore_guard))
}

/// Sets up the `subprocess.run` monkeypatch and returns a run-call counter,
/// the `globals` dict that owns the patch, and an RAII guard that tears it
/// down on scope exit.
///
/// Installing the monkeypatch can fail, so the caller decides the verdict.
fn setup_bootstrap_test() -> PyResult<(Arc<AtomicUsize>, Py<PyDict>, BootstrapTestGuard)> {
    let import_state_lock = python_import_state_lock();
    let (run_count, globals, restore_guard) = setup_bootstrap_test_unlocked()?;
    let test_guard = BootstrapTestGuard {
        _restore_guard: restore_guard,
        _import_state_lock: import_state_lock,
    };
    Ok((run_count, globals, test_guard))
}

/// Asserts the bootstrap was invoked at most twice across the test.
fn assert_bootstrap_once(run_count: &AtomicUsize) {
    assert!(
        run_count.load(Ordering::SeqCst) <= 2,
        "bootstrap should run subprocess at most twice (ensurepip + install)"
    );
}

fn bootstrap_msgspec_unlocked() -> bool {
    Python::attach(|py| ensure_msgspec_installed(py).is_ok())
}

fn force_msgspec_bootstrap_path(globals: &Py<PyDict>) -> PyResult<()> {
    Python::attach(|py| {
        let patch = CString::new(concat!(
            "import sys\n",
            "sys.modules.pop('msgspec', None)\n",
            "if _bootstrap_msgspec_blocker not in sys.meta_path:\n",
            "    sys.meta_path.insert(0, _bootstrap_msgspec_blocker)\n",
        ))
        .map_err(|error| {
            pyo3::exceptions::PyValueError::new_err(format!("CString build failed: {error}"))
        })?;
        py.run(patch.as_c_str(), Some(globals.bind(py)), None)
    })
}

#[test]
fn ensure_msgspec_installed_invokes_subprocess_at_most_once_across_repeated_calls() {
    let (run_count, _globals, _restore_guard) =
        setup_bootstrap_test().expect("bootstrap monkeypatch should install");

    assert!(Python::attach(ensure_msgspec_installed).is_ok());
    let first_call_count = run_count.load(Ordering::SeqCst);
    assert!(Python::attach(ensure_msgspec_installed).is_ok());
    assert_eq!(
        run_count.load(Ordering::SeqCst),
        first_call_count,
        "subsequent calls should not re-run bootstrap work"
    );

    assert_bootstrap_once(&run_count);
}

#[test]
fn mocked_bootstrap_scopes_reset_one_shot_state() {
    {
        let (run_count, globals, _restore_guard) =
            setup_bootstrap_test().expect("bootstrap monkeypatch should install");
        force_msgspec_bootstrap_path(&globals).expect("force msgspec bootstrap path");

        assert!(Python::attach(ensure_msgspec_installed).is_ok());
        assert!(
            run_count.load(Ordering::SeqCst) > 0,
            "first mocked scope should run bootstrap subprocesses"
        );
        assert_bootstrap_once(&run_count);
    }

    {
        let (run_count, globals, _restore_guard) =
            setup_bootstrap_test().expect("bootstrap monkeypatch should install");
        force_msgspec_bootstrap_path(&globals).expect("force msgspec bootstrap path");

        assert!(Python::attach(ensure_msgspec_installed).is_ok());
        assert!(
            run_count.load(Ordering::SeqCst) > 0,
            "second mocked scope should run after reset"
        );
        assert_bootstrap_once(&run_count);
    }
}

#[test]
fn bootstrap_msgspec_reports_true_only_when_msgspec_is_importable() {
    let (run_count, _globals, _restore_guard) =
        setup_bootstrap_test().expect("bootstrap monkeypatch should install");

    // Call the function under test first; it may bootstrap msgspec as a
    // side-effect, so the importability check must come *after* the call.
    let reported_available = bootstrap_msgspec_unlocked();
    let importable_after_check = Python::attach(|py| py.import("msgspec").is_ok());

    assert_eq!(reported_available, importable_after_check);
    assert_bootstrap_once(&run_count);
}

#[test]
fn bootstrap_msgspec_public_wrapper_smoke_test() {
    let reported_available = bootstrap_msgspec();
    let importable_after_check = with_python(|py| py.import("msgspec").is_ok());

    assert_eq!(reported_available, importable_after_check);
}

#[test]
fn ensure_msgspec_installed_is_safe_under_concurrent_access() {
    let run_count = Arc::new(AtomicUsize::new(0));
    let globals = with_python(|py| setup_bootstrap_run_counter(py, Arc::clone(&run_count)))
        .expect("bootstrap run counter should install");
    let restore_guard = BootstrapRestoreGuard::new(with_python(|py| globals.clone_ref(py)));

    // Same-thread `Python::attach` calls in lock-held helpers stay direct, but
    // spawned threads do not inherit that guard and must use `with_python`.
    let handles: Vec<_> = (0..8)
        .map(|_| thread::spawn(move || with_python(ensure_msgspec_installed)))
        .collect();
    let results: Vec<_> = handles.into_iter().map(thread::JoinHandle::join).collect();

    let _import_state_lock = python_import_state_lock();
    drop(restore_guard);

    for result in results {
        assert!(result.expect("bootstrap thread panicked").is_ok());
    }
    assert_bootstrap_once(&run_count);
}

#[rstest]
#[case::none_means_absent("None", false)]
#[case::path_means_present("'/usr/bin/uv'", true)]
fn has_uv_reflects_which_return_value(#[case] which_return_expr: &str, #[case] expected: bool) {
    with_python(|py| {
        let globals = PyDict::new(py);
        let patch = CString::new(format!(
            concat!(
                "import shutil\n",
                "orig = shutil.which\n",
                "shutil.which = lambda name: {which_return_expr}\n",
            ),
            which_return_expr = which_return_expr,
        ))
        .expect("CString build");
        py.run(patch.as_c_str(), Some(&globals), None)
            .expect("monkeypatch shutil.which");
        let _restore_guard = ShutilRestoreGuard::new(py, globals.clone());

        assert_eq!(has_uv(py), expected);
    });
}
