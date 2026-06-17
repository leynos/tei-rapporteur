//! Unit tests for Python-side test support helpers.

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
    setup_run_and_kwargs,
};

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

#[test]
fn msgspec_available_reports_true_only_when_msgspec_is_importable() {
    // Call the function under test first; it may bootstrap msgspec as a
    // side-effect, so the importability check must come *after* the call.
    let reported_available = msgspec_available();
    let importable_after_check = Python::attach(|py| py.import("msgspec").is_ok());

    assert_eq!(reported_available, importable_after_check);
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

pub(super) fn restore_shutil_which(py: Python<'_>, globals: &Bound<'_, PyDict>) {
    let restore = CString::new("import shutil\nshutil.which = orig\n").expect("CString build");
    py.run(restore.as_c_str(), Some(globals), None).ok();
}
