//! Unit tests validating the `tei_rapporteur.structs` submodule registration
//! and `MessagePack` round-trip through Python `msgspec.Struct` projections.
use super::*;
use crate::test_support::with_python;
use pyo3::{
    Py, Python,
    exceptions::{PyAttributeError, PyValueError},
    types::{PyAnyMethods, PyDict, PyModule},
};
use rstest::{fixture, rstest};
use std::ffi::CString;

fn report_import_restore_failure(py: Python<'_>, error: &pyo3::PyErr) {
    if std::thread::panicking() {
        if let Ok(stderr) = py.import("sys").and_then(|sys| sys.getattr("stderr")) {
            stderr
                .call_method1(
                    "write",
                    (format!(
                        "failed to restore msgspec import blocker: {error}\n"
                    ),),
                )
                .ok();
        }
        return;
    }

    panic!("failed to restore msgspec import blocker: {error}");
}

struct RestoreImportsGuard<'py> {
    py: Python<'py>,
    script: CString,
}

impl Drop for RestoreImportsGuard<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.py.run(self.script.as_c_str(), None, None) {
            report_import_restore_failure(self.py, &error);
        }
    }
}

#[fixture]
fn registered_module() -> Py<PyModule> {
    registered_structs_module("msgspec bootstrap should succeed for structs module tests")
}

#[rstest]
fn structs_submodule_is_registered(#[from(registered_module)] module: Py<PyModule>) {
    with_python(|py| {
        let bound_module = module.bind(py);
        assert!(
            bound_module
                .hasattr("structs")
                .expect("attribute check should succeed"),
            "structs submodule must be exported"
        );

        let structs = bound_module
            .getattr("structs")
            .expect("structs module should exist");
        assert!(
            structs
                .hasattr("Episode")
                .expect("Episode attribute lookup should succeed"),
            "Episode class must be available for msgspec decoding"
        );
    });
}

#[test]
fn structs_submodule_is_not_registered_when_msgspec_missing() {
    // Restores the import machinery on scope exit — including panic unwind —
    // so the msgspec blocker can never leak into other in-process tests.
    with_python(|py| {
        // Block msgspec imports for the duration of this test.
        let block_msgspec = CString::new(
            r#"
import sys

_orig_meta_path_structs_test = list(sys.meta_path)
_orig_msgspec_structs_missing = object()
_orig_msgspec_structs_test = sys.modules.get("msgspec", _orig_msgspec_structs_missing)

class _BlockMsgspecImport:
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "msgspec" or fullname.startswith("msgspec."):
            raise ModuleNotFoundError("msgspec is blocked for test", name="msgspec")
        return None

_blocker_structs_test = _BlockMsgspecImport()
sys.meta_path.insert(0, _blocker_structs_test)
sys.modules.pop("msgspec", None)
"#,
        )
        .expect("inline Python should be valid");
        py.run(block_msgspec.as_c_str(), None, None)
            .expect("failed to install msgspec import blocker");

        // Arrange restoration now so the blocker is removed even if an assertion
        // below panics (Drop runs during unwind while the GIL is still held).
        let restore_imports = CString::new(
            r#"
import sys

try:
    sys.meta_path.remove(_blocker_structs_test)
except ValueError:
    pass

if "_orig_meta_path_structs_test" in globals():
    sys.meta_path = _orig_meta_path_structs_test

if "_orig_msgspec_structs_test" in globals():
    if _orig_msgspec_structs_test is _orig_msgspec_structs_missing:
        sys.modules.pop("msgspec", None)
    else:
        sys.modules["msgspec"] = _orig_msgspec_structs_test
"#,
        )
        .expect("inline Python should be valid");
        let _restore_guard = RestoreImportsGuard {
            py,
            script: restore_imports,
        };

        // Register the module without calling the helper so msgspec remains absent.
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation should succeed");
        tei_rapporteur(py, &module)
            .expect("module registration should succeed even when msgspec is missing");

        let has_structs = module
            .hasattr("structs")
            .expect("attribute check for structs should succeed");
        assert!(
            !has_structs,
            "structs submodule must not be exported when msgspec is unavailable"
        );

        let err = module
            .getattr("structs")
            .expect_err("structs attribute should be absent when msgspec is missing");
        assert!(
            err.is_instance_of::<PyAttributeError>(py),
            "missing structs attribute should surface as AttributeError"
        );

        // Import machinery is restored by `_restore_guard` on scope exit.
    });
}

#[rstest]
fn episode_struct_round_trips_messagepack(#[from(registered_module)] module: Py<PyModule>) {
    with_python(|py| {
        let bound_module = module.bind(py);
        let document = Document::try_from_title("Bridgewater")
            .expect("valid title should construct a document");
        let payload: Vec<u8> = bound_module
            .getattr("to_msgpack")
            .expect("to_msgpack export")
            .call1((document,))
            .expect("to_msgpack call")
            .extract()
            .expect("payload extraction");

        let structs = bound_module.getattr("structs").expect("structs module");
        let episode_type = structs.getattr("Episode").expect("Episode class");
        let msgpack = py
            .import("msgspec.msgpack")
            .expect("msgspec.msgpack import should succeed");
        let decode_kwargs = PyDict::new(py);
        decode_kwargs
            .set_item("type", episode_type)
            .expect("kwargs population");

        let episode = msgpack
            .getattr("decode")
            .expect("decode function")
            .call((payload,), Some(&decode_kwargs))
            .expect("msgspec decoding should succeed");

        let header = episode
            .getattr("header")
            .expect("Episode should expose header");
        let file_desc = header
            .getattr("file_desc")
            .expect("Header should expose file_desc");
        file_desc
            .setattr("title", "Bridgewater Remix")
            .expect("titles should be mutable");

        let updated_payload: Vec<u8> = msgpack
            .getattr("encode")
            .expect("encode function")
            .call1((episode,))
            .expect("encoding Episode should succeed")
            .extract()
            .expect("payload extraction");
        let round_tripped =
            document_from_msgpack(updated_payload.as_slice()).expect("payload should decode");

        assert_eq!(round_tripped.title().as_str(), "Bridgewater Remix");
    });
}

#[rstest]
fn list_block_rejects_empty_items(#[from(registered_module)] module: Py<PyModule>) {
    with_python(|py| {
        let bound_module = module.bind(py);
        let structs = bound_module.getattr("structs").expect("structs module");
        let list_block_type = structs.getattr("ListBlock").expect("ListBlock class");

        let error = list_block_type
            .call0()
            .expect_err("ListBlock should reject empty items");
        assert!(
            error.is_instance_of::<PyValueError>(py),
            "empty ListBlock should raise ValueError"
        );
        assert!(
            error
                .to_string()
                .contains("ListBlock must contain at least one Item"),
            "error should explain the ListBlock invariant"
        );
    });
}

#[rstest]
fn div_block_rejects_blank_type(#[from(registered_module)] module: Py<PyModule>) {
    with_python(|py| {
        let bound_module = module.bind(py);
        let structs = bound_module.getattr("structs").expect("structs module");
        let div_block_type = structs.getattr("DivBlock").expect("DivBlock class");

        let error = div_block_type
            .call1(("   ",))
            .expect_err("DivBlock should reject blank div_type values");
        assert!(
            error.is_instance_of::<PyValueError>(py),
            "blank DivBlock div_type should raise ValueError"
        );
        assert!(
            error
                .to_string()
                .contains("div_type must contain non-whitespace text"),
            "error should explain the DivBlock invariant"
        );
    });
}
