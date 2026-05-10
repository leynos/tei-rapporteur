//! XML adapter for extracting ADR-006 spoken text segments.

use quick_xml::{Reader, events::BytesStart, events::Event};
use tei_core::{SpokenTextSegment, TeiError};

use self::{
    element_names::{BODY, DIV, TEI, TEI_HEADER, TEXT},
    frame::ElementFrame,
    header::HeaderRecorder,
    predicates::{is_body_element, is_excluded_element, is_silent_boundary_element},
    segments::SegmentCollector,
    xml_utils::{extract_attribute, local_name, make_locator, resolve_entity_ref},
};

mod element_names;
mod frame;
mod header;
mod predicates;
mod segments;
mod xml_utils;

/// Extracts ordered spoken text segments from a complete TEI XML document.
///
/// # Errors
///
/// Returns [`TeiError::Xml`] when the input is malformed XML, omits the
/// required TEI document shell, or uses unsupported body markup for the current
/// Episodic profile.
///
/// # Examples
///
/// ```
/// use tei_xml::spoken_text_segments;
///
/// let xml = concat!(
///     "<TEI>",
///     "<teiHeader><fileDesc><title>Example</title></fileDesc></teiHeader>",
///     "<text><body><p>Hello <seg>there</seg>.</p></body></text>",
///     "</TEI>",
/// );
/// let segments = spoken_text_segments(xml)?;
/// assert_eq!(segments[0].text(), "Hello there.");
/// # Ok::<(), tei_core::TeiError>(())
/// ```
pub fn spoken_text_segments(xml: &str) -> Result<Vec<SpokenTextSegment>, TeiError> {
    SpokenTextParser::new(xml).parse()
}

#[derive(Debug)]
struct SpokenTextParser<'a> {
    reader: Reader<&'a [u8]>,
    stack: Vec<ElementFrame>,
    segment_collector: SegmentCollector,
    inside_body: bool,
    exclusion_depth: usize,
    document_state: DocumentState,
    header: HeaderRecorder,
}

#[derive(Clone, Copy, Debug, Default)]
struct DocumentState {
    phase: DocumentPhase,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum DocumentPhase {
    #[default]
    Start,
    SawTei,
    SawHeader,
    SawText,
    SawBody,
}

impl<'a> SpokenTextParser<'a> {
    fn new(xml: &'a str) -> Self {
        Self {
            reader: Reader::from_str(xml),
            stack: Vec::new(),
            segment_collector: SegmentCollector::default(),
            inside_body: false,
            exclusion_depth: 0,
            document_state: DocumentState::default(),
            header: HeaderRecorder::default(),
        }
    }

    fn parse(mut self) -> Result<Vec<SpokenTextSegment>, TeiError> {
        loop {
            let event = self
                .reader
                .read_event()
                .map_err(|error| TeiError::xml(error.to_string()))?;
            if let Event::Eof = event {
                break;
            }
            self.handle_event(event)?;
        }
        self.finish()
    }

    fn handle_event(&mut self, event: Event<'_>) -> Result<(), TeiError> {
        match event {
            Event::Start(element) => self.handle_start(&element),
            Event::Empty(element) => self.handle_empty(&element),
            Event::End(element) => {
                let name = local_name(element.local_name().as_ref())?;
                self.header.record_end(&name)?;
                self.handle_end(&name)
            }
            Event::Text(text) => {
                self.header.record_raw_text(text.as_ref());
                let value = text
                    .decode()
                    .map_err(|error| TeiError::xml(error.to_string()))?
                    .into_owned();
                self.push_text(&value);
                Ok(())
            }
            Event::CData(cdata) => {
                self.header.record_cdata(cdata.as_ref());
                let value = std::str::from_utf8(cdata.as_ref())
                    .map_err(|error| TeiError::xml(format!("invalid UTF-8 in CDATA: {error}")))?;
                self.push_text(value);
                Ok(())
            }
            Event::GeneralRef(reference) => {
                self.header.record_general_ref(reference.as_ref());
                let value = resolve_entity_ref(&reference)?;
                self.push_text(&value);
                Ok(())
            }
            Event::Decl(_) | Event::PI(_) | Event::Comment(_) | Event::DocType(_) | Event::Eof => {
                Ok(())
            }
        }
    }

