//! `PyO3` glue that surfaces `TeiDocument` helpers to Python callers.

use crate::{TeiDocument, TeiError, emit_title_markup};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use std::ops::Deref;

/// Wrapper around [`TeiDocument`] surfaced to Python.
#[pyclass(module = "tei_rapporteur", name = "Document", from_py_object)]
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

#[path = "document_methods.rs"]
mod document_methods;

pub(crate) mod py_exports {
    //! PyO3-exported functions and classes for the Python module.
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
        document_to_dict, document_to_msgpack, document_to_xml, extract_spoken_segments,
        structs::register_structs_module,
    };
    use pyo3::types::PyModuleMethods;
    use pyo3::{
        Bound, Py,
        pybacked::PyBackedStr,
        types::{PyAny, PyAnyMethods, PyModule},
    };
    use pyo3::{pyfunction, pymodule, wrap_pyfunction};

    const STRUCTS_MODULE_NAME: &str = "tei_rapporteur.structs";
    const SPOKEN_TEXT_SEGMENT_CLASS_NAME: &str = "SpokenTextSegment";
    const SPOKEN_TEXT_SEGMENT_CLASS_CACHE: &str = "_spoken_text_segment_class";

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

    /// Extracts ADR-006 spoken text segments from a TEI XML string.
    ///
    /// XML parsing runs inside `py.detach()`, releasing the GIL while
    /// [`extract_spoken_segments`] performs blocking parsing so other Python
    /// threads may progress. The GIL is reacquired before constructing the
    /// Python segment objects.
    ///
    /// The `xml` argument is taken as a [`PyBackedStr`] so the Python-owned
    /// string buffer stays valid across the GIL release without an extra copy.
    ///
    /// # Errors
    ///
    /// Returns [`PyValueError`] when XML parsing or profile validation fails.
    #[pyfunction(name = "spoken_text_segments")]
    pub fn spoken_text_segments(py: Python<'_>, xml: PyBackedStr) -> PyResult<Vec<Py<PyAny>>> {
        let segments = wrap_tei_result(py.detach(move || extract_spoken_segments(xml.as_str())))?;
        let segment_class = spoken_text_segment_class(py)?;
        segments
            .into_iter()
            .map(|segment| {
                let xml_id = segment.provenance().xml_id().map(str::to_owned);
                segment_class
                    .call1((
                        segment.text().to_owned(),
                        segment.provenance().locator().to_owned(),
                        xml_id,
                    ))
                    .map(Bound::unbind)
            })
            .collect()
    }

    fn spoken_text_segment_class(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
        let sys_modules = py.import("sys")?.getattr("modules")?;
        let structs = sys_modules.get_item(STRUCTS_MODULE_NAME).map_err(|_| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "tei_rapporteur.structs is not registered; \
                 initialise the module before calling spoken_text_segments",
            )
        })?;
        if let Ok(segment_class) = structs.getattr(SPOKEN_TEXT_SEGMENT_CLASS_CACHE) {
            Ok(segment_class)
        } else {
            let segment_class = structs.getattr(SPOKEN_TEXT_SEGMENT_CLASS_NAME)?;
            structs.setattr(SPOKEN_TEXT_SEGMENT_CLASS_CACHE, &segment_class)?;
            Ok(segment_class)
        }
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
        /// use tei_py::{Document, from_msgpack, to_msgpack};
        ///
        /// let source = Document::try_from_title("Wolf 359")
        ///     .expect("fixture title should validate");
        /// let payload = to_msgpack(&source)?;
        /// let document = from_msgpack(&payload)?;
        /// assert_eq!(document.title(), "Wolf 359");
        /// # Ok::<(), pyo3::PyErr>(())
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
        /// let document = Document::try_from_title("Wolf 359")
        ///     .expect("fixture title should validate");
        /// let payload = to_msgpack(&document)?;
        /// let decoded = from_msgpack(&payload)?;
        /// assert_eq!(decoded.title(), "Wolf 359");
        /// # Ok::<(), pyo3::PyErr>(())
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
        /// let document = Document::try_from_title("Wolf 359")
        ///     .expect("fixture title should validate");
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
    /// Python::attach(|py| {
    ///     let document = Document::try_from_title("Wolf 359")
    ///         .expect("fixture title should validate");
    ///     let payload = to_dict(py, &document)?;
    ///     let round_tripped = from_dict(payload)?;
    ///     assert_eq!(round_tripped.title(), "Wolf 359");
    ///     Ok::<(), Box<dyn std::error::Error>>(())
    /// })?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
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
    /// Python::attach(|py| {
    ///     let document = Document::try_from_title("Bridgewater")
    ///         .expect("fixture title should validate");
    ///     let payload = to_dict(py, &document)?;
    ///     let title: String = payload
    ///         .get_item("header")?
    ///         .get_item("file_desc")?
    ///         .get_item("title")?
    ///         .extract()?;
    ///     assert_eq!(title, "Bridgewater");
    ///     Ok::<(), Box<dyn std::error::Error>>(())
    /// })?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
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
        #[cfg(any(test, feature = "test-support"))]
        let _registration_guard =
            crate::test_support::lock_python_module_registration_attached_for_tests(py_context);

        register_tei_rapporteur_module(py_context, py_module)
    }

    pub(crate) fn register_tei_rapporteur_module(
        py_context: Python<'_>,
        py_module: &Bound<'_, PyModule>,
    ) -> PyResult<()> {
        py_module.add_class::<Document>()?;
        py_module.add_function(wrap_pyfunction!(emit_title_markup_py, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(from_msgpack, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(to_msgpack, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(from_dict, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(to_dict, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(parse_xml, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(emit_xml, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(iter_parse, py_module)?)?;
        py_module.add_function(wrap_pyfunction!(spoken_text_segments, py_module)?)?;
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
    Python::attach(|py| {
        if inner.is_instance_of::<PyTypeError>(py) {
            inner
        } else {
            PyValueError::new_err(inner.to_string())
        }
    })
}
