//! `PyO3` bindings and helper functions exposed to Python callers.
//!
//! The crate surfaces the `tei_rapporteur` module, offering a lightweight
//! `Document` wrapper that delegates validation to the Rust core. The module
//! currently exposes title-centric helpers so downstream phases can evolve the
//! API without rewriting the glue code. Rust callers continue to use the
//! `emit_title_markup` helper directly whilst Python receives mirrored
//! bindings.

use rmp_serde::decode::Error as MsgpackError;
use tei_core::{TeiDocument, TeiError};
use tei_xml::serialize_document_title;

pub use bindings::{Document, from_msgpack, tei_rapporteur};

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

fn document_from_msgpack(bytes: &[u8]) -> Result<TeiDocument, MsgpackError> {
    rmp_serde::from_slice(bytes)
}

mod bindings {
    #![expect(
        unsafe_op_in_unsafe_fn,
        reason = "PyO3 generates unavoidable unsafe glue for the Python bindings"
    )]
    #![expect(
        clippy::shadow_reuse,
        reason = "PyO3 reuses module parameters when generating the PyInit stub"
    )]
    #![expect(
        clippy::too_many_arguments,
        reason = "PyO3 synthesises adapter parameters for exported pyfunctions"
    )]
    #![expect(
        clippy::useless_conversion,
        reason = "Result<T, TeiError> must be mapped into PyResult<T> for Python error translation"
    )]

    use super::{TeiDocument, TeiError, document_from_msgpack, emit_title_markup};
    use pyo3::Bound;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use pyo3::types::PyModule;
    use pyo3::wrap_pyfunction;
    use std::ops::Deref;

    /// Wrapper around [`TeiDocument`] surfaced to Python.
    #[pyclass(module = "tei_rapporteur", name = "Document")]
    #[derive(Clone, Debug)]
    pub struct Document {
        inner: TeiDocument,
    }

    impl Document {
        /// Attempts to build a [`Document`] from a raw title string.
        ///
        /// # Errors
        ///
        /// Returns [`TeiError::DocumentTitle`] when the supplied title trims to
        /// an empty string.
        pub fn try_from_title(title: &str) -> Result<Self, TeiError> {
            TeiDocument::from_title_str(title).map(Self::from)
        }
    }

    impl From<TeiDocument> for Document {
        fn from(inner: TeiDocument) -> Self {
            Self { inner }
        }
    }

    impl From<Document> for TeiDocument {
        fn from(value: Document) -> Self {
            value.inner
        }
    }

    impl Deref for Document {
        type Target = TeiDocument;

        fn deref(&self) -> &Self::Target {
            &self.inner
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
            self.inner.title().to_string()
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

    #[pyfunction(name = "emit_title_markup")]
    fn emit_title_markup_py(raw_title: &str) -> PyResult<String> {
        wrap_tei_result(emit_title_markup(raw_title))
    }

    /// Deserialises `MessagePack` bytes into a [`Document`].
    ///
    /// # Errors
    ///
    /// Returns [`PyValueError`] when the payload cannot be decoded into a
    /// valid [`TeiDocument`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rmp_serde::to_vec_named;
    /// use tei_core::TeiDocument;
    /// use tei_py::from_msgpack;
    ///
    /// let source = TeiDocument::from_title_str("Wolf 359")?;
    /// let payload = to_vec_named(&source)?;
    /// let document = from_msgpack(&payload)?;
    /// assert_eq!(document.title(), "Wolf 359");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[pyfunction]
    pub fn from_msgpack(bytes: &[u8]) -> PyResult<Document> {
        document_from_msgpack(bytes)
            .map(Document::from)
            .map_err(|error| PyValueError::new_err(format!("invalid MessagePack payload: {error}")))
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
        py_module.add_function(wrap_pyfunction!(from_msgpack, py_module)?)?;
        py_module.add("__version__", env!("CARGO_PKG_VERSION"))?;
        py_module.add("__py_runtime__", py_context.version())?;
        Ok(())
    }

    /// Converts a Rust `Result<T, TeiError>` into a Python-friendly [`PyResult`].
    ///
    /// Successful values are forwarded unchanged, while [`TeiError`] values are
    /// rendered via [`to_string`](TeiError::to_string) and wrapped in
    /// [`PyValueError`]. This keeps the FFI boundary consistent by mapping Rust
    /// domain errors to Python exceptions in one place.
    fn wrap_tei_result<T>(result: Result<T, TeiError>) -> PyResult<T> {
        result.map_err(|error| PyValueError::new_err(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::{
        Python,
        types::{PyAnyMethods, PyModule},
    };
    use rmp_serde::to_vec_named;

    #[test]
    fn document_construction_trims_titles() {
        let document =
            Document::try_from_title("  Wolf 359  ").expect("valid document title should succeed");
        assert_eq!(document.title(), "Wolf 359");
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
            assert!(
                module
                    .hasattr("from_msgpack")
                    .expect("from_msgpack attribute check")
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

    #[test]
    fn from_msgpack_decodes_documents() {
        let fixture = TeiDocument::from_title_str("Wolf 359")
            .expect("valid title should build a TeiDocument");
        let payload = to_vec_named(&fixture).expect("MessagePack encoding should succeed");

        let document = from_msgpack(&payload).expect("MessagePack payload should decode");
        assert_eq!(document.title(), "Wolf 359");
    }

    #[test]
    fn from_msgpack_rejects_invalid_payloads() {
        let error = from_msgpack(b"this is not msgpack data")
            .expect_err("invalid payload should surface as an error");
        let message = error.to_string();
        assert!(
            message.contains("invalid MessagePack payload"),
            "error message should communicate MessagePack failure; found {message}"
        );
    }
}
