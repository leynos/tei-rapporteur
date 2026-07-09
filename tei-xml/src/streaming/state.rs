//! Parser state machine for incremental TEI parsing.
//!
//! The [`ParserState`] enum tracks the current parsing context, enabling the
//! pull parser to handle nested elements correctly and yield events at
//! appropriate boundaries.

use tei_core::{DivContent, Head, Inline, Item, Label};

/// Raw TEI utterance attributes captured during streaming parsing.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RawUtteranceAttrs {
    /// Optional `xml:id` attribute value.
    pub id: Option<String>,
    /// Optional free-form `@n` label.
    pub n: Option<String>,
    /// Optional `who` attribute (speaker reference).
    pub who: Option<String>,
    /// Optional `source` pointer list.
    pub source: Option<String>,
    /// Optional `resp` pointer list.
    pub resp: Option<String>,
    /// Optional `cert` token.
    pub cert: Option<String>,
    /// Optional `corresp` pointer list.
    pub corresp: Option<String>,
    /// Optional `ana` pointer list.
    pub ana: Option<String>,
}

/// Internal parsing state for the streaming parser.
///
/// The state machine tracks where in the document structure the parser
/// currently resides, enabling correct event emission and validation.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ParserState {
    /// Initial state before any parsing has occurred.
    #[default]
    Initial,

    /// Emitted `DocumentStart`, now waiting for `<TEI>` root element.
    AwaitingRoot,

    /// Inside `<TEI>`, waiting for `<teiHeader>`.
    AwaitingHeader,

    /// Inside `<teiHeader>`, accumulating header content.
    ///
    /// The `depth` field tracks nesting level to detect `</teiHeader>`.
    InHeader {
        /// Current nesting depth within the header (1 = direct child of header).
        depth: usize,
        /// Accumulated raw XML bytes for the header section.
        buffer: Vec<u8>,
    },

    /// Header parsed, waiting for `<text>` element.
    AwaitingText,

    /// Inside `<text>`, waiting for `<body>`.
    AwaitingBody,

    /// Inside `<body>`, ready to parse block elements.
    InBody,

    /// After `</body>` but before `</text>` or `</TEI>`.
    AfterBody,

    /// Inside a `<p>` element, accumulating inline content.
    InParagraph {
        /// Optional `xml:id` attribute value.
        id: Option<String>,
        /// Accumulated inline content.
        content: Vec<Inline>,
    },

    /// Inside a `<u>` element, accumulating inline content.
    InUtterance {
        /// Raw utterance attributes.
        attrs: RawUtteranceAttrs,
        /// Accumulated inline content.
        content: Vec<Inline>,
    },

    /// Inside an inline `<hi>` element.
    InEmphasis {
        /// The parent state to return to after closing `</hi>`.
        ///
        /// Uses `Option` to enable explicit ownership transfer via `.take()`.
        parent: Option<Box<ParserState>>,
        /// Optional `rend` attribute for rendering hint.
        rend: Option<String>,
        /// Accumulated inline content within the emphasis.
        content: Vec<Inline>,
    },

    /// Inside a `<div>` element, accumulating block-level children.
    InDiv {
        /// Required `@type` attribute.
        div_type: String,
        /// Optional `@subtype` attribute.
        subtype: Option<String>,
        /// Optional `xml:id` attribute.
        id: Option<String>,
        /// Optional heading captured before any child content.
        head: Option<Head>,
        /// Optional parent div state for nested structural divisions.
        ///
        /// This is distinct from `TeiPullParser::pending_div_state`, which is
        /// only used while a direct paragraph or utterance child is being
        /// assembled.
        parent_div: Option<Box<ParserState>>,
        /// Accumulated content.
        content: Vec<DivContent>,
    },

    /// Inside a `<head>` element within a `<div>`.
    InHead {
        /// The parent div state to return to after closing `</head>`.
        parent_div: Option<Box<ParserState>>,
        /// Accumulated inline content.
        content: Vec<Inline>,
    },

    /// Inside a `<list>` element within a `<div>`, accumulating items.
    InList {
        /// The parent div state to return to after closing `</list>`.
        parent_div: Option<Box<ParserState>>,
        /// Optional `xml:id` for the list.
        list_id: Option<String>,
        /// Accumulated items.
        items: Vec<Item>,
    },

    /// Inside an `<item>` element, accumulating inline content.
    InItem {
        /// The parent list state to return to after closing `</item>`.
        parent_list: Option<Box<ParserState>>,
        /// Optional `xml:id`.
        item_id: Option<String>,
        /// Optional `@n` attribute.
        item_n: Option<String>,
        /// Optional `@corresp` attribute.
        item_corresp: Option<String>,
        /// Optional label child element.
        label: Option<Label>,
        /// Accumulated inline content.
        content: Vec<Inline>,
    },

    /// Inside a `<label>` element within an `<item>`.
    InLabel {
        /// The parent item state to return to after closing `</label>`.
        parent_item: Option<Box<ParserState>>,
        /// Accumulated inline content.
        content: Vec<Inline>,
    },

    /// Document parsing completed successfully.
    DocumentComplete,

    /// An error occurred; no more events will be yielded.
    Error,
}

