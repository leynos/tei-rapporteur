//! Streaming pull parser for TEI documents.
//!
//! The [`TeiPullParser`] provides incremental parsing of TEI XML documents,
//! yielding high-level domain events as the document is processed.

use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

use tei_core::{BodyBlock, Hi, Inline, P, Pause, TeiError, TeiHeader, Utterance};

use super::event::TeiEvent;
use super::state::ParserState;

/// Incremental pull parser for TEI documents.
///
/// The parser implements [`Iterator`], yielding [`TeiEvent`] values as it
/// processes the document. This allows handling of large documents without
/// loading the entire content into memory.
///
/// # Examples
///
/// ```ignore
/// use std::io::BufReader;
/// use std::fs::File;
/// use tei_xml::streaming::{TeiPullParser, TeiEvent};
///
/// let file = File::open("large-episode.tei.xml")?;
/// let reader = BufReader::new(file);
/// let parser = TeiPullParser::new(reader);
///
/// for event in parser {
///     match event? {
///         TeiEvent::DocumentStart => println!("Parsing started"),
///         TeiEvent::Header(header) => {
///             println!("Title: {}", header.file_desc().title().as_str());
///         }
///         TeiEvent::BodyBlock(block) => println!("Received block"),
///         TeiEvent::DocumentEnd => println!("Parsing complete"),
///     }
/// }
/// ```
pub struct TeiPullParser<R: BufRead> {
    reader: Reader<R>,
    state: ParserState,
    header: Option<TeiHeader>,
}

/// Macro to finish a block by extracting state, building, and wrapping in an event.
macro_rules! finish_block {
    ($self:expr, $pattern:pat => $extract:expr, $builder:expr, $variant:path) => {{
        if let $pattern = &mut $self.state {
            let result = $builder($extract)?;
            $self.state = ParserState::InBody;
            return Ok(Some(TeiEvent::BodyBlock($variant(result))));
        }
        Ok(None)
    }};
}