    fn handle_start(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        self.handle_element(element, false)
    }

    fn handle_empty(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        self.handle_element(element, true)
    }

    /// Shared setup for start and empty elements.
    ///
    /// Decodes the local name, records the element in the header recorder,
    /// builds and pushes a stack frame, and enters element/cached state. When
    /// `is_empty` is `true`, immediately finalises the element by calling
    /// [`Self::handle_end`].
    fn handle_element(&mut self, element: &BytesStart<'_>, is_empty: bool) -> Result<(), TeiError> {
        let name = local_name(element.local_name().as_ref())?;
        if is_empty {
            self.header.record_empty(&name, element)?;
        } else {
            self.header.record_start(&name, element)?;
        }
        let frame = self.frame_for_start(&name, element)?;
        self.enter_element(&name, element, &frame)?;
        self.enter_cached_state(&name, &frame);
        self.stack.push(frame);
        if is_empty {
            self.handle_end(&name)?;
        }
        Ok(())
    }

    fn enter_element(
        &mut self,
        name: &str,
        element: &BytesStart<'_>,
        frame: &ElementFrame,
    ) -> Result<(), TeiError> {
        match name {
            TEI => self.record_tei_root()?,
            TEI_HEADER => self.record_tei_header()?,
            TEXT => self.validate_text_path()?,
            BODY => self.record_body()?,
            _ => {}
        }

        if self.is_inside_body() {
            self.validate_body_element(name)?;
        }

        if frame.is_excluded {
            self.push_boundary();
            return Ok(());
        }
        if self.is_excluded() {
            return Ok(());
        }
        if is_silent_boundary_element(name) {
            self.push_boundary();
            return Ok(());
        }
        if !self.is_inside_body() {
            return Ok(());
        }
        self.segment_collector
            .maybe_start_segment(name, element, frame)
    }

    fn handle_end(&mut self, name: &str) -> Result<(), TeiError> {
        if self.segment_collector.should_finish_active_segment(name) {
            self.segment_collector.finish_active_segment()?;
        }

        let frame = self
            .stack
            .pop()
            .ok_or_else(|| TeiError::xml(format!("unexpected closing element </{name}>")))?;
        if frame.name != name {
            return Err(TeiError::xml(format!(
                "mismatched closing element </{name}> for <{}>",
                frame.name
            )));
        }
        if frame.is_excluded {
            self.push_boundary();
        }
        self.exit_cached_state(name, &frame);
        Ok(())
    }

    fn enter_cached_state(&mut self, name: &str, frame: &ElementFrame) {
        if name == BODY {
            self.inside_body = true;
        }
        if frame.is_excluded {
            self.exclusion_depth += 1;
        }
    }

    fn exit_cached_state(&mut self, name: &str, frame: &ElementFrame) {
        if frame.is_excluded {
            self.exclusion_depth = self.exclusion_depth.saturating_sub(1);
        }
        if name == BODY {
            self.inside_body = false;
        }
    }

    fn frame_for_start(
        &mut self,
        name: &str,
        element: &BytesStart<'_>,
    ) -> Result<ElementFrame, TeiError> {
        let index = self.next_child_index(name);
        let parent_locator = self.stack.last().map(|frame| frame.locator.as_str());
        let locator = make_locator(parent_locator, name, index);
        let is_excluded = Self::element_is_excluded(name, element)?;
        Ok(ElementFrame::new(name.to_owned(), locator, is_excluded))
    }

    fn next_child_index(&mut self, name: &str) -> usize {
        let Some(parent) = self.stack.last_mut() else {
            return 1;
        };
        let count = parent.child_counts.entry(name.to_owned()).or_insert(0);
        *count += 1;
        *count
    }

    fn push_text(&mut self, value: &str) {
        self.segment_collector
            .push_text(value, self.is_inside_body(), self.is_excluded());
    }

    fn push_boundary(&mut self) {
        self.segment_collector.push_boundary();
    }

