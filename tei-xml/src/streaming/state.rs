//! Parser state machine for incremental TEI parsing.
//!
//! The [`ParserState`] enum tracks the current parsing context, enabling the
//! pull parser to handle nested elements correctly and yield events at
//! appropriate boundaries.

use tei_core::Inline;

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
        /// Optional `xml:id` attribute value.
        id: Option<String>,
        /// Optional `who` attribute (speaker reference).
        who: Option<String>,
        /// Accumulated inline content.
        content: Vec<Inline>,
    },

    /// Inside an inline `<hi>` element.
    InEmphasis {
        /// The parent state to return to after closing `</hi>`.
        parent: Box<ParserState>,
        /// Optional `rend` attribute for rendering hint.
        rend: Option<String>,
        /// Accumulated inline content within the emphasis.
        content: Vec<Inline>,
    },

    /// Document parsing completed successfully.
    DocumentComplete,

    /// An error occurred; no more events will be yielded.
    Error,
}

impl ParserState {
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

    /// Creates a new `InUtterance` state with the given optional id and speaker.
    #[must_use]
    pub const fn in_utterance(id: Option<String>, who: Option<String>) -> Self {
        Self::InUtterance {
            id,
            who,
            content: Vec::new(),
        }
    }

    /// Creates a new `InEmphasis` state with the given parent and rend attribute.
    #[must_use]
    pub fn in_emphasis(parent: Self, rend: Option<String>) -> Self {
        Self::InEmphasis {
            parent: Box::new(parent),
            rend,
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
            | Self::InEmphasis { content, .. } => Some(content),
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
mod tests {
    use super::*;

    /// Test-only helper trait for `ParserState` predicates.
    trait ParserStateTestExt {
        fn is_initial(&self) -> bool;
        fn is_complete(&self) -> bool;
        fn is_error(&self) -> bool;
        fn is_in_body(&self) -> bool;
        fn is_in_block(&self) -> bool;
        fn take_content(&mut self) -> Vec<Inline>;
    }

    impl ParserStateTestExt for ParserState {
        fn is_initial(&self) -> bool {
            matches!(self, Self::Initial)
        }
        fn is_complete(&self) -> bool {
            matches!(self, Self::DocumentComplete)
        }
        fn is_error(&self) -> bool {
            matches!(self, Self::Error)
        }
        fn is_in_body(&self) -> bool {
            matches!(self, Self::InBody)
        }
        fn is_in_block(&self) -> bool {
            matches!(
                self,
                Self::InParagraph { .. } | Self::InUtterance { .. } | Self::InEmphasis { .. }
            )
        }
        fn take_content(&mut self) -> Vec<Inline> {
            self.content_mut().map(std::mem::take).unwrap_or_default()
        }
    }

    #[test]
    fn default_state_is_initial() {
        assert_eq!(ParserState::default(), ParserState::Initial);
        assert!(ParserState::Initial.is_initial());
    }

    #[test]
    fn state_predicates() {
        assert!(ParserState::DocumentComplete.is_complete());
        assert!(ParserState::Error.is_error());
        assert!(ParserState::InBody.is_in_body());
    }

    #[test]
    fn in_block_detection() {
        let paragraph = ParserState::in_paragraph(None);
        assert!(paragraph.is_in_block());

        let utterance = ParserState::in_utterance(Some("u1".into()), Some("speaker".into()));
        assert!(utterance.is_in_block());

        let emphasis = ParserState::in_emphasis(ParserState::InBody, Some("italic".into()));
        assert!(emphasis.is_in_block());

        assert!(!ParserState::InBody.is_in_block());
    }

    #[test]
    fn push_and_take_inline_content() {
        let mut state = ParserState::in_paragraph(Some("p1".into()));
        state.push_inline(Inline::Text("Hello".into()));
        state.push_inline(Inline::Text(" World".into()));

        let content = state.take_content();
        assert_eq!(content.len(), 2);
    }

    #[test]
    fn header_buffer_accumulation() {
        let state = ParserState::in_header(1);
        match state {
            ParserState::InHeader { depth, buffer } => {
                assert_eq!(depth, 1);
                assert!(buffer.is_empty());
            }
            _ => panic!("expected InHeader state"),
        }
    }
}
