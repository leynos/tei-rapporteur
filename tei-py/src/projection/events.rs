//! Streaming event projections for Python.

use serde::{Deserialize, Serialize};
use tei_core::BodyBlock;

use super::{
    PyDivContent, PyInline, PyTeiHeader, certainty_to_string, pointer_list_to_vec,
    py_div_content_from_core,
};

/// Tagged streaming event union surfaced to Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub(crate) enum PyEvent {
    /// Signals the start of the document stream.
    #[serde(rename = "document_start")]
    DocumentStart,
    /// Carries the parsed header once available.
    #[serde(rename = "header")]
    Header {
        /// Fully projected TEI header.
        header: PyTeiHeader,
    },
    /// Delivers a paragraph body block.
    #[serde(rename = "paragraph")]
    Paragraph {
        /// Optional TEI `xml:id` identifier.
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        /// Inline content contained in the paragraph.
        content: Vec<PyInline>,
    },
    /// Delivers an utterance body block.
    #[serde(rename = "utterance")]
    Utterance {
        /// Optional TEI `xml:id` identifier.
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        /// Optional TEI `@n` label.
        #[serde(skip_serializing_if = "Option::is_none")]
        n: Option<String>,
        /// Optional speaker reference.
        #[serde(skip_serializing_if = "Option::is_none")]
        speaker: Option<String>,
        /// Optional source pointer list.
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        source: Vec<String>,
        /// Optional responsibility pointer list.
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        resp: Vec<String>,
        /// Optional certainty token.
        #[serde(skip_serializing_if = "Option::is_none")]
        cert: Option<String>,
        /// Optional correspondence pointer list.
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        corresp: Vec<String>,
        /// Optional analysis pointer list.
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        ana: Vec<String>,
        /// Inline content contained in the utterance.
        content: Vec<PyInline>,
    },
    /// Delivers a division body block.
    #[serde(rename = "div")]
    Div {
        /// Optional TEI `xml:id` identifier.
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        /// Required `@type` attribute.
        div_type: String,
        /// Content of the division.
        content: Vec<PyDivContent>,
    },
    /// Signals the end of the document stream.
    #[serde(rename = "document_end")]
    DocumentEnd,
}

/// Maps a core streaming event to its Python projection.
pub fn py_event_from_core(event: tei_xml::streaming::TeiEvent) -> PyEvent {
    match event {
        tei_xml::streaming::TeiEvent::DocumentStart => PyEvent::DocumentStart,
        tei_xml::streaming::TeiEvent::Header(header) => PyEvent::Header {
            header: PyTeiHeader::from(&header),
        },
        tei_xml::streaming::TeiEvent::BodyBlock(block) => match block {
            BodyBlock::Paragraph(p) => PyEvent::Paragraph {
                xml_id: p.id().map(|id| id.as_str().to_owned()),
                content: p.content().iter().cloned().map(PyInline::from).collect(),
            },
            BodyBlock::Utterance(u) => PyEvent::Utterance {
                xml_id: u.id().map(|id| id.as_str().to_owned()),
                n: u.number().map(str::to_owned),
                speaker: u.speaker().map(|s| s.as_str().to_owned()),
                source: pointer_list_to_vec(u.source()),
                resp: pointer_list_to_vec(u.resp()),
                cert: certainty_to_string(u.cert()),
                corresp: pointer_list_to_vec(u.corresp()),
                ana: pointer_list_to_vec(u.ana()),
                content: u.content().iter().cloned().map(PyInline::from).collect(),
            },
            BodyBlock::Div(div) => PyEvent::Div {
                xml_id: div.id().map(|id| id.as_str().to_owned()),
                div_type: div.div_type().to_owned(),
                content: div
                    .content()
                    .iter()
                    .map(py_div_content_from_core)
                    .collect(),
            },
        },
        tei_xml::streaming::TeiEvent::DocumentEnd => PyEvent::DocumentEnd,
    }
}
