#![expect(
    unsafe_op_in_unsafe_fn,
    reason = "PyO3 0.22 emits implicit unsafe glue under Rust 2024"
)]
#![expect(
    clippy::too_many_arguments,
    reason = "PyO3 synthesises wrapper signatures beyond Clippy's threshold"
)]
#![expect(
    clippy::shadow_reuse,
    reason = "PyO3 reuses parameter names when generating PyInit stubs"
)]
#![expect(
    clippy::useless_conversion,
    reason = "Converting TeiError into PyErr is required at the Python boundary"
)]
//! `PyO3` bindings and helper functions exposed to Python callers.
//!
//! The crate surfaces the `tei_rapporteur` module, offering a lightweight
//! `Document` wrapper that delegates validation to the Rust core. The module
//! currently exposes title-centric helpers so downstream phases can evolve the
//! API without rewriting the glue code. Rust callers continue to use the
//! `emit_title_markup` helper directly whilst Python receives mirrored
//! bindings.

use pyo3::Bound;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::wrap_pyfunction;
use tei_core::{TeiDocument, TeiError};
use tei_xml::serialize_document_title;

/// Wrapper around [`TeiDocument`] surfaced to Python.
#[pyclass(module = "tei_rapporteur", name = "Document")]
#[derive(Clone, Debug)]
pub struct Document {
    inner: TeiDocument,
}

impl Document {
    /// Builds a wrapper from an existing [`TeiDocument`].
    #[must_use]
    pub const fn from_core(inner: TeiDocument) -> Self {
        Self { inner }
    }

    /// Attempts to build a [`Document`] from a raw title string.
    ///
    /// # Errors
    ///
    /// Returns [`TeiError::DocumentTitle`] when the supplied title trims to an
    /// empty string.
    pub fn try_from_title(title: &str) -> Result<Self, TeiError> {
        Ok(Self::from_core(TeiDocument::from_title_str(title)?))
    }

    /// Returns the underlying [`TeiDocument`] reference.
    #[must_use]
    pub const fn as_core(&self) -> &TeiDocument {
        &self.inner
    }

    /// Consumes the wrapper, yielding the underlying document.
    #[must_use]
    pub fn into_inner(self) -> TeiDocument {
        self.inner
    }
}

#[pymethods]
impl Document {
    /// Constructs a document with the provided title.
    ///
    /// # Errors
    ///
    /// Returns [`PyValueError`] when the trimmed title is empty.
    #[new]
    pub fn new(title: &str) -> PyResult<Self> {
        wrap_tei_result(Self::try_from_title(title))
    }

    /// Returns the validated document title.
    #[getter]
    #[must_use]
    pub fn title(&self) -> String {
        self.inner.title().as_str().to_owned()
    }

    /// Emits the document title as TEI markup.
    ///
    /// # Errors
    ///
    /// Returns [`PyValueError`] when the stored document title is invalid.
    pub fn emit_title_markup(&self) -> PyResult<String> {
        wrap_tei_result(emit_title_markup(self.inner.title().as_str()))
    }
}

/// Validates and emits TEI markup suitable for exposure through `PyO3`.
///
/// # Errors
///
/// Returns [`tei_core::TeiError::DocumentTitle`] when the provided title is
/// blank after trimming. The helper exists so `PyO3` glue can focus on Python
/// ergonomics whilst reusing the Rust validation logic.
///
/// # Examples
///
/// ```
/// use tei_py::emit_title_markup;
///
/// let markup = emit_title_markup("Welcome to Night Vale")?;
/// assert_eq!(markup, "<title>Welcome to Night Vale</title>");
/// # Ok::<(), tei_core::TeiError>(())
/// ```
pub fn emit_title_markup(raw_title: &str) -> Result<String, TeiError> {
    serialize_document_title(raw_title)
}

/// Python-facing helper that validates arbitrary titles.
///
/// # Errors
///
/// Returns [`PyValueError`] when the supplied title trims to an empty string.
#[pyfunction(name = "emit_title_markup")]
fn emit_title_markup_py(raw_title: &str) -> PyResult<String> {
    wrap_tei_result(emit_title_markup(raw_title))
}

/// Registers the `tei_rapporteur` Python module.
///
/// # Errors
///
/// Returns [`PyErr`] when registering the module exports fails because the
/// interpreter rejects one of the additions.
#[pymodule]
pub fn tei_rapporteur(py_context: Python<'_>, py_module: &Bound<'_, PyModule>) -> PyResult<()> {
    py_module.add_class::<Document>()?;
    py_module.add_function(wrap_pyfunction!(emit_title_markup_py, py_module)?)?;
    py_module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    py_module.add("__py_runtime__", py_context.version())?;
    Ok(())
}

fn wrap_tei_result<T>(result: Result<T, TeiError>) -> PyResult<T> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(PyValueError::new_err(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::{Python, types::PyModule};

    #[test]
    fn document_construction_trims_titles() {
        let document =
            Document::try_from_title("  Wolf 359  ").expect("valid document title should succeed");
        assert_eq!(document.as_core().title().as_str(), "Wolf 359");
    }

    #[test]
    fn document_construction_rejects_blank_titles() {
        let error = Document::try_from_title("   ").expect_err("blank titles should fail");
        assert!(matches!(error, TeiError::DocumentTitle(_)));
    }

    #[test]
    fn module_registers_python_bindings() {
        Python::with_gil(|py| {
            let module = PyModule::new_bound(py, "tei_rapporteur").expect("module allocation");
            tei_rapporteur(py, &module).expect("module registration");

            assert!(
                module
                    .hasattr("Document")
                    .expect("Document attribute check")
            );
            assert!(
                module
                    .hasattr("emit_title_markup")
                    .expect("emit_title_markup attribute check")
            );
        });
    }

    #[test]
    fn python_function_emits_markup() {
        Python::with_gil(|py| {
            let module = PyModule::new_bound(py, "tei_rapporteur").expect("module allocation");
            tei_rapporteur(py, &module).expect("module registration");
            let emit = module
                .getattr("emit_title_markup")
                .expect("emit_title_markup attribute");
            let result: String = emit
                .call1(("Archive 81",))
                .expect("Python call")
                .extract()
                .expect("string extraction");
            assert_eq!(result, "<title>Archive 81</title>");
        });
    }

    #[test]
    fn document_method_emits_markup() {
        let document = Document::try_from_title("King Falls AM").expect("valid doc");
        let markup = document
            .emit_title_markup()
            .expect("method should reuse core helper");
        assert_eq!(markup, "<title>King Falls AM</title>");
    }
}
