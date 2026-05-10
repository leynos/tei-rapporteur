//! Internal parser frame types for spoken-text extraction.

use std::collections::BTreeMap;

use tei_core::SpokenTextNormalizer;

/// XML element stack frame with locator state.
#[derive(Clone, Debug)]
pub(crate) struct ElementFrame {
    /// Local element name.
    pub(crate) name: String,
    /// Stable XPath-like locator for this element.
    pub(crate) locator: String,
    /// Per-child-name counters used to assign locator indexes.
    pub(crate) child_counts: BTreeMap<String, usize>,
    /// Whether this element and descendants are excluded from speech.
    pub(crate) is_excluded: bool,
}

impl ElementFrame {
    /// Builds a stack frame for a newly entered XML element.
    pub(crate) const fn new(name: String, locator: String, is_excluded: bool) -> Self {
        Self {
            name,
            locator,
            child_counts: BTreeMap::new(),
            is_excluded,
        }
    }
}

/// Type of active spoken segment currently being collected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SegmentKind {
    /// A block element such as `<p>`, `<ab>`, `<l>`, or standalone `<seg>`.
    Block,
    /// A direct `<u>` utterance that may be suppressed by child spoken blocks.
    Utterance,
}

/// Active spoken segment accumulator.
#[derive(Clone, Debug)]
pub(crate) struct ActiveSegment {
    /// Segment kind.
    pub(crate) kind: SegmentKind,
    /// Local source element name.
    pub(crate) name: String,
    /// Stable source locator.
    pub(crate) locator: String,
    /// Optional source `xml:id`.
    pub(crate) xml_id: Option<String>,
    /// Normalized text accumulator.
    pub(crate) normalizer: SpokenTextNormalizer,
    /// Whether this segment has child spoken blocks that own the text.
    pub(crate) has_child_spoken_block: bool,
}

impl ActiveSegment {
    /// Builds an active spoken segment accumulator.
    pub(crate) fn new(
        kind: SegmentKind,
        name: String,
        locator: String,
        xml_id: Option<String>,
    ) -> Self {
        Self {
            kind,
            name,
            locator,
            xml_id,
            normalizer: SpokenTextNormalizer::default(),
            has_child_spoken_block: false,
        }
    }
}