impl ParserState {
    const fn make_in_div(
        div_type: String,
        subtype: Option<String>,
        id: Option<String>,
        parent_div: Option<Box<Self>>,
    ) -> Self {
        Self::InDiv {
            div_type,
            subtype,
            id,
            head: None,
            parent_div,
            content: Vec::new(),
        }
    }

    /// Creates a new `InHeader` state with the given initial depth.
    #[must_use]
    pub const fn in_header(depth: usize) -> Self {
        Self::InHeader {
            depth,
            buffer: Vec::new(),
        }
    }

    /// Creates a new `InParagraph` state with the given optional id.
    #[must_use]
    pub const fn in_paragraph(id: Option<String>) -> Self {
        Self::InParagraph {
            id,
            content: Vec::new(),
        }
    }

    /// Creates a new `InUtterance` state with the given raw attributes.
    #[must_use]
    pub const fn in_utterance(attrs: RawUtteranceAttrs) -> Self {
        Self::InUtterance {
            attrs,
            content: Vec::new(),
        }
    }

    /// Creates a new `InEmphasis` state with the given parent and rend attribute.
    #[must_use]
    pub fn in_emphasis(parent: Self, rend: Option<String>) -> Self {
        Self::InEmphasis {
            parent: Some(Box::new(parent)),
            rend,
            content: Vec::new(),
        }
    }

    /// Transitions the current state to `InEmphasis`, preserving `self` as the parent.
    ///
    /// This method safely moves the current state into the new emphasis state without
    /// exposing any temporary invalid state values externally.
    pub fn transition_to_emphasis(&mut self, rend: Option<String>) {
        let parent = std::mem::take(self);
        *self = Self::in_emphasis(parent, rend);
    }

    /// Creates a new `InDiv` state with the given type and optional id.
    #[must_use]
    pub const fn in_div(div_type: String, subtype: Option<String>, id: Option<String>) -> Self {
        Self::make_in_div(div_type, subtype, id, None)
    }

    /// Creates a new nested `InDiv` state with the given parent division.
    #[must_use]
    pub fn nested_div(
        parent_div: Self,
        div_type: String,
        subtype: Option<String>,
        id: Option<String>,
    ) -> Self {
        Self::make_in_div(div_type, subtype, id, Some(Box::new(parent_div)))
    }

    /// Creates a new `InList` state with the given parent div and optional list id.
    #[must_use]
    pub fn in_list(parent_div: Self, list_id: Option<String>) -> Self {
        Self::InList {
            parent_div: Some(Box::new(parent_div)),
            list_id,
            items: Vec::new(),
        }
    }

    /// Creates a new `InItem` state with the given parent list and attributes.
    #[must_use]
    pub fn in_item(
        parent_list: Self,
        item_id: Option<String>,
        item_n: Option<String>,
        item_corresp: Option<String>,
    ) -> Self {
        Self::InItem {
            parent_list: Some(Box::new(parent_list)),
            item_id,
            item_n,
            item_corresp,
            label: None,
            content: Vec::new(),
        }
    }

    /// Creates a new `InLabel` state with the given parent item.
    #[must_use]
    pub fn in_label(parent_item: Self) -> Self {
        Self::InLabel {
            parent_item: Some(Box::new(parent_item)),
            content: Vec::new(),
        }
    }

    /// Creates a new `InHead` state with the given parent division.
    #[must_use]
    pub fn in_head(parent_div: Self) -> Self {
        Self::InHead {
            parent_div: Some(Box::new(parent_div)),
            content: Vec::new(),
        }
    }

    /// Returns a mutable reference to the inline content of the current block state, if any.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "const fn with &mut self is not stable"
    )]
    fn content_mut(&mut self) -> Option<&mut Vec<Inline>> {
        match self {
            Self::InParagraph { content, .. }
            | Self::InUtterance { content, .. }
            | Self::InEmphasis { content, .. }
            | Self::InItem { content, .. }
            | Self::InHead { content, .. }
            | Self::InLabel { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Pushes inline content to any block state that accepts inline content.
    ///
    /// In debug builds, asserts that the state has inline content.
    pub fn push_inline(&mut self, inline: Inline) {
        if let Some(content) = self.content_mut() {
            content.push(inline);
        } else {
            debug_assert!(false, "push_inline called on non-block state: {self:?}");
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