impl<R: BufRead> TeiPullParser<R> {
    /// Creates a new pull parser from a buffered reader.
    #[must_use]
    pub fn new(reader: R) -> Self {
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);

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
            Event::CData(e) => Ok(self.handle_cdata(e)),
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
            self.append_to_header_buffer(element)?;
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
            ParserState::InEmphasis { .. } if name_bytes == b"hi" => Ok(self.finish_emphasis()),
            ParserState::InBody => Ok(self.handle_body_end(name_bytes)),
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
        finish_block!(
            self,
            ParserState::InParagraph { id, content } => {
                (id.take(), std::mem::take(content))
            },
            |(id_val, content_val)| build_paragraph(id_val, content_val),
            BodyBlock::Paragraph
        )
    }

    /// Finishes parsing an utterance and emits a `BodyBlock` event.
    fn finish_utterance(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        finish_block!(
            self,
            ParserState::InUtterance { id, who, content } => {
                (id.take(), who.take(), std::mem::take(content))
            },
            |(id_val, who_val, content_val): (Option<String>, Option<String>, Vec<Inline>)| {
                build_utterance(id_val, who_val.as_deref(), content_val)
            },
            BodyBlock::Utterance
        )
    }

    /// Finishes parsing emphasis and pushes it to the parent state.
    fn finish_emphasis(&mut self) -> Option<TeiEvent> {
        if let ParserState::InEmphasis {
            parent,
            rend,
            content,
        } = &mut self.state
        {
            let rend_val = rend.take();
            let content_val = std::mem::take(content);
            let hi = build_hi(rend_val, content_val);
            let parent_state = std::mem::replace(parent.as_mut(), ParserState::Error);
            self.state = parent_state;
            self.state.push_inline(Inline::Hi(hi));
        }
        None
    }

    /// Handles body end elements.
    fn handle_body_end(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        if name_bytes == b"body" {
            // Body closed, wait for text and TEI to close
            None
        } else if name_bytes == b"text" || name_bytes == b"TEI" {
            self.state = ParserState::DocumentComplete;
            Some(TeiEvent::DocumentEnd)
        } else {
            None
        }
    }

    /// Handles a text event.
    fn handle_text(&mut self, text: &BytesText<'_>) -> Result<Option<TeiEvent>, TeiError> {
        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                let unescaped = text.unescape().map_err(|e| TeiError::xml(e.to_string()))?;
                buffer.extend_from_slice(unescaped.as_bytes());
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
    fn handle_cdata(&mut self, cdata: &quick_xml::events::BytesCData<'_>) -> Option<TeiEvent> {
        match &mut self.state {
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. } => {
                // CDATA content is already unescaped, just convert to string
                let text = String::from_utf8_lossy(cdata.as_ref());
                if !text.is_empty() {
                    content.push(Inline::Text(text.into_owned()));
                }
                None
            }
            _ => None,
        }
    }

    /// Handles end-of-file.
    fn handle_eof(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        match &self.state {
            ParserState::DocumentComplete => Ok(None),
            ParserState::InBody => {
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

    /// Appends start element to header buffer.
    fn append_to_header_buffer(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        if let ParserState::InHeader { buffer, .. } = &mut self.state {
            append_start_element(buffer, element)?;
        }
        Ok(())
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

// Helper functions

fn extract_xml_id(element: &BytesStart<'_>) -> Result<Option<String>, TeiError> {
    extract_attribute(element, b"xml:id")
}

fn extract_attribute(element: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, TeiError> {
    for attr_result in element.attributes() {
        let attr = attr_result.map_err(|e| TeiError::xml(e.to_string()))?;
        if attr.key.as_ref() == name {
            let value = attr
                .unescape_value()
                .map_err(|e| TeiError::xml(e.to_string()))?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

/// Appends an element opening tag with attributes and a custom closing sequence.
fn append_element_with_attributes(
    buffer: &mut Vec<u8>,
    element: &BytesStart<'_>,
    closing: &[u8],
) -> Result<(), TeiError> {
    buffer.push(b'<');
    buffer.extend_from_slice(element.name().as_ref());
    for attr_result in element.attributes() {
        let attr = attr_result.map_err(|e| TeiError::xml(e.to_string()))?;
        buffer.push(b' ');
        buffer.extend_from_slice(attr.key.as_ref());
        buffer.extend_from_slice(b"=\"");
        buffer.extend_from_slice(&attr.value);
        buffer.push(b'"');
    }
    buffer.extend_from_slice(closing);
    Ok(())
}

fn append_start_element(buffer: &mut Vec<u8>, element: &BytesStart<'_>) -> Result<(), TeiError> {
    append_element_with_attributes(buffer, element, b">")
}

fn append_end_element(buffer: &mut Vec<u8>, element: &BytesEnd<'_>) {
    buffer.extend_from_slice(b"</");
    buffer.extend_from_slice(element.name().as_ref());
    buffer.push(b'>');
}

fn append_empty_element(buffer: &mut Vec<u8>, element: &BytesStart<'_>) -> Result<(), TeiError> {
    append_element_with_attributes(buffer, element, b"/>")
}

/// Sets the ID on a block element if present, using the provided setter closure.
fn set_id_if_present<T, E, F>(block: &mut T, id: Option<String>, setter: F) -> Result<(), TeiError>
where
    E: std::fmt::Display,
    F: FnOnce(&mut T, String) -> Result<(), E>,
{
    if let Some(id_str) = id {
        setter(block, id_str).map_err(|e| TeiError::xml(e.to_string()))?;
    }
    Ok(())
}

fn build_paragraph(id: Option<String>, content: Vec<Inline>) -> Result<P, TeiError> {
    let mut paragraph = if content.is_empty() {
        P::from_text_segments([""]).map_err(|e| TeiError::xml(e.to_string()))?
    } else {
        P::from_inline(content).map_err(|e| TeiError::xml(e.to_string()))?
    };
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "Method reference causes lifetime inference failure with generic setter"
    )]
    set_id_if_present(&mut paragraph, id, |p, id_val| p.set_id(id_val))?;
    Ok(paragraph)
}

fn build_utterance(
    id: Option<String>,
    who: Option<&str>,
    content: Vec<Inline>,
) -> Result<Utterance, TeiError> {
    let mut utterance = if content.is_empty() {
        Utterance::from_text_segments(who, [""]).map_err(|e| TeiError::xml(e.to_string()))?
    } else {
        Utterance::from_inline(who, content).map_err(|e| TeiError::xml(e.to_string()))?
    };
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "Method reference causes lifetime inference failure with generic setter"
    )]
    set_id_if_present(&mut utterance, id, |u, id_val| u.set_id(id_val))?;
    Ok(utterance)
}

fn build_hi(rend: Option<String>, content: Vec<Inline>) -> Hi {
    if content.is_empty() {
        // Empty hi element - use a single empty text node
        let hi = Hi::new([Inline::text("")]);
        return match rend {
            Some(r) => Hi::with_rend(r, hi.content().iter().cloned()),
            None => hi,
        };
    }

    match rend {
        Some(r) => Hi::with_rend(r, content),
        None => Hi::new(content),
    }
}

fn build_pause(dur: Option<String>, pause_type: Option<String>) -> Pause {
    let mut pause = Pause::new();
    if let Some(d) = dur {
        pause.set_duration(d);
    }
    if let Some(t) = pause_type {
        pause.set_kind(t);
    }
    pause
}
