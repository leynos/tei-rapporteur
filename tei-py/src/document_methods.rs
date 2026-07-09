//! `#[pymethods]` implementations for the `Document` class.
use super::{Document, PyResult, emit_title_markup, wrap_tei_result};
use pyo3::pymethods;

#[pymethods]
impl Document {
    /// Constructs a document with the provided title.
    ///
    /// # Errors
    ///
    /// Returns [`pyo3::exceptions::PyValueError`] when the trimmed title is empty.
    #[new]
    pub fn new(title: &str) -> PyResult<Self> {
        wrap_tei_result(Self::try_from_title(title))
    }

    /// Returns the validated document title.
    #[getter]
    #[must_use]
    pub fn title(&self) -> String {
        self.inner.title().to_string()
    }

    /// Emits the document title as TEI markup.
    ///
    /// # Errors
    ///
    /// Returns [`pyo3::exceptions::PyValueError`] when the stored document title is invalid.
    pub fn emit_title_markup(&self) -> PyResult<String> {
        wrap_tei_result(emit_title_markup(self.inner.title().as_str()))
    }

    /// Validates document-wide invariants.
    ///
    /// # Errors
    ///
    /// Returns [`pyo3::exceptions::PyValueError`] when duplicated identifiers
    /// or unknown speaker references are detected.
    pub fn validate(&self) -> PyResult<()> {
        wrap_tei_result(self.inner.validate())
    }
}
