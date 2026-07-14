//! `PyO3` bindings and helper functions exposed to Python callers.
//!
//! The crate surfaces the `tei_rapporteur` module, offering a lightweight
//! `Document` wrapper that delegates validation to the Rust core. Python gets
//! mirrored title helpers (`Document.emit_title_markup`, module-level
//! `emit_title_markup`), a `MessagePack` bridge (`from_msgpack`, `to_msgpack`),
//! dictionary helpers (`from_dict`, `to_dict`) backed by `pyo3-serde`, and XML
//! bindings (`parse_xml`, `emit_xml`) that forward directly to `tei-xml`. The
//! module also exposes a `structs` submodule containing `msgspec.Struct`
//! projections of the TEI data model so Python callers can decode `MessagePack`
//! directly into typed objects. Rust callers continue to reuse the same
//! helpers without the `PyO3` glue, keeping validation logic centralized.

use projection::PyTeiDocument;
use pyo3::types::PyAny;
use pyo3::{Bound, Python};
use pyo3_serde::{from_pyobject, to_pyobject};
use serde::de::Error as DeError;
use tei_core::{SpokenTextSegment, TeiDocument, TeiError};
use tei_serde::msgpack::{
    MsgpackDecodeError, MsgpackEncodeError, from_slice as msgpack_from_slice, to_vec_named,
};
use tei_xml::{
    emit_xml as emit_document_xml, parse_xml as parse_document_xml, serialize_document_title,
    spoken_text_segments as extract_spoken_text_segments,
};

mod macros;
use macros::{
    define_conversion_pair, define_py_from_error_wrapper, define_py_from_result_wrapper,
    define_py_to_error_wrapper, define_py_to_result_wrapper,
};

mod bindings;
#[cfg(test)]
mod bindings_test_support;
pub mod projection;
mod streaming;
mod structs;
/// Test-only infrastructure for Python embedding and `msgspec` bootstrapping.
///
/// Exposes [`test_support::bootstrap_msgspec`] and
/// [`test_support::with_python`] to crate unit tests under `cfg(test)` and to
/// feature-gated integration and BDD tests under `feature = "test-support"`.
/// The module remains absent from production wheel builds.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub use bindings::Document;
pub use bindings::py_exports::{
    emit_xml, from_dict, from_msgpack, iter_parse, parse_xml, spoken_text_segments, tei_rapporteur,
    to_dict, to_msgpack,
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

/// Extracts spoken text segments from a complete TEI XML document.
///
/// # Errors
///
/// Returns [`TeiError`] when XML parsing or profile validation fails.
pub fn extract_spoken_segments(xml: &str) -> Result<Vec<SpokenTextSegment>, TeiError> {
    extract_spoken_text_segments(xml)
}

fn document_from_msgpack(bytes: &[u8]) -> Result<TeiDocument, MsgpackDecodeError> {
    let projection: PyTeiDocument = msgpack_from_slice(bytes)?;
    TeiDocument::try_from(projection).map_err(|error| MsgpackDecodeError::Syntax(error.to_string()))
}

fn document_to_msgpack(document: &TeiDocument) -> Result<Vec<u8>, MsgpackEncodeError> {
    let projection = PyTeiDocument::from(document);
    to_vec_named(&projection)
}

define_conversion_pair! {
    from document_from_xml(xml: &str) -> TeiError { parse_document_xml(xml) };
    to document_to_xml(document: &TeiDocument) -> String, TeiError { emit_document_xml(document) }
}

pub(crate) fn document_from_dict(
    payload: Bound<'_, PyAny>,
) -> Result<TeiDocument, pyo3_serde::Error> {
    let projection: PyTeiDocument = from_pyobject(payload)?;
    TeiDocument::try_from(projection).map_err(|error| pyo3_serde::Error::custom(error.to_string()))
}

pub(crate) fn document_to_dict<'py>(
    py: Python<'py>,
    document: &TeiDocument,
) -> Result<Bound<'py, PyAny>, pyo3_serde::Error> {
    let projection = PyTeiDocument::from(document);
    to_pyobject(py, &projection)
}

#[cfg(test)]
mod tests;
