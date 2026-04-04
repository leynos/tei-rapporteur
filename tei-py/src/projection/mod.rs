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
mod events;
mod header;

pub(crate) use annotation::{
    PyStandOff, apply_optional_pointer_list, certainty_from_option, certainty_to_string,
    pointer_list_to_vec,
};
use header::PyTeiHeader;
use serde::{Deserialize, Serialize};
use tei_core::{
    BodyBlock, Div, DivContent, Inline, Item, Label, List, P, Pause, PointerList, TeiBody,
    TeiDocument, TeiError, TeiHeader, TeiText, Utterance,
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
        content: Vec<PyInline>,
    },
    #[serde(rename = "list")]
    List {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        items: Vec<PyItem>,
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
        PyInline::Hi { rend, content } => {
            let converted_values: Result<Vec<Inline>, TeiError> =
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
        BodyBlock::Utterance(u) => py_utterance_body_block(u),
        BodyBlock::Div(div) => PyBodyBlock::Div {
            xml_id: div.id().map(|id| id.as_str().to_owned()),
            div_type: div.div_type().to_owned(),
            content: div.content().iter().map(py_div_content_from_core).collect(),
        },
    }
}

fn py_utterance_body_block(u: &Utterance) -> PyBodyBlock {
    PyBodyBlock::Utterance {
        xml_id: u.id().map(|id| id.as_str().to_owned()),
        n: u.number().map(str::to_owned),
        speaker: u.speaker().map(|s| s.as_str().to_owned()),
        source: pointer_list_to_vec(u.source()),
        resp: pointer_list_to_vec(u.resp()),
        cert: certainty_to_string(u.cert()),
        corresp: pointer_list_to_vec(u.corresp()),
        ana: pointer_list_to_vec(u.ana()),
        content: u.content().iter().cloned().map(PyInline::from).collect(),
    }
}

fn py_div_content_from_core(content: &DivContent) -> PyDivContent {
    match content {
        DivContent::Paragraph(p) => PyDivContent::Paragraph {
            xml_id: p.id().map(|id| id.as_str().to_owned()),
            content: p.content().iter().cloned().map(PyInline::from).collect(),
        },
        DivContent::Utterance(u) => PyDivContent::Utterance {
            xml_id: u.id().map(|id| id.as_str().to_owned()),
            n: u.number().map(str::to_owned),
            speaker: u.speaker().map(|s| s.as_str().to_owned()),
            content: u.content().iter().cloned().map(PyInline::from).collect(),
        },
        DivContent::List(list) => PyDivContent::List {
            xml_id: list.id().map(|id| id.as_str().to_owned()),
            items: list.items().iter().map(py_item_from_core).collect(),
        },
    }
}

fn py_item_from_core(item: &Item) -> PyItem {
    PyItem {
        xml_id: item.id().map(|id| id.as_str().to_owned()),
        n: item.n().map(str::to_owned),
        corresp: pointer_list_to_vec(item.corresp()),
        label: item.label().map(py_label_from_core),
        content: item.content().iter().cloned().map(PyInline::from).collect(),
    }
}

fn py_label_from_core(label: &Label) -> PyLabel {
    PyLabel {
        content: label.content().iter().cloned().map(PyInline::from).collect(),
    }
}

fn core_block_from_py(block: PyBodyBlock) -> Result<BodyBlock, TeiError> {
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
            n,
            speaker,
            source,
            resp,
            cert,
            corresp,
            ana,
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
            if let Some(number) = n {
                utterance.set_number(number);
            }
            apply_optional_pointer_list(&mut utterance, source, Utterance::set_source)?;
            apply_optional_pointer_list(&mut utterance, resp, Utterance::set_resp)?;
            if let Some(certainty) = certainty_from_option(cert)? {
                utterance.set_cert(certainty);
            }
            apply_optional_pointer_list(&mut utterance, corresp, Utterance::set_corresp)?;
            apply_optional_pointer_list(&mut utterance, ana, Utterance::set_ana)?;
            Ok(BodyBlock::Utterance(utterance))
        }
        PyBodyBlock::Div {
            xml_id,
            div_type,
            content,
        } => {
            let mut div = Div::new(div_type)?;
            if let Some(id) = xml_id {
                div.set_id(id)?;
            }
            for py_content in content {
                match core_div_content_from_py(py_content)? {
                    DivContent::Paragraph(p) => div.push_paragraph(p),
                    DivContent::Utterance(u) => div.push_utterance(u),
                    DivContent::List(l) => div.push_list(l),
                }
            }
            Ok(BodyBlock::Div(div))
        }
    }
}

fn core_div_content_from_py(content: PyDivContent) -> Result<DivContent, TeiError> {
    match content {
        PyDivContent::Paragraph { xml_id, content } => {
            let mut paragraph = P::from_inline(
                content
                    .into_iter()
                    .map(inline_from_py)
                    .collect::<Result<Vec<_>, _>>()?,
            )?;
            if let Some(id) = xml_id {
                paragraph.set_id(id)?;
            }
            Ok(DivContent::Paragraph(paragraph))
        }
        PyDivContent::Utterance {
            xml_id,
            n,
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
            if let Some(number) = n {
                utterance.set_number(number);
            }
            Ok(DivContent::Utterance(utterance))
        }
        PyDivContent::List { xml_id, items } => {
            let core_items: Vec<Item> = items
                .into_iter()
                .map(core_item_from_py)
                .collect::<Result<Vec<_>, _>>()?;
            let mut list = List::new(core_items);
            if let Some(id) = xml_id {
                list.set_id(id)?;
            }
            Ok(DivContent::List(list))
        }
    }
}

fn core_item_from_py(item: PyItem) -> Result<Item, TeiError> {
    let inlines: Vec<Inline> = item
        .content
        .into_iter()
        .map(inline_from_py)
        .collect::<Result<Vec<_>, _>>()?;
    let mut core_item = Item::new(inlines)?;
    if let Some(id) = item.xml_id {
        core_item.set_id(id)?;
    }
    if let Some(n_value) = item.n {
        core_item.set_n(n_value)?;
    }
    if !item.corresp.is_empty() {
        core_item.set_corresp(PointerList::parse_attribute(item.corresp.join(" "))?);
    }
    if let Some(py_label) = item.label {
        let label_inlines: Vec<Inline> = py_label
            .content
            .into_iter()
            .map(inline_from_py)
            .collect::<Result<Vec<_>, _>>()?;
        core_item.set_label(Label::new(label_inlines)?);
    }
    Ok(core_item)
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
