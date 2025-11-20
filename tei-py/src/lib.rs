//! `PyO3` bindings and helper functions exposed to Python callers.
//!
//! The crate surfaces the `tei_rapporteur` module, offering a lightweight
//! `Document` wrapper that delegates validation to the Rust core. The module
//! currently exposes title-centric helpers so downstream phases can evolve the
//! API without rewriting the glue code. Rust callers continue to use the
//! `emit_title_markup` helper directly whilst Python receives mirrored
//! bindings.

use rmp_serde::{decode::Error as MsgpackDecodeError, encode::Error as MsgpackEncodeError};
use tei_core::{TeiDocument, TeiError};
use tei_xml::serialize_document_title;

pub use bindings::{Document, emit_xml, from_msgpack, parse_xml, tei_rapporteur, to_msgpack};

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

fn document_from_msgpack(bytes: &[u8]) -> Result<TeiDocument, MsgpackDecodeError> {
    rmp_serde::from_slice(bytes)
}

fn document_to_msgpack(document: &TeiDocument) -> Result<Vec<u8>, MsgpackEncodeError> {
    rmp_serde::to_vec_named(document)
}

mod bindings {
    //! `PyO3` glue that surfaces `TeiDocument` helpers to Python callers.

    use super::{
        TeiDocument, TeiError, document_from_msgpack, document_to_msgpack, emit_title_markup,
    };
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

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

    mod document_methods {
        #![expect(
            unsafe_op_in_unsafe_fn,
            reason = "PyO3 #[pymethods] glue performs unavoidable unsafe argument extraction"
        )]
        #![expect(
            clippy::useless_conversion,
            reason = "PyO3 wraps #[pymethods] returns in conversion helpers"
        )]

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
        }
    }

    mod py_exports {
        #![expect(
            unsafe_op_in_unsafe_fn,
            reason = "PyO3 #[pyfunction] glue performs unavoidable unsafe argument extraction"
        )]
        #![expect(
            clippy::too_many_arguments,
            reason = "PyO3 injects abi parameters into #[pyfunction] signatures"
        )]
        #![expect(
            clippy::useless_conversion,
            reason = "PyO3 wraps #[pyfunction] returns in conversion helpers"
        )]
        #![expect(
            clippy::shadow_reuse,
            reason = "PyO3 #[pymodule] expansion reuses the module parameter names"
        )]

        use super::{
            Document, PyResult, PyValueError, Python, document_from_msgpack, document_to_msgpack,
            emit_title_markup, wrap_tei_result,
        };
        use pyo3::types::PyModuleMethods;
        use pyo3::{Bound, types::PyModule};
        use pyo3::{pyfunction, pymodule, wrap_pyfunction};
        use tei_xml::{emit_xml as emit_document_xml, parse_xml as parse_document_xml};

        #[pyfunction(name = "emit_title_markup")]
        pub fn emit_title_markup_py(raw_title: &str) -> PyResult<String> {
            wrap_tei_result(emit_title_markup(raw_title))
        }

        /// Deserialises `MessagePack` bytes into a [`Document`].
        ///
        /// # Errors
        ///
        /// Returns [`pyo3::exceptions::PyValueError`] when the payload cannot be decoded into a
        /// valid [`tei_core::TeiDocument`].
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
                .map_err(|error| {
                    PyValueError::new_err(format!("invalid MessagePack payload: {error}"))
                })
        }

        /// Serialises a [`Document`] into `MessagePack` bytes.
        ///
        /// # Errors
        ///
        /// Returns [`pyo3::exceptions::PyValueError`] when `rmp_serde` fails to encode the document.
        ///
        /// # Examples
        ///
        /// ```
        /// use tei_py::{Document, to_msgpack, from_msgpack};
        ///
        /// let document = Document::try_from_title("Wolf 359")?;
        /// let payload = to_msgpack(&document)?;
        /// let decoded = from_msgpack(&payload)?;
        /// assert_eq!(decoded.title(), "Wolf 359");
        /// # Ok::<(), Box<dyn std::error::Error>>(())
        /// ```
        #[pyfunction]
        pub fn to_msgpack(document: &Document) -> PyResult<Vec<u8>> {
            document_to_msgpack(document).map_err(|error| {
                PyValueError::new_err(format!("MessagePack encoding failed: {error}"))
            })
        }

        /// Parses TEI XML into a [`Document`].
        ///
        /// # Errors
        ///
        /// Returns [`pyo3::exceptions::PyValueError`] when parsing fails due to
        /// invalid XML or TEI content.
        ///
        /// # Examples
        ///
        /// ```
        /// use tei_core::TeiDocument;
        /// use tei_py::parse_xml;
        /// use tei_xml::emit_xml;
        ///
        /// let source = TeiDocument::from_title_str("Wolf 359")?;
        /// let xml = emit_xml(&source)?;
        /// let document = parse_xml(&xml)?;
        /// assert_eq!(document.title(), "Wolf 359");
        /// # Ok::<(), Box<dyn std::error::Error>>(())
        /// ```
        #[pyfunction]
        pub fn parse_xml(xml: &str) -> PyResult<Document> {
            wrap_tei_result(parse_document_xml(xml)).map(Document::from)
        }

        /// Emits TEI XML from a [`Document`].
        ///
        /// # Errors
        ///
        /// Returns [`pyo3::exceptions::PyValueError`] when XML emission fails,
        /// for example due to forbidden control characters.
        ///
        /// # Examples
        ///
        /// ```
        /// use tei_py::{Document, emit_xml};
        ///
        /// let document = Document::try_from_title("Wolf 359")?;
        /// let xml = emit_xml(&document)?;
        /// assert!(xml.contains("<title>Wolf 359</title>"));
        /// # Ok::<(), Box<dyn std::error::Error>>(())
        /// ```
        #[pyfunction]
        pub fn emit_xml(document: &Document) -> PyResult<String> {
            wrap_tei_result(emit_document_xml(document))
        }

        /// Registers the `tei_rapporteur` Python module.
        ///
        /// # Errors
        ///
        /// Returns [`pyo3::PyErr`] when registering the module exports fails because the
        /// interpreter rejects one of the additions.
        #[pymodule]
        pub fn tei_rapporteur(
            py_context: Python<'_>,
            py_module: &Bound<'_, PyModule>,
        ) -> PyResult<()> {
            py_module.add_class::<Document>()?;
            py_module.add_function(wrap_pyfunction!(emit_title_markup_py, py_module)?)?;
            py_module.add_function(wrap_pyfunction!(from_msgpack, py_module)?)?;
            py_module.add_function(wrap_pyfunction!(to_msgpack, py_module)?)?;
            py_module.add_function(wrap_pyfunction!(parse_xml, py_module)?)?;
            py_module.add_function(wrap_pyfunction!(emit_xml, py_module)?)?;
            py_module.add("__version__", env!("CARGO_PKG_VERSION"))?;
            py_module.add("__py_runtime__", py_context.version())?;
            Ok(())
        }
    }

    pub use py_exports::{emit_xml, from_msgpack, parse_xml, tei_rapporteur, to_msgpack};

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
mod tests;
