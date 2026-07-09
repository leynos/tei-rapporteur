//! Event types yielded by the streaming TEI parser.
//!
//! The [`TeiEvent`] enum represents high-level domain events encountered during
//! incremental parsing of a TEI document. Events are designed to allow processing
//! of large documents without loading the entire content into memory.

use tei_core::{BodyBlock, TeiHeader};

/// High-level events yielded during incremental TEI parsing.
///
/// The streaming parser yields these events as it processes the document,
/// allowing callers to handle content incrementally rather than waiting for
/// the complete document to be parsed.
///
/// # Event Sequence
///
/// A well-formed TEI document produces events in this order:
/// 1. [`TeiEvent::DocumentStart`] - exactly once at the beginning
/// 2. [`TeiEvent::Header`] - exactly once after the header is fully parsed
/// 3. [`TeiEvent::BodyBlock`] - zero or more times, one per paragraph/utterance
/// 4. [`TeiEvent::DocumentEnd`] - exactly once at the end
///
/// # Examples
///
/// ```no_run
/// use tei_xml::streaming::{TeiPullParser, TeiEvent};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let xml = "<TEI>...</TEI>";
///     let parser = TeiPullParser::from_str(xml);
///     for event in parser {
///         match event? {
///             TeiEvent::DocumentStart => println!("Parsing started"),
///             TeiEvent::Header(header) => println!("Title: {}", header.file_desc().title().as_str()),
///             TeiEvent::BodyBlock(block) => println!("Received block: {block:?}"),
///             TeiEvent::DocumentEnd => println!("Parsing complete"),
///         }
///     }
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum TeiEvent {
    /// Signals the start of document parsing.
    ///
    /// This event is emitted once at the beginning of parsing, before any
    /// content is processed. It allows callers to perform setup before
    /// document content arrives.
    DocumentStart,

    /// The fully-parsed TEI header metadata.
    ///
    /// The header is yielded as a complete unit rather than streaming its
    /// children because:
    /// - Headers are typically small (metadata only)
    /// - Speaker declarations must be available before validating utterance
    ///   `@who` references in body blocks
    ///
    /// After this event, [`crate::streaming::TeiPullParser::header`] returns
    /// the same header.
    Header(TeiHeader),

    /// A body block element (paragraph or utterance).
    ///
    /// Body blocks are yielded one at a time as they are parsed, providing
    /// the primary streaming benefit for large documents. Each block is
    /// complete when yielded, including any inline elements.
    BodyBlock(BodyBlock),

    /// Signals the end of document parsing.
    ///
    /// This event is emitted once after all content has been successfully
    /// parsed. The iterator returns `None` after yielding this event.
    DocumentEnd,
}

impl TeiEvent {
    /// Returns `true` if this is a [`TeiEvent::DocumentStart`] event.
    #[must_use]
    pub const fn is_document_start(&self) -> bool {
        matches!(self, Self::DocumentStart)
    }

    /// Returns `true` if this is a [`TeiEvent::Header`] event.
    #[must_use]
    pub const fn is_header(&self) -> bool {
        matches!(self, Self::Header(_))
    }

    /// Returns `true` if this is a [`TeiEvent::BodyBlock`] event.
    #[must_use]
    pub const fn is_body_block(&self) -> bool {
        matches!(self, Self::BodyBlock(_))
    }

    /// Returns `true` if this is a [`TeiEvent::DocumentEnd`] event.
    #[must_use]
    pub const fn is_document_end(&self) -> bool {
        matches!(self, Self::DocumentEnd)
    }

    /// Returns the header if this is a [`TeiEvent::Header`] event.
    #[must_use]
    pub const fn as_header(&self) -> Option<&TeiHeader> {
        match self {
            Self::Header(header) => Some(header),
            _ => None,
        }
    }

    /// Returns the body block if this is a [`TeiEvent::BodyBlock`] event.
    #[must_use]
    pub const fn as_body_block(&self) -> Option<&BodyBlock> {
        match self {
            Self::BodyBlock(block) => Some(block),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Tests for streaming parser events.
    use super::*;
    use rstest::rstest;
    use tei_core::{BodyContentError, DocumentTitleError, FileDesc, P};

    fn test_header() -> Result<TeiHeader, DocumentTitleError> {
        Ok(TeiHeader::new(FileDesc::from_title_str("Test")?))
    }

    fn test_body_block() -> Result<BodyBlock, BodyContentError> {
        Ok(BodyBlock::Paragraph(P::from_text_segments(["Hello"])?))
    }

    /// Predicate expectations encoded as (`is_start`, `is_header`, `is_body`, `is_end`).
    type PredicateExpectation = (bool, bool, bool, bool);

    #[rstest]
    #[case::document_start(TeiEvent::DocumentStart, (true, false, false, false))]
    #[case::header(TeiEvent::Header(test_header().expect("valid header")), (false, true, false, false))]
    #[case::body_block(TeiEvent::BodyBlock(test_body_block().expect("valid body block")), (false, false, true, false))]
    #[case::document_end(TeiEvent::DocumentEnd, (false, false, false, true))]
    fn event_predicates(#[case] event: TeiEvent, #[case] expected: PredicateExpectation) {
        let (is_start, is_header, is_body, is_end) = expected;
        assert_eq!(event.is_document_start(), is_start);
        assert_eq!(event.is_header(), is_header);
        assert_eq!(event.is_body_block(), is_body);
        assert_eq!(event.is_document_end(), is_end);
    }

    #[test]
    fn header_accessor() {
        let header = test_header().expect("valid header");
        let event = TeiEvent::Header(header.clone());
        assert_eq!(event.as_header(), Some(&header));
        assert!(TeiEvent::DocumentStart.as_header().is_none());
    }

    #[test]
    fn body_block_accessor() {
        let block = test_body_block().expect("valid body block");
        let event = TeiEvent::BodyBlock(block.clone());
        assert_eq!(event.as_body_block(), Some(&block));
        assert!(TeiEvent::DocumentStart.as_body_block().is_none());
    }
}
