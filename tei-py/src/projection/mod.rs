//! Python-facing projection types with tagged unions for inline content and
//! body blocks.
//!
//! The core TEI model uses untagged Serde enums for inline content, which
//! prevents Python from defining fully typed `msgspec.Struct` unions. This
//! module introduces a parallel, internally tagged representation used at the
//! FFI boundary. All dictionary and `MessagePack` exchange now flows through
//! these projection types so that Python callers receive and submit stable,
//! unambiguous payloads.

mod events;
mod header;

use header::PyTeiHeader;
use serde::{Deserialize, Serialize};
use tei_core::{
    BodyBlock, BodyContentError, Inline, P, Pause, TeiBody, TeiDocument, TeiError, TeiHeader,
    TeiText, Utterance,
};
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

/// Tagged body block union (paragraph or utterance) for Python.
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
        speaker: Option<String>,
        content: Vec<PyInline>,
    },
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
    pub(crate) text: PyTeiText,
}

impl From<&TeiBody> for PyTeiBody {
    fn from(body: &TeiBody) -> Self {
        let blocks = body.blocks().iter().map(py_body_block_from_core).collect();
        Self { blocks }
    }
}

impl TryFrom<PyTeiBody> for TeiBody {
    type Error = BodyContentError;

    fn try_from(value: PyTeiBody) -> Result<Self, Self::Error> {
        let mut body = Self::default();
        for block in value.blocks {
            match core_block_from_py(block)? {
                BodyBlock::Paragraph(paragraph) => body.push_paragraph(paragraph),
                BodyBlock::Utterance(utterance) => body.push_utterance(utterance),
            }
        }
        Ok(body)
    }
}

impl From<&TeiDocument> for PyTeiDocument {
    fn from(document: &TeiDocument) -> Self {
        Self {
            header: PyTeiHeader::from(document.header()),
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
        let body = TeiBody::try_from(value.text.body).map_err(TeiError::from)?;
        Ok(Self::new(header, TeiText::new(body)))
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

fn inline_from_py(inline_value: PyInline) -> Result<Inline, BodyContentError> {
    match inline_value {
        PyInline::Text { value } => Ok(Inline::Text(value)),
        PyInline::Hi { rend, content } => {
            let converted_values: Result<Vec<Inline>, BodyContentError> =
                content.into_iter().map(inline_from_py).collect();
            let converted_inlines = converted_values?;
            let hi = match rend {
                Some(r) => tei_core::Hi::try_with_rend(r, converted_inlines)?,
                None => tei_core::Hi::try_new(converted_inlines)?,
            };
            Ok(Inline::Hi(hi))
        }
        PyInline::Pause { dur, kind } => {
            let mut pause = Pause::new();
            if let Some(duration) = dur {
                pause.set_duration(duration);
            }
            if let Some(classification) = kind {
                pause.set_kind(classification);
            }
            Ok(Inline::Pause(pause))
        }
    }
}

fn py_body_block_from_core(block: &BodyBlock) -> PyBodyBlock {
    match block {
        BodyBlock::Paragraph(p) => PyBodyBlock::Paragraph {
            xml_id: p.id().map(|id| id.as_str().to_owned()),
            content: p.content().iter().cloned().map(PyInline::from).collect(),
        },
        BodyBlock::Utterance(u) => PyBodyBlock::Utterance {
            xml_id: u.id().map(|id| id.as_str().to_owned()),
            speaker: u.speaker().map(|s| s.as_str().to_owned()),
            content: u.content().iter().cloned().map(PyInline::from).collect(),
        },
    }
}

fn core_block_from_py(block: PyBodyBlock) -> Result<BodyBlock, BodyContentError> {
    match block {
        PyBodyBlock::Paragraph { xml_id, content } => {
            let mut paragraph = P::from_inline(
                content
                    .into_iter()
                    .map(inline_from_py)
                    .collect::<Result<Vec<_>, _>>()?,
            )?;
            if let Some(id) = xml_id {
                paragraph.set_id(id)?;
            }
            Ok(BodyBlock::Paragraph(paragraph))
        }
        PyBodyBlock::Utterance {
            xml_id,
            speaker,
            content,
        } => {
            let mut utterance = Utterance::from_inline(
                speaker.as_deref(),
                content
                    .into_iter()
                    .map(inline_from_py)
                    .collect::<Result<Vec<_>, _>>()?,
            )?;
            if let Some(id) = xml_id {
                utterance.set_id(id)?;
            }
            Ok(BodyBlock::Utterance(utterance))
        }
    }
}

/// Converts a core TEI document into a projection `Value` for Python exchange.
///
/// Primarily used by integration tests and Python fixtures; not part of the
/// stable public surface.
///
/// # Errors
///
/// Returns a JSON serialisation error when the projection cannot be rendered.
pub fn document_to_value(document: &TeiDocument) -> Result<Value, tei_serde::serde_json::Error> {
    let projection = PyTeiDocument::from(document);
    tei_serde::json::to_value(&projection)
}

/// Errors surfaced during projection deserialisation.
#[derive(thiserror::Error, Debug)]
pub enum ProjectionError {
    /// JSON decoding failed.
    #[error("invalid TEI projection: {0}")]
    Serde(#[from] tei_serde::serde_json::Error),
    /// TEI validation failed after successful decoding.
    #[error("invalid TEI body: {0}")]
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
