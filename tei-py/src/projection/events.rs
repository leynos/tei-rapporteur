//! Streaming event projections for Python.

use serde::{Deserialize, Serialize};
use tei_core::BodyBlock;

use super::{PyInline, PyTeiHeader};

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
        /// Optional speaker reference.
        #[serde(skip_serializing_if = "Option::is_none")]
        speaker: Option<String>,
        /// Inline content contained in the utterance.
        content: Vec<PyInline>,
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
                speaker: u.speaker().map(|s| s.as_str().to_owned()),
                content: u.content().iter().cloned().map(PyInline::from).collect(),
            },
        },
        tei_xml::streaming::TeiEvent::DocumentEnd => PyEvent::DocumentEnd,
    }
}
