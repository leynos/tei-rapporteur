//! Python-facing projection types with tagged unions for inline content and
//! body blocks.
//!
//! The core TEI model uses untagged Serde enums for inline content, which
//! prevents Python from defining fully typed `msgspec.Struct` unions. This
//! module introduces a parallel, internally tagged representation used at the
//! FFI boundary. All dictionary and `MessagePack` exchange now flows through
//! these projection types so that Python callers receive and submit stable,
//! unambiguous payloads.

mod annotation;
mod body;
mod events;
mod header;

pub(crate) use annotation::{
    PyStandOff, apply_optional_pointer_list, certainty_from_option, certainty_to_string,
    pointer_list_to_vec,
};
use body::{core_block_from_py, py_body_block_from_core};
use header::PyTeiHeader;
use serde::{Deserialize, Serialize};
use tei_core::{BodyBlock, Inline, Pause, TeiBody, TeiDocument, TeiError, TeiHeader, TeiText};
use tei_serde::json::Value;

/// Tagged inline content for Python consumption.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub(crate) enum PyInline {
    #[serde(rename = "text")]
    Text { value: String },
    #[serde(rename = "hi")]
    Hi {
        #[serde(skip_serializing_if = "Option::is_none")]
        rend: Option<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "pause")]
    Pause {
        #[serde(skip_serializing_if = "Option::is_none")]
        dur: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
    },
}

/// Tagged body block union (paragraph, utterance, or division) for Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub(crate) enum PyBodyBlock {
    #[serde(rename = "paragraph")]
    Paragraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "utterance")]
    Utterance {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        n: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        speaker: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        source: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        resp: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cert: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        corresp: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        ana: Vec<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "div")]
    Div {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        div_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subtype: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        head: Option<PyHead>,
        content: Vec<PyDivContent>,
    },
}

/// Tagged content within a division for Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub(crate) enum PyDivContent {
    #[serde(rename = "paragraph")]
    Paragraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "utterance")]
    Utterance {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        n: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        speaker: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        source: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        resp: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cert: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        corresp: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        ana: Vec<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "list")]
    List {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        items: Vec<PyItem>,
    },
    #[serde(rename = "div")]
    Div {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        div_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subtype: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        head: Option<PyHead>,
        content: Vec<PyDivContent>,
    },
}

/// A list item projected for Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    xml_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    corresp: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<PyLabel>,
    content: Vec<PyInline>,
}

/// A label prefix projected for Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyLabel {
    content: Vec<PyInline>,
}

/// A division heading projected for Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyHead {
    content: Vec<PyInline>,
}

/// Python projection of the TEI body.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyTeiBody {
    #[serde(default)]
    pub(crate) blocks: Vec<PyBodyBlock>,
}

/// Python projection of the `<text>` element.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyTeiText {
    pub(crate) body: PyTeiBody,
}

/// Python projection of the full TEI document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PyTeiDocument {
    pub(crate) header: PyTeiHeader,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) stand_off: Option<PyStandOff>,
    pub(crate) text: PyTeiText,
}

impl From<&TeiBody> for PyTeiBody {
    fn from(body: &TeiBody) -> Self {
        let blocks = body.blocks().iter().map(py_body_block_from_core).collect();
        Self { blocks }
    }
}

impl TryFrom<PyTeiBody> for TeiBody {
    type Error = TeiError;

    fn try_from(value: PyTeiBody) -> Result<Self, Self::Error> {
        let mut body = Self::default();
        for block in value.blocks {
            match core_block_from_py(block)? {
                BodyBlock::Paragraph(paragraph) => body.push_paragraph(paragraph),
                BodyBlock::Utterance(utterance) => body.push_utterance(utterance),
                BodyBlock::Div(div) => body.push_div(div),
            }
        }
        Ok(body)
    }
}

impl From<&TeiDocument> for PyTeiDocument {
    fn from(document: &TeiDocument) -> Self {
        Self {
            header: PyTeiHeader::from(document.header()),
            stand_off: document.stand_off().map(PyStandOff::from),
            text: PyTeiText {
                body: PyTeiBody::from(document.text().body()),
            },
        }
    }
}