    fn finish(self) -> Result<Vec<SpokenTextSegment>, TeiError> {
        if !self.document_state.saw_tei() {
            return Err(TeiError::xml("missing TEI root element"));
        }
        if !self.document_state.saw_header() {
            return Err(TeiError::xml("missing teiHeader element"));
        }
        if !self.header.is_validated() {
            return Err(TeiError::xml("invalid teiHeader element"));
        }
        if !self.document_state.saw_body() {
            return Err(TeiError::xml("missing body element"));
        }
        if !self.stack.is_empty() {
            return Err(TeiError::xml("unexpected end of document"));
        }
        self.segment_collector.finish()
    }

    const fn is_inside_body(&self) -> bool {
        self.inside_body
    }

    const fn is_excluded(&self) -> bool {
        self.exclusion_depth > 0
    }

    fn element_is_excluded(name: &str, element: &BytesStart<'_>) -> Result<bool, TeiError> {
        if name == DIV && extract_attribute(element, b"type")?.as_deref() == Some("notes") {
            return Ok(true);
        }
        Ok(is_excluded_element(name))
    }

    fn validate_body_element(&self, name: &str) -> Result<(), TeiError> {
        if self.is_excluded() {
            return Ok(());
        }
        let is_known = is_body_element(name);
        if is_known {
            Ok(())
        } else {
            Err(TeiError::xml(format!(
                "unsupported TEI body element <{name}>"
            )))
        }
    }

    fn record_tei_root(&mut self) -> Result<(), TeiError> {
        let ok = self.stack.is_empty() && self.document_state.phase == DocumentPhase::Start;
        self.advance_phase_if(
            ok,
            DocumentPhase::SawTei,
            "TEI root element must be the document root",
        )
    }

    fn record_tei_header(&mut self) -> Result<(), TeiError> {
        let ok = self.stack.last().is_some_and(|f| f.name == TEI)
            && self.document_state.phase == DocumentPhase::SawTei;
        self.advance_phase_if(
            ok,
            DocumentPhase::SawHeader,
            "teiHeader element must be inside TEI root",
        )
    }

    fn validate_text_path(&mut self) -> Result<(), TeiError> {
        if self.stack.last().is_none_or(|frame| frame.name != TEI) {
            return Err(TeiError::xml("text element must be inside TEI root"));
        }
        if self.document_state.phase == DocumentPhase::SawTei {
            return Err(TeiError::xml("missing teiHeader element"));
        }
        if self.document_state.phase == DocumentPhase::SawHeader {
            self.document_state.phase = DocumentPhase::SawText;
            Ok(())
        } else {
            Err(TeiError::xml("text element must be inside TEI root"))
        }
    }

    fn record_body(&mut self) -> Result<(), TeiError> {
        let ok = self.is_direct_child_of_text_in_tei()
            && self.document_state.phase == DocumentPhase::SawText;
        self.advance_phase_if(
            ok,
            DocumentPhase::SawBody,
            "body element must be inside TEI text",
        )
    }

    /// Advances the document phase to `next` when `condition` holds; otherwise
    /// returns a structured XML error with the given message.
    fn advance_phase_if(
        &mut self,
        condition: bool,
        next: DocumentPhase,
        error: &str,
    ) -> Result<(), TeiError> {
        if condition {
            self.document_state.phase = next;
            Ok(())
        } else {
            Err(TeiError::xml(error))
        }
    }

    fn is_direct_child_of_text_in_tei(&self) -> bool {
        let [.., grandparent, parent] = self.stack.as_slice() else {
            return false;
        };
        parent.name == TEXT && grandparent.name == TEI
    }
}

impl DocumentState {
    const fn saw_tei(self) -> bool {
        !matches!(self.phase, DocumentPhase::Start)
    }

    const fn saw_header(self) -> bool {
        matches!(
            self.phase,
            DocumentPhase::SawHeader | DocumentPhase::SawText | DocumentPhase::SawBody
        )
    }

    const fn saw_body(self) -> bool {
        matches!(self.phase, DocumentPhase::SawBody)
    }
}
