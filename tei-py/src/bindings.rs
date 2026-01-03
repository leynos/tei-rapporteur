//! `PyO3` glue that surfaces `TeiDocument` helpers to Python callers.

use crate::{TeiDocument, TeiError, emit_title_markup};
use pyo3::exceptions::{PyTypeError, PyValueError};
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
}

pub(crate) mod py_exports {
    // PyO3 expands #[pyfunction] into wrappers with extra ABI parameters; the
    // resulting signatures trip clippy::too_many_arguments but are unavoidable
    // at this FFI boundary, so the lint is locally expected here.
    #![expect(
        clippy::too_many_arguments,
        reason = "PyO3 #[pyfunction] wrappers add ABI parameters the lint counts"
    )]
    use super::{
        Document, PyResult, PyValueError, Python, emit_title_markup, map_serde_error,
        wrap_tei_result,
    };
    use crate::{
        define_py_from_error_wrapper, define_py_from_result_wrapper, define_py_to_error_wrapper,
        define_py_to_result_wrapper, document_from_dict, document_from_msgpack, document_from_xml,
        document_to_dict, document_to_msgpack, document_to_xml, structs::register_structs_module,
    };
    use pyo3::types::PyModuleMethods;
    use pyo3::{
        Bound,
        types::{PyAny, PyModule},
    };
    use pyo3::{pyfunction, pymodule, wrap_pyfunction};

    #[pyfunction(name = "emit_title_markup")]
    pub fn emit_title_markup_py(raw_title: &str) -> PyResult<String> {
        wrap_tei_result(emit_title_markup(raw_title))
    }

    /// Streams TEI events from an XML string as tagged dictionaries.
    ///
    /// The iterator yields domain events (`document_start`, `header`,
    /// `paragraph`, `utterance`, `document_end`). Malformed XML or validation
    /// failures raise [`pyo3::exceptions::PyValueError`] and exhaust the
    /// iterator.
    ///
    /// # Errors
    ///
    /// Returns [`PyValueError`] when parsing fails before exhaustion.
    #[pyfunction(name = "iter_parse")]
    pub fn iter_parse(xml: &str) -> PyResult<crate::streaming::TeiEventIterator> {
        Ok(crate::streaming::iter_parse_py(xml))
    }

    define_py_from_error_wrapper!(
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
        /// use tei_serde::msgpack::to_vec_named;
        /// use tei_core::TeiDocument;
        /// use tei_py::from_msgpack;
        ///
        /// let source = TeiDocument::from_title_str("Wolf 359")?;
        /// let payload = to_vec_named(&source)?;
        /// let document = from_msgpack(&payload)?;
        /// assert_eq!(document.title(), "Wolf 359");
        /// # Ok::<(), Box<dyn std::error::Error>>(())
        /// ```
        fn from_msgpack(bytes: &[u8]) -> Document using document_from_msgpack,
        "invalid MessagePack payload: {error}"
    );

    define_py_to_error_wrapper!(
        /// Serialises a [`Document`] into `MessagePack` bytes.
        ///
        /// # Errors
        ///
        /// Returns [`pyo3::exceptions::PyValueError`] when `MessagePack` encoding fails.
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
        fn to_msgpack(document: &Document) -> Vec<u8> => document_to_msgpack,
        "MessagePack encoding failed: {error}"
    );

    define_py_from_result_wrapper!(
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
        fn parse_xml(xml: &str) -> Document using document_from_xml
    );

    define_py_to_result_wrapper!(
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
        fn emit_xml(document: &Document) -> String => document_to_xml
    );

    /// Constructs a [`Document`] from a JSON-like Python structure.
    ///
    /// # Errors
    ///
    /// Returns [`pyo3::exceptions::PyValueError`] when the payload cannot be
    /// deserialised into a valid [`tei_core::TeiDocument`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pyo3::{Python, types::PyAnyMethods};
    /// use tei_py::{Document, from_dict, to_dict};
    ///
    /// Python::with_gil(|py| {
    ///     let document = Document::try_from_title("Wolf 359")?;
    ///     let payload = to_dict(py, &document)?;
    ///     let round_tripped = from_dict(payload)?;
    ///     assert_eq!(round_tripped.title(), "Wolf 359");
    ///     Ok::<(), pyo3::PyErr>(())
    /// })?;
    /// # Ok::<(), pyo3::PyErr>(())
    /// ```
    #[pyfunction(name = "from_dict")]
    pub fn from_dict(payload: Bound<'_, PyAny>) -> PyResult<Document> {
        document_from_dict(payload)
            .map(Document::from)
            .map_err(map_serde_error)
    }

    /// Serialises a [`Document`] into a Python `dict`/`list` tree.
    ///
    /// # Errors
    ///
    /// Returns [`pyo3::exceptions::PyValueError`] when converting the
    /// document into Python objects fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use pyo3::{Python, types::PyAnyMethods};
    /// use tei_py::{Document, to_dict};
    ///
    /// Python::with_gil(|py| {
    ///     let document = Document::try_from_title("Bridgewater")?;
    ///     let payload = to_dict(py, &document)?;
    ///     let title: String = payload
    ///         .get_item("teiHeader")?
    ///         .get_item("fileDesc")?
    ///         .get_item("title")?
    ///         .extract()?;
    ///     assert_eq!(title, "Bridgewater");
    ///     Ok::<(), pyo3::PyErr>(())
    /// })?;
    /// # Ok::<(), pyo3::PyErr>(())
    /// ```
    #[pyfunction(name = "to_dict")]
    pub fn to_dict<'py>(py: Python<'py>, document: &'py Document) -> PyResult<Bound<'py, PyAny>> {
        document_to_dict(py, document).map_err(map_serde_error)
    }

    /// Registers the `tei_rapporteur` Python module.
    ///
    /// # Errors
    ///
    /// Returns [`pyo3::PyErr`] when registering the module exports fails because the
    /// interpreter rejects one of the additions.
    #[pymodule]
    pub fn tei_rapporteur(py_context: Python<'_>, py_module: &Bound<'_, PyModule>) -> PyResult<()> {
        py_module.add_class::<Document>()?;
        py_module.add_function(wrap_pyfunction!(emit_title_markup_py, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(from_msgpack, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(to_msgpack, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(from_dict, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(to_dict, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(parse_xml, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(emit_xml, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(iter_parse, py_module)?)?;
        register_structs_module(py_context, py_module)?;
        py_module.add("__version__", env!("CARGO_PKG_VERSION"))?;
        py_module.add("__py_runtime__", py_context.version())?;
        Ok(())
    }
}

/// Converts a Rust `Result<T, TeiError>` into a Python-friendly [`PyResult`].
///
/// Successful values are forwarded unchanged, while [`TeiError`] values are
/// rendered via [`to_string`](TeiError::to_string) and wrapped in
/// [`PyValueError`]. This keeps the FFI boundary consistent by mapping Rust
/// domain errors to Python exceptions in one place.
pub(crate) fn wrap_tei_result<T>(result: Result<T, TeiError>) -> PyResult<T> {
    result.map_err(|error| PyValueError::new_err(error.to_string()))
}

fn map_serde_error(error: pyo3_serde::Error) -> PyErr {
    let inner: PyErr = error.into();
    Python::with_gil(|py| {
        if inner.is_instance_of::<PyTypeError>(py) {
            inner
        } else {
            PyValueError::new_err(inner.to_string())
        }
    })
}