impl TryFrom<PyTeiDocument> for TeiDocument {
    type Error = TeiError;

    fn try_from(value: PyTeiDocument) -> Result<Self, Self::Error> {
        let header = TeiHeader::try_from(value.header)?;
        let body = TeiBody::try_from(value.text.body)?;
        let document = Self::new(header, TeiText::new(body));
        match value.stand_off {
            Some(stand_off) => Ok(document.with_stand_off(stand_off.try_into()?)),
            None => Ok(document),
        }
    }
}

impl From<Inline> for PyInline {
    fn from(value: Inline) -> Self {
        match value {
            Inline::Text(text) => Self::Text { value: text },
            Inline::Hi(hi) => Self::Hi {
                rend: hi.rend().map(str::to_owned),
                content: hi.content().iter().cloned().map(Self::from).collect(),
            },
            Inline::Pause(pause) => Self::Pause {
                dur: pause.duration().map(str::to_owned),
                kind: pause.kind().map(str::to_owned),
            },
        }
    }
}

fn inline_from_py(inline_value: PyInline) -> Result<Inline, TeiError> {
    match inline_value {
        PyInline::Text { value } => Ok(Inline::Text(value)),
        PyInline::Hi { rend, content } => hi_from_py(rend, content),
        PyInline::Pause { dur, kind } => Ok(pause_from_py(dur, kind)),
    }
}

/// Converts a projected highlight into a core `Inline::Hi`.
fn hi_from_py(rend: Option<String>, content: Vec<PyInline>) -> Result<Inline, TeiError> {
    let converted_values: Result<Vec<Inline>, TeiError> =
        content.into_iter().map(inline_from_py).collect();
    let converted_inlines = converted_values?;
    let hi = match rend {
        Some(r) => tei_core::Hi::try_with_rend(r, converted_inlines)?,
        None => tei_core::Hi::try_new(converted_inlines)?,
    };
    Ok(Inline::Hi(hi))
}

/// Converts a projected pause into a core `Inline::Pause`.
fn pause_from_py(dur: Option<String>, kind: Option<String>) -> Inline {
    let mut pause = Pause::new();
    if let Some(duration) = dur {
        pause.set_duration(duration);
    }
    if let Some(classification) = kind {
        pause.set_kind(classification);
    }
    Inline::Pause(pause)
}

/// Converts a core TEI document into a projection `Value` for Python exchange.
///
/// Primarily used by integration tests and Python fixtures; not part of the
/// stable public surface.
///
/// # Errors
///
/// Returns a JSON serialization error when the projection cannot be rendered.
pub fn document_to_value(document: &TeiDocument) -> Result<Value, tei_serde::serde_json::Error> {
    let projection = PyTeiDocument::from(document);
    tei_serde::json::to_value(&projection)
}

/// Errors surfaced during projection deserialization.
///
/// These distinguish JSON decoding problems from TEI validation failures so
/// callers can report precise failure causes back to Python.
#[derive(thiserror::Error, Debug)]
pub enum ProjectionError {
    /// JSON decoding failed.
    #[error("invalid TEI projection: {0}")]
    Serde(#[from] tei_serde::serde_json::Error),
    /// TEI conversion or validation failed after successful decoding.
    #[error("invalid TEI document: {0}")]
    Tei(#[from] TeiError),
}

/// Converts a projection `Value` into a core document.
///
/// This is exposed for integration tests and fixtures that round-trip through
/// the Python projection shape; it is not a stable public API.
///
/// # Errors
///
/// Returns a [`ProjectionError`] when the payload is not a valid projection or
/// when conversion back to the core TEI model fails.
pub fn value_to_document(value: &Value) -> Result<TeiDocument, ProjectionError> {
    let projection: PyTeiDocument = tei_serde::json::from_value(value.clone())?;
    TeiDocument::try_from(projection).map_err(ProjectionError::from)
}

pub(crate) use events::py_event_from_core;
