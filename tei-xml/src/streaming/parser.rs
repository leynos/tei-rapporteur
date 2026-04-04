//! Streaming pull parser for TEI documents.
//!
//! The [`TeiPullParser`] provides incremental parsing of TEI XML documents,
//! yielding high-level domain events as the document is processed.

use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::Event;

use tei_core::{TeiError, TeiHeader};

use super::event::TeiEvent;
use super::state::ParserState;

/// Incremental pull parser for TEI documents.
///
/// The parser implements [`Iterator`], yielding [`TeiEvent`] values as it
/// processes the document. This allows handling of large documents without
/// loading the entire content into memory.
///
/// # EOF Handling
///
/// The parser is lenient with end-of-file conditions: if the document body
/// has been fully processed but the closing `</text>` or `</TEI>` tags are
/// missing, the parser will still emit a [`TeiEvent::DocumentEnd`] event
/// rather than returning an error. This accommodates truncated or incomplete
/// documents while preserving successfully parsed content. Documents that
/// end before the body content is complete will return an error.
///
/// # Examples
///
/// ```no_run
/// use std::io::BufReader;
/// use std::fs::File;
/// use tei_xml::streaming::{TeiPullParser, TeiEvent};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let file = File::open("large-episode.tei.xml")?;
///     let reader = BufReader::new(file);
///     let parser = TeiPullParser::new(reader);
///
///     for event in parser {
///         match event? {
///             TeiEvent::DocumentStart => println!("Parsing started"),
///             TeiEvent::Header(header) => {
///                 println!("Title: {}", header.file_desc().title().as_str());
///             }
///             TeiEvent::BodyBlock(block) => println!("Received block: {block:?}"),
///             TeiEvent::DocumentEnd => println!("Parsing complete"),
///         }
///     }
///     Ok(())
/// }
/// ```
pub struct TeiPullParser<R: BufRead> {
    pub(super) reader: Reader<R>,
    pub(super) state: ParserState,
    pub(super) header: Option<TeiHeader>,
    pub(super) pending_div_state: Option<Box<ParserState>>,
}

impl<R: BufRead> TeiPullParser<R> {
    /// Creates a new pull parser from a buffered reader.
    #[must_use]
    pub fn new(reader: R) -> Self {
        let xml_reader = Reader::from_reader(reader);

        Self {
            reader: xml_reader,
            state: ParserState::Initial,
            header: None,
            pending_div_state: None,
        }
    }

    /// Returns the parsed header, if available.
    ///
    /// The header becomes available after the [`TeiEvent::Header`] event is
    /// yielded. Returns `None` if the header has not yet been parsed.
    #[must_use]
    pub const fn header(&self) -> Option<&TeiHeader> {
        self.header.as_ref()
    }

    /// Advances the parser and returns the next event.
    fn advance(&mut self) -> Option<Result<TeiEvent, TeiError>> {
        match &self.state {
            ParserState::Initial => {
                self.state = ParserState::AwaitingRoot;
                Some(Ok(TeiEvent::DocumentStart))
            }
            ParserState::DocumentComplete | ParserState::Error => None,
            _ => self.process_xml_events(),
        }
    }

