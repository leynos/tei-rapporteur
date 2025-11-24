//! `PyO3` bindings and helper functions exposed to Python callers.
//!
//! The crate surfaces the `tei_rapporteur` module, offering a lightweight
//! `Document` wrapper that delegates validation to the Rust core. Python gets
//! mirrored title helpers (`Document.emit_title_markup`, module-level
//! `emit_title_markup`), a `MessagePack` bridge (`from_msgpack`, `to_msgpack`),
//! dictionary helpers (`from_dict`, `to_dict`) backed by `pyo3-serde`, and XML
//! bindings (`parse_xml`, `emit_xml`) that forward directly to `tei-xml`. Rust
//! callers continue to reuse the same helpers without the `PyO3` glue, keeping
//! validation logic centralised.

use pyo3::types::PyAny;
use pyo3::{Bound, Python};
use pyo3_serde::{from_pyobject, to_pyobject};
use rmp_serde::{decode::Error as MsgpackDecodeError, encode::Error as MsgpackEncodeError};
use tei_core::{TeiDocument, TeiError};
use tei_xml::{
    emit_xml as emit_document_xml, parse_xml as parse_document_xml, serialize_document_title,
};

macro_rules! define_conversion_pair {
    (
        from $from_fn:ident($from_arg:ident : $from_ty:ty) -> $from_err:ty { $from_body:expr };
        to $to_fn:ident($to_arg:ident : $to_ty:ty) -> $to_ret:ty, $to_err:ty { $to_body:expr }
    ) => {
        fn $from_fn($from_arg: $from_ty) -> Result<TeiDocument, $from_err> {
            $from_body
        }

        fn $to_fn($to_arg: $to_ty) -> Result<$to_ret, $to_err> {
            $to_body
        }
    };
}

macro_rules! define_py_from_error_wrapper {
    ($(#[$meta:meta])* fn $py_name:ident($param:ident : $ty:ty) -> Document using $inner:ident, $fmt:expr) => {
        $(#[$meta])*
        #[pyfunction]
        pub fn $py_name($param: $ty) -> PyResult<Document> {
            $inner($param)
                .map(Document::from)
                .map_err(|error| PyValueError::new_err(format!($fmt, error = error)))
        }
    };
}

macro_rules! define_py_to_error_wrapper {
    ($(#[$meta:meta])* fn $py_name:ident($param:ident : &$ty:ty) -> $ret:ty => $inner:ident, $fmt:expr) => {
        $(#[$meta])*
        #[pyfunction]
        pub fn $py_name($param: &$ty) -> PyResult<$ret> {
            $inner($param).map_err(|error| PyValueError::new_err(format!($fmt, error = error)))
        }
    };
}

macro_rules! define_py_from_result_wrapper {
    ($(#[$meta:meta])* fn $py_name:ident($param:ident : $ty:ty) -> Document using $inner:ident) => {
        $(#[$meta])*
        #[pyfunction]
        pub fn $py_name($param: $ty) -> PyResult<Document> {
            wrap_tei_result($inner($param)).map(Document::from)
        }
    };
}

macro_rules! define_py_to_result_wrapper {
    ($(#[$meta:meta])* fn $py_name:ident($param:ident : &$ty:ty) -> $ret:ty => $inner:ident) => {
        $(#[$meta])*
        #[pyfunction]
        pub fn $py_name($param: &$ty) -> PyResult<$ret> {
            wrap_tei_result($inner($param))
        }
    };
}

pub use bindings::Document;
pub use bindings::py_exports::{
    emit_xml, from_dict, from_msgpack, parse_xml, tei_rapporteur, to_dict, to_msgpack,
};

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

define_conversion_pair! {
    from document_from_msgpack(bytes: &[u8]) -> MsgpackDecodeError { rmp_serde::from_slice(bytes) };
    to document_to_msgpack(document: &TeiDocument) -> Vec<u8>, MsgpackEncodeError { rmp_serde::to_vec_named(document) }
}

define_conversion_pair! {
    from document_from_xml(xml: &str) -> TeiError { parse_document_xml(xml) };
    to document_to_xml(document: &TeiDocument) -> String, TeiError { emit_document_xml(document) }
}

fn document_from_dict(payload: Bound<'_, PyAny>) -> Result<TeiDocument, pyo3_serde::Error> {
    from_pyobject(payload)
}

fn document_to_dict<'py>(
    py: Python<'py>,
    document: &TeiDocument,
) -> Result<Bound<'py, PyAny>, pyo3_serde::Error> {
    to_pyobject(py, document)
}

mod bindings {
    //! `PyO3` glue that surfaces `TeiDocument` helpers to Python callers.

    use super::{TeiDocument, TeiError, emit_title_markup};
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
        #![allow(
            unsafe_op_in_unsafe_fn,
            reason = "PyO3 #[pymethods] glue performs unavoidable unsafe argument extraction"
        )]
        #![allow(
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

    pub(crate) mod py_exports {
        #![allow(
            unsafe_op_in_unsafe_fn,
            reason = "PyO3 #[pyfunction] glue performs unavoidable unsafe argument extraction"
        )]
        #![allow(
            clippy::too_many_arguments,
            reason = "PyO3 injects abi parameters into #[pyfunction] signatures"
        )]
        #![allow(
            clippy::useless_conversion,
            reason = "PyO3 wraps #[pyfunction] returns in conversion helpers"
        )]
        #![allow(
            clippy::shadow_reuse,
            reason = "PyO3 #[pymodule] expansion reuses the module parameter names"
        )]

        use super::{Document, PyResult, PyValueError, Python, emit_title_markup, wrap_tei_result};
        use crate::{
            document_from_dict, document_from_msgpack, document_from_xml, document_to_dict,
            document_to_msgpack, document_to_xml,
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
            fn from_msgpack(bytes: &[u8]) -> Document using document_from_msgpack,
            "invalid MessagePack payload: {error}"
        );

        define_py_to_error_wrapper!(
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
            fn to_msgpack(document: &Document) -> Vec<u8> => document_to_msgpack,
            "MessagePack encoding failed: {error}"
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
        /// use pyo3::Python;
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
                .map_err(|error| {
                    PyValueError::new_err(format!("invalid dictionary payload: {error}"))
                })
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
        pub fn to_dict<'py>(
            py: Python<'py>,
            document: &'py Document,
        ) -> PyResult<Bound<'py, PyAny>> {
            document_to_dict(py, document).map_err(|error| {
                PyValueError::new_err(format!("dictionary encoding failed: {error}"))
            })
        }

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
            py_module.add_function(wrap_pyfunction!(from_dict, py_module)?)?;
            py_module.add_function(wrap_pyfunction!(to_dict, py_module)?)?;
            py_module.add_function(wrap_pyfunction!(parse_xml, py_module)?)?;
            py_module.add_function(wrap_pyfunction!(emit_xml, py_module)?)?;
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
    fn wrap_tei_result<T>(result: Result<T, TeiError>) -> PyResult<T> {
        result.map_err(|error| PyValueError::new_err(error.to_string()))
    }
}

#[cfg(test)]
mod tests;
