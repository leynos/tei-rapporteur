//! Integration-style tests for the `PyO3` bindings that require module wiring.

use crate::test_support::{bootstrap_msgspec_attached, with_python};
use pyo3::types::{PyAnyMethods, PyList};
use pyo3::{Bound, PyResult, Python, exceptions::PyKeyError, types::PyModule};

fn report_import_restore_failure(py: Python<'_>, error: &pyo3::PyErr) {
    if std::thread::panicking() {
        if let Ok(stderr) = py.import("sys").and_then(|sys| sys.getattr("stderr")) {
            stderr
                .call_method1(
                    "write",
                    (format!(
                        "failed to restore tei_rapporteur.structs module: {error}\n"
                    ),),
                )
                .ok();
        }
        return;
    }

    panic!("failed to restore tei_rapporteur.structs module: {error}");
}

fn ignore_missing_structs_module(py: Python<'_>, error: pyo3::PyErr) -> PyResult<()> {
    if error.is_instance_of::<PyKeyError>(py) {
        Ok(())
    } else {
        Err(error)
    }
}

/// Restores `sys.modules["tei_rapporteur.structs"]` on drop so a test that
/// deletes it cannot leak that mutation into other in-process tests, even on
/// panic.
struct RestoreStructs<'py> {
    sys_modules: Bound<'py, pyo3::types::PyAny>,
    previous: Option<Bound<'py, pyo3::types::PyAny>>,
}

impl<'py> RestoreStructs<'py> {
    /// Snapshots `sys.modules["tei_rapporteur.structs"]` and removes the entry
    /// so the test starts from a clean state. `Drop` unconditionally restores
    /// the snapshot on scope exit, including panic unwind.
    fn new(sys_modules: &Bound<'py, pyo3::types::PyAny>) -> Self {
        let previous = sys_modules.get_item("tei_rapporteur.structs").ok();
        sys_modules.del_item("tei_rapporteur.structs").ok();
        Self {
            sys_modules: sys_modules.clone(),
            previous,
        }
    }
}

impl Drop for RestoreStructs<'_> {
    fn drop(&mut self) {
        let py = self.sys_modules.py();
        let restore_result = if let Some(previous) = self.previous.take() {
            self.sys_modules
                .set_item("tei_rapporteur.structs", previous)
        } else {
            self.sys_modules
                .del_item("tei_rapporteur.structs")
                .or_else(|error| ignore_missing_structs_module(py, error))
        };

        if let Err(error) = restore_result {
            report_import_restore_failure(py, &error);
        }
    }
}

/// Allocates and registers the `tei_rapporteur` module for a test.
///
/// Registration is arrangement, not the assertion under test, so failures
/// propagate to the calling test body.
fn registered_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let module = PyModule::new(py, "tei_rapporteur")?;
    crate::bindings_test_support::register_tei_rapporteur_module_for_tests(py, &module)?;
    Ok(module)
}

#[test]
fn to_dict_rejects_non_document_inputs() {
    with_python(|py| {
        let module = registered_module(py).expect("module registration should succeed");

        let to_dict = module
            .getattr("to_dict")
            .expect("to_dict should be registered");

        let error = to_dict
            .call1((py.None(),))
            .expect_err("passing non-Document should fail with a Python type error");
        assert!(error.is_instance_of::<pyo3::exceptions::PyTypeError>(py));
    });
}

#[test]
fn spoken_text_segments_return_msgspec_structs() {
    with_python(|py| {
        assert!(
            bootstrap_msgspec_attached(py),
            "msgspec bootstrap should succeed for spoken text struct tests"
        );
        let module = registered_module(py).expect("module registration should succeed");
        let extractor = module
            .getattr("spoken_text_segments")
            .expect("spoken_text_segments should be registered");
        let xml = concat!(
            "<TEI>",
            "<teiHeader><fileDesc><title>Example</title></fileDesc></teiHeader>",
            "<text><body>",
            "<sp><speaker>Host</speaker><p xml:id=\"line-1\">Hello <seg>there</seg>.</p></sp>",
            "</body></text>",
            "</TEI>"
        );

        let result = extractor
            .call1((xml,))
            .expect("spoken extraction should succeed");
        let segments = result
            .cast::<PyList>()
            .expect("spoken extraction should return a list");
        assert_eq!(segments.len().expect("list length should be available"), 1);
        let segment = segments.get_item(0).expect("segment should be present");
        let structs = module.getattr("structs").expect("structs module present");
        let segment_type = structs
            .getattr("SpokenTextSegment")
            .expect("SpokenTextSegment class present");

        assert!(
            segment
                .is_instance(&segment_type)
                .expect("type check should not raise"),
            "spoken_text_segments[0] should be a structs.SpokenTextSegment instance"
        );

        let text: String = segment
            .getattr("text")
            .expect("segment should expose text")
            .extract()
            .expect("text should be a string");
        let locator: String = segment
            .getattr("locator")
            .expect("segment should expose locator")
            .extract()
            .expect("locator should be a string");
        let xml_id: Option<String> = segment
            .getattr("xml_id")
            .expect("segment should expose xml_id")
            .extract()
            .expect("xml_id should be optional string");

        assert_eq!(text, "Hello there.");
        assert_eq!(locator, "/TEI/text/body/sp[1]/p[1]");
        assert_eq!(xml_id.as_deref(), Some("line-1"));
    });
}

#[test]
fn spoken_text_segments_requires_registered_structs_module() {
    with_python(|py| {
        assert!(
            bootstrap_msgspec_attached(py),
            "msgspec bootstrap should succeed for missing structs-module test"
        );
        let sys_modules = py
            .import("sys")
            .expect("sys should import")
            .getattr("modules")
            .expect("sys.modules should exist");
        // RAII guard: snapshots and removes the entry now, restores on scope exit
        // (including panic unwind) so the mutation cannot leak into other in-process tests.
        let _restore = RestoreStructs::new(&sys_modules);
        let xml = concat!(
            "<TEI>",
            "<teiHeader><fileDesc><title>Example</title></fileDesc></teiHeader>",
            "<text><body><p>Hello.</p></body></text>",
            "</TEI>"
        );

        let xml_arg: pyo3::pybacked::PyBackedStr = pyo3::types::PyString::new(py, xml)
            .as_any()
            .extract()
            .expect("XML literal should back a PyBackedStr");
        let call_result = crate::bindings::py_exports::spoken_text_segments(py, xml_arg);

        let error = call_result.expect_err("missing structs module should raise");
        assert!(error.is_instance_of::<pyo3::exceptions::PyRuntimeError>(py));
        assert!(
            error
                .to_string()
                .contains("tei_rapporteur.structs is not registered")
        );
    });
}