    /// Processes XML events until a domain event can be yielded.
    fn process_xml_events(&mut self) -> Option<Result<TeiEvent, TeiError>> {
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let event = match self.reader.read_event_into(&mut buf) {
                Ok(event) => event,
                Err(error) => {
                    self.state = ParserState::Error;
                    return Some(Err(TeiError::xml(error.to_string())));
                }
            };

            match self.handle_event(&event) {
                Ok(Some(tei_event)) => return Some(Ok(tei_event)),
                Ok(None) => {}
                Err(error) => {
                    self.state = ParserState::Error;
                    return Some(Err(error));
                }
            }
        }
    }

    /// Handles a single XML event, potentially yielding a domain event.
    fn handle_event(&mut self, event: &Event<'_>) -> Result<Option<TeiEvent>, TeiError> {
        match event {
            Event::Start(e) => self.handle_start_element(e),
            Event::End(e) => self.handle_end_element(e),
            Event::Text(e) => self.handle_text(e),
            Event::Empty(e) => self.handle_empty_element(e),
            Event::Eof => self.handle_eof(),
            Event::Decl(_) | Event::PI(_) | Event::Comment(_) | Event::DocType(_) => Ok(None),
            Event::CData(e) => self.handle_cdata(e),
        }
    }

    /// Handles a start element event.
    ///
    /// This method dispatches to state-specific handlers based on the current
    /// parser state.
    fn handle_start_element(
        &mut self,
        element: &quick_xml::events::BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes = name.as_ref();

        match &self.state {
            ParserState::AwaitingRoot => Ok(self.handle_root_start(name_bytes)),
            ParserState::AwaitingHeader => self.handle_awaiting_header_start(name_bytes, element),
            ParserState::InHeader { .. } => self.handle_in_header_start(element),
            ParserState::AwaitingText => Ok(self.handle_awaiting_text_start(name_bytes)),
            ParserState::AwaitingBody => Ok(self.handle_awaiting_body_start(name_bytes)),
            ParserState::InBody => self.handle_body_content_start(name_bytes, element),
            ParserState::InDiv { .. } => self.handle_div_content_start(name_bytes, element),
            ParserState::InList { .. } => self.handle_list_content_start(name_bytes, element),
            ParserState::InItem { .. } | ParserState::InLabel { .. } => {
                self.handle_item_content_start(name_bytes, element)
            }
            ParserState::InParagraph { .. }
            | ParserState::InUtterance { .. }
            | ParserState::InEmphasis { .. } => {
                self.handle_block_content_start(name_bytes, element)
            }
            _ => Ok(None),
        }
    }

    /// Handles an end element event.
    fn handle_end_element(
        &mut self,
        element: &quick_xml::events::BytesEnd<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes = name.as_ref();

        match &self.state {
            ParserState::InHeader { .. } => self.handle_header_end(element),
            ParserState::InParagraph { .. } if name_bytes == b"p" => self.finish_paragraph(),
            ParserState::InUtterance { .. } if name_bytes == b"u" => self.finish_utterance(),
            ParserState::InEmphasis { .. } if name_bytes == b"hi" => self.finish_emphasis(),
            ParserState::InDiv { .. } if name_bytes == b"div" => self.finish_div(),
            ParserState::InList { .. } if name_bytes == b"list" => self.finish_list(),
            ParserState::InItem { .. } if name_bytes == b"item" => self.finish_item(),
            ParserState::InLabel { .. } if name_bytes == b"label" => self.finish_label(),
            ParserState::InBody => Ok(self.handle_body_end(name_bytes)),
            ParserState::AfterBody => Ok(self.handle_after_body_end(name_bytes)),
            _ => Ok(None),
        }
    }

    /// Parses the accumulated header buffer into a `TeiHeader`.
    pub(super) fn parse_accumulated_header(&mut self) -> Result<TeiHeader, TeiError> {
        let buffer = match &mut self.state {
            ParserState::InHeader { buffer, .. } => std::mem::take(buffer),
            _ => return Err(TeiError::xml("not in header state")),
        };

        let xml = String::from_utf8(buffer).map_err(|e| TeiError::xml(e.to_string()))?;
        quick_xml::de::from_str(&xml).map_err(|e| TeiError::xml(e.to_string()))
    }
}

impl<'a> TeiPullParser<&'a [u8]> {
    /// Creates a parser from a string slice.
    ///
    /// This is a convenience constructor for parsing XML from an in-memory
    /// string. For file-based parsing, use [`TeiPullParser::new`] with a
    /// [`std::io::BufReader`].
    #[must_use]
    #[expect(
        clippy::should_implement_trait,
        reason = "FromStr cannot express the required lifetime relationship"
    )]
    pub fn from_str(xml: &'a str) -> Self {
        Self::new(xml.as_bytes())
    }
}

impl<R: BufRead> Iterator for TeiPullParser<R> {
    type Item = Result<TeiEvent, TeiError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.advance()
    }
}
