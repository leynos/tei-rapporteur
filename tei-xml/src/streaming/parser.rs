//! Streaming pull parser for TEI documents.
//!
//! The [`TeiPullParser`] provides incremental parsing of TEI XML documents,
//! yielding high-level domain events as the document is processed.

use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

use tei_core::{BodyBlock, Inline, TeiError, TeiHeader};

use super::event::TeiEvent;
use super::helpers::{
    append_empty_element, append_end_element, append_start_element, build_hi, build_paragraph,
    build_pause, build_utterance, extract_attribute, extract_xml_id,
};
use super::state::ParserState;

/// Incremental pull parser for TEI documents.
///
/// The parser implements [`Iterator`], yielding [`TeiEvent`] values as it
/// processes the document. This allows handling of large documents without
/// loading the entire content into memory.
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
    reader: Reader<R>,
    state: ParserState,
    header: Option<TeiHeader>,
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
        element: &BytesStart<'_>,
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
            ParserState::InParagraph { .. }
            | ParserState::InUtterance { .. }
            | ParserState::InEmphasis { .. } => {
                self.handle_block_content_start(name_bytes, element)
            }
            _ => Ok(None),
        }
    }

    /// Handles start elements when awaiting the root TEI element.
    fn handle_root_start(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        if name_bytes == b"TEI" {
            self.state = ParserState::AwaitingHeader;
        }
        None
    }

    /// Handles start elements when awaiting the teiHeader element.
    fn handle_awaiting_header_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"teiHeader" {
            self.state = ParserState::in_header(1);
            if let ParserState::InHeader { buffer, .. } = &mut self.state {
                append_start_element(buffer, element)?;
            }
        }
        Ok(None)
    }

    /// Handles start elements when inside the header.
    fn handle_in_header_start(
        &mut self,
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHeader { depth, buffer } = &mut self.state {
            *depth += 1;
            append_start_element(buffer, element)?;
        }
        Ok(None)
    }

    /// Handles start elements when awaiting the text element.
    fn handle_awaiting_text_start(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        if name_bytes == b"text" {
            self.state = ParserState::AwaitingBody;
        }
        None
    }

    /// Handles start elements when awaiting the body element.
    fn handle_awaiting_body_start(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        if name_bytes == b"body" {
            self.state = ParserState::InBody;
        }
        None
    }

    /// Handles start elements for body content (paragraphs and utterances).
    fn handle_body_content_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"p" {
            let id = extract_xml_id(element)?;
            self.state = ParserState::in_paragraph(id);
        } else if name_bytes == b"u" {
            let id = extract_xml_id(element)?;
            let who = extract_attribute(element, b"who")?;
            self.state = ParserState::in_utterance(id, who);
        }
        Ok(None)
    }

    /// Handles start elements for inline content (emphasis).
    fn handle_block_content_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"hi" {
            let rend = extract_attribute(element, b"rend")?;
            let parent = std::mem::replace(&mut self.state, ParserState::Error);
            self.state = ParserState::in_emphasis(parent, rend);
        }
        Ok(None)
    }

    /// Handles an end element event.
    fn handle_end_element(&mut self, element: &BytesEnd<'_>) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes = name.as_ref();

        match &self.state {
            ParserState::InHeader { .. } => self.handle_header_end(element),
            ParserState::InParagraph { .. } if name_bytes == b"p" => self.finish_paragraph(),
            ParserState::InUtterance { .. } if name_bytes == b"u" => self.finish_utterance(),
            ParserState::InEmphasis { .. } if name_bytes == b"hi" => self.finish_emphasis(),
            ParserState::InBody => Ok(self.handle_body_end(name_bytes)),
            ParserState::AfterBody => Ok(self.handle_after_body_end(name_bytes)),
            _ => Ok(None),
        }
    }

    /// Handles closing of header elements and emits Header event when complete.
    fn handle_header_end(&mut self, element: &BytesEnd<'_>) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHeader { depth, buffer } = &mut self.state {
            append_end_element(buffer, element);
            *depth -= 1;
            if *depth == 0 {
                let header = self.parse_accumulated_header()?;
                self.header = Some(header.clone());
                self.state = ParserState::AwaitingText;
                return Ok(Some(TeiEvent::Header(header)));
            }
        }
        Ok(None)
    }

    /// Finishes parsing a paragraph and emits a `BodyBlock` event.
    fn finish_paragraph(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InParagraph { id, content } = &mut self.state {
            let paragraph = build_paragraph(id.take(), std::mem::take(content))?;
            self.state = ParserState::InBody;
            return Ok(Some(TeiEvent::BodyBlock(BodyBlock::Paragraph(paragraph))));
        }
        Ok(None)
    }

    /// Finishes parsing an utterance and emits a `BodyBlock` event.
    fn finish_utterance(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InUtterance { id, who, content } = &mut self.state {
            let utterance =
                build_utterance(id.take(), who.take().as_deref(), std::mem::take(content))?;
            self.state = ParserState::InBody;
            return Ok(Some(TeiEvent::BodyBlock(BodyBlock::Utterance(utterance))));
        }
        Ok(None)
    }

    /// Finishes parsing emphasis and pushes it to the parent state.
    fn finish_emphasis(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        let state = std::mem::take(&mut self.state);
        if let ParserState::InEmphasis {
            parent,
            rend,
            content,
        } = state
        {
            let hi = build_hi(rend, content)?;
            self.state = *parent;
            self.state.push_inline(Inline::Hi(hi));
        }
        Ok(None)
    }

    /// Handles body end elements.
    fn handle_body_end(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        match name_bytes {
            b"body" => {
                // Body closed, transition to AfterBody state
                self.state = ParserState::AfterBody;
                None
            }
            _ => None,
        }
    }

    /// Handles end elements after the body has closed.
    fn handle_after_body_end(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        match name_bytes {
            b"text" | b"TEI" => {
                self.state = ParserState::DocumentComplete;
                Some(TeiEvent::DocumentEnd)
            }
            _ => None,
        }
    }

    /// Handles a text event.
    fn handle_text(&mut self, text: &BytesText<'_>) -> Result<Option<TeiEvent>, TeiError> {
        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                // Keep text escaped for later reparsing by quick_xml::de
                buffer.extend_from_slice(text.as_ref());
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. } => {
                let unescaped = text.unescape().map_err(|e| TeiError::xml(e.to_string()))?;
                if !unescaped.is_empty() {
                    content.push(Inline::Text(unescaped.into_owned()));
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Handles an empty element event (self-closing tag).
    fn handle_empty_element(
        &mut self,
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes = name.as_ref();

        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                append_empty_element(buffer, element)?;
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. }
                if name_bytes == b"pause" =>
            {
                let dur = extract_attribute(element, b"dur")?;
                let pause_type = extract_attribute(element, b"type")?;
                let pause = build_pause(dur, pause_type);
                content.push(Inline::Pause(pause));
                Ok(None)
            }
            ParserState::AwaitingBody if name_bytes == b"body" => {
                // Empty body, move to document end
                self.state = ParserState::DocumentComplete;
                Ok(Some(TeiEvent::DocumentEnd))
            }
            _ => Ok(None),
        }
    }

    /// Handles CDATA sections.
    fn handle_cdata(
        &mut self,
        cdata: &quick_xml::events::BytesCData<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                // Reconstruct CDATA for reparsing
                buffer.extend_from_slice(b"<![CDATA[");
                buffer.extend_from_slice(cdata.as_ref());
                buffer.extend_from_slice(b"]]>");
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. } => {
                // CDATA content is already unescaped, convert to string with strict UTF-8
                let text = std::str::from_utf8(cdata.as_ref())
                    .map_err(|e| TeiError::xml(format!("invalid UTF-8 in CDATA: {e}")))?;
                if !text.is_empty() {
                    content.push(Inline::Text(text.to_owned()));
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Handles end-of-file.
    fn handle_eof(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        match &self.state {
            ParserState::DocumentComplete => Ok(None),
            ParserState::InBody | ParserState::AfterBody => {
                // Allow EOF after body content if document wasn't properly closed
                self.state = ParserState::DocumentComplete;
                Ok(Some(TeiEvent::DocumentEnd))
            }
            _ => {
                self.state = ParserState::Error;
                Err(TeiError::xml("unexpected end of document"))
            }
        }
    }

    /// Parses the accumulated header buffer into a `TeiHeader`.
    fn parse_accumulated_header(&mut self) -> Result<TeiHeader, TeiError> {
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
