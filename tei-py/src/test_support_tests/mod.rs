//! Unit tests for Python-side test support helpers.
//!
//! This parent module wires together deterministic unit tests, bootstrap mocks,
//! property tests, and shared Python monkeypatch helpers. `bootstrap_mocks`
//! provides the `subprocess.run` replacement and `msgspec` import blocker used
//! to exercise the installer path. `properties` contains the `proptest`
//! coverage for idempotency and thread-safety invariants. `test_helpers`
//! provides the shared fixtures and lock-backed restoration guards for tests
//! that mutate process-wide Python state. This module directly tests
//! `run_with_kwargs`, `msgspec_available`, and `has_uv`.

mod bootstrap_mocks;
mod properties;
mod test_helpers;

use super::*;
use bootstrap_mocks::{recorded_args, recorded_call_count};
use pyo3::{Bound, Python, types::PyDict};
use rstest::rstest;
use std::ffi::CString;
use test_helpers::{
    RunAndKwargs, RunWithKwargsArgShape, ShutilRestoreGuard, SubprocessRestoreGuard,
    acquire_shutil_patch_lock, setup_run_and_kwargs,
};

/// Verifies that supported Rust argument shapes reach Python unchanged.
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
            patch_guard,
        } = setup_run_and_kwargs(py);
        let _restore_guard = SubprocessRestoreGuard {
            py,
            globals: globals.clone(),
            _patch_guard: patch_guard,
        };

        match arg_shape {
            RunWithKwargsArgShape::Unit => run_with_kwargs(&run, (), &kwargs),
            RunWithKwargsArgShape::NestedPyTuple => {
                let args_tuple =
                    pyo3::types::PyTuple::new(py, ["true"]).expect("build argument tuple");

                run_with_kwargs(&run, (args_tuple,), &kwargs);
            }
            RunWithKwargsArgShape::DirectPyTuple => {
                let args_tuple =
                    pyo3::types::PyTuple::new(py, [["true"]]).expect("build subprocess args");

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

/// Verifies that availability mirrors actual Python importability.
#[test]
fn msgspec_available_reports_true_only_when_msgspec_is_importable() {
    // Call the function under test first; it may bootstrap msgspec as a
    // side-effect, so the importability check must come *after* the call.
    let reported_available = msgspec_available();
    let importable_after_check = Python::attach(|py| py.import("msgspec").is_ok());

    assert_eq!(reported_available, importable_after_check);
}

/// Verifies `uv` discovery against mocked `shutil.which` outcomes.
#[rstest]
#[case::none_means_absent(None, false)]
#[case::path_means_present(Some("/usr/bin/uv"), true)]
fn has_uv_reflects_which_return_value(
    #[case] which_return_value: Option<&str>,
    #[case] expected: bool,
) {
    let patch_guard = acquire_shutil_patch_lock();
    Python::attach(|py| {
        let globals = PyDict::new(py);
        globals
            .set_item("_which_return", which_return_value)
            .expect("install shutil.which return value");
        let patch = CString::new(
            "import shutil\n\
             orig = shutil.which\n\
             shutil.which = lambda name: _which_return\n",
        )
        .expect("CString build");
        py.run(patch.as_c_str(), Some(&globals), None)
            .expect("monkeypatch shutil.which");
        let _restore_guard = ShutilRestoreGuard {
            py,
            globals: globals.clone(),
            _patch_guard: patch_guard,
        };

        assert_eq!(has_uv(py), expected);
    });
}

/// Restores `shutil.which` after `has_uv` tests patch it.
pub(super) fn restore_shutil_which(py: Python<'_>, globals: &Bound<'_, PyDict>) {
    let restore = CString::new("import shutil\nshutil.which = orig\n").expect("CString build");
    py.run(restore.as_c_str(), Some(globals), None).ok();
}
