//! Streaming event projections for Python.

use serde::{Deserialize, Serialize};
use tei_core::BodyBlock;

use super::body::py_body_block_from_core;
use super::{PyDivContent, PyInline, PyTeiHeader};

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
        /// Optional `@subtype` attribute.
        #[serde(skip_serializing_if = "Option::is_none")]
        subtype: Option<String>,
        /// Optional heading content.
        #[serde(skip_serializing_if = "Option::is_none")]
        head: Option<super::PyHead>,
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
        tei_xml::streaming::TeiEvent::BodyBlock(block) => py_event_from_body_block(&block),
        tei_xml::streaming::TeiEvent::DocumentEnd => PyEvent::DocumentEnd,
    }
}

/// Converts a body block into the matching [`PyEvent`] variant.
///
/// Paragraph and utterance projection is delegated to the shared body helpers
/// via [`py_body_block_from_core`], then mapped to the corresponding event
/// variant. This eliminates the third copy of the projection logic.
fn py_event_from_body_block(block: &BodyBlock) -> PyEvent {
    use super::PyBodyBlock;

    match py_body_block_from_core(block) {
        PyBodyBlock::Paragraph { xml_id, content } => PyEvent::Paragraph { xml_id, content },
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
        } => PyEvent::Utterance {
            xml_id,
            n,
            speaker,
            source,
            resp,
            cert,
            corresp,
            ana,
            content,
        },
        PyBodyBlock::Div {
            xml_id,
            div_type,
            subtype,
            head,
            content,
        } => PyEvent::Div {
            xml_id,
            div_type,
            subtype,
            head,
            content,
        },
    }
}
