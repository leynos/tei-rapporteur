//! Spoken segment lifecycle management for TEI spoken-text extraction.

use quick_xml::events::BytesStart;
use tei_core::{SpokenTextNormalizer, SpokenTextProvenance, SpokenTextSegment, TeiError};

use super::{
    element_names::{AB, L, P, SEG, U},
    frame::ElementFrame,
    xml_utils::extract_xml_id,
};

/// Collects active spoken segments and finished spoken text output.
#[derive(Debug, Default)]
pub(super) struct SegmentCollector {
    active_segments: Vec<ActiveSegment>,
    segments: Vec<SpokenTextSegment>,
}

impl SegmentCollector {
    /// Attempts to start a spoken segment for the current element name.
    ///
    /// Returns [`TeiError`] when segment provenance such as `xml:id` cannot be
    /// extracted from the source element.
    pub(super) fn maybe_start_segment(
        &mut self,
        name: &str,
        element: &BytesStart<'_>,
        frame: &ElementFrame,
    ) -> Result<(), TeiError> {
        match name {
            U => self.start_segment(SegmentStart::new(
                SegmentKind::Utterance,
                name,
                element,
                frame,
            )?),
            P | AB | L => {
                self.start_segment(SegmentStart::new(SegmentKind::Block, name, element, frame)?);
            }
            // Standalone <seg> starts a segment, but nested <seg> contributes
            // to the enclosing spoken block and must not be counted twice.
            SEG if self.active_segments.is_empty() => self.start_segment_without_marking_parent(
                SegmentStart::new(SegmentKind::Block, name, element, frame)?,
            ),
            _ => {}
        }
        Ok(())
    }

    fn start_segment(&mut self, request: SegmentStart) {
        self.mark_parent_has_child_spoken_block();
        self.start_segment_without_marking_parent(request);
    }

    fn start_segment_without_marking_parent(&mut self, request: SegmentStart) {
        self.active_segments.push(ActiveSegment::new(
            request.kind,
            request.name,
            request.locator,
            request.xml_id,
        ));
    }

    /// Returns whether the named end tag closes the current active segment.
    pub(super) fn should_finish_active_segment(&self, name: &str) -> bool {
        self.active_segments
            .last()
            .is_some_and(|segment| segment.name == name)
    }

    /// Completes the current active segment and emits it when it owns text.
    pub(super) fn finish_active_segment(&mut self) -> Result<(), TeiError> {
        let segment = self
            .active_segments
            .pop()
            .ok_or_else(|| TeiError::xml("no active spoken segment to finish"))?;
        if segment.kind == SegmentKind::Utterance && segment.has_child_spoken_block {
            return Ok(());
        }
        if let Some(text) = segment.normalizer.finish() {
            let provenance = SpokenTextProvenance::new(segment.xml_id, segment.locator);
            self.segments.push(SpokenTextSegment::new(text, provenance));
        }
        Ok(())
    }

    /// Pushes text into the current segment when body/exclusion state permits.
    pub(super) fn push_text(&mut self, value: &str, is_inside_body: bool, is_excluded: bool) {
        if !is_inside_body || is_excluded {
            return;
        }
        if let Some(segment) = self.active_segments.last_mut() {
            segment.normalizer.push_text(value);
        }
    }

    /// Records a word boundary in the current active segment.
    pub(super) fn push_boundary(&mut self) {
        if let Some(segment) = self.active_segments.last_mut() {
            segment.normalizer.push_boundary();
        }
    }

    /// Finishes collection and returns ordered spoken text segments.
    pub(super) fn finish(self) -> Result<Vec<SpokenTextSegment>, TeiError> {
        if !self.active_segments.is_empty() {
            return Err(TeiError::xml("unfinished spoken segment"));
        }
        Ok(self.segments)
    }

    fn mark_parent_has_child_spoken_block(&mut self) {
        if let Some(parent) = self.active_segments.last_mut() {
            parent.has_child_spoken_block = true;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SegmentKind {
    Block,
    Utterance,
}

#[derive(Clone, Debug)]
struct ActiveSegment {
    kind: SegmentKind,
    name: String,
    locator: String,
    xml_id: Option<String>,
    normalizer: SpokenTextNormalizer,
    has_child_spoken_block: bool,
}

struct SegmentStart {
    kind: SegmentKind,
    name: String,
    locator: String,
    xml_id: Option<String>,
}

impl SegmentStart {
    fn new(
        kind: SegmentKind,
        name: &str,
        element: &BytesStart<'_>,
        frame: &ElementFrame,
    ) -> Result<Self, TeiError> {
        Ok(Self {
            kind,
            name: name.to_owned(),
            locator: frame.locator.clone(),
            xml_id: extract_xml_id(element)?,
        })
    }
}

impl ActiveSegment {
    fn new(kind: SegmentKind, name: String, locator: String, xml_id: Option<String>) -> Self {
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

#[cfg(test)]
mod tests {
    use quick_xml::events::BytesStart;

    use super::*;

    fn frame(name: &str, locator: &str) -> ElementFrame {
        ElementFrame::new(name.to_owned(), locator.to_owned(), false)
    }

    fn element(name: &str) -> BytesStart<'static> {
        BytesStart::new(name.to_owned())
    }

    fn element_with_xml_id(name: &str, xml_id: &'static str) -> BytesStart<'static> {
        let mut element = element(name);
        element.push_attribute(("xml:id", xml_id));
        element
    }

    #[test]
    fn standalone_seg_starts_segment() {
        let mut collector = SegmentCollector::default();
        let element = element_with_xml_id(SEG, "seg1");
        let frame = frame(SEG, "/TEI/text/body/seg[1]");

        collector
            .maybe_start_segment(SEG, &element, &frame)
            .expect("standalone seg should start");
        collector.push_text("Standalone", true, false);
        assert!(collector.should_finish_active_segment(SEG));
        collector
            .finish_active_segment()
            .expect("standalone seg should finish");

        let segments = collector.finish().expect("collector should finish");
        assert_eq!(segments.len(), 1);
        let emitted_segment = segments.first().expect("one segment should be emitted");
        assert_eq!(emitted_segment.text(), "Standalone");
        assert_eq!(emitted_segment.provenance().xml_id(), Some("seg1"));
    }

    #[test]
    fn nested_seg_contributes_to_parent_without_double_counting() {
        let mut collector = SegmentCollector::default();
        let paragraph = element_with_xml_id(P, "p1");
        let paragraph_frame = frame(P, "/TEI/text/body/p[1]");
        let segment = element_with_xml_id(SEG, "seg1");
        let segment_frame = frame(SEG, "/TEI/text/body/p[1]/seg[1]");

        collector
            .maybe_start_segment(P, &paragraph, &paragraph_frame)
            .expect("paragraph should start");
        collector.push_text("Hello ", true, false);
        collector
            .maybe_start_segment(SEG, &segment, &segment_frame)
            .expect("nested seg should not start a new segment");
        collector.push_text("there", true, false);
        assert!(!collector.should_finish_active_segment(SEG));
        assert!(collector.should_finish_active_segment(P));
        collector
            .finish_active_segment()
            .expect("paragraph should finish");

        let segments = collector.finish().expect("collector should finish");
        assert_eq!(segments.len(), 1);
        let emitted_segment = segments.first().expect("one segment should be emitted");
        assert_eq!(emitted_segment.text(), "Hello there");
        assert_eq!(emitted_segment.provenance().xml_id(), Some("p1"));
    }

    #[test]
    fn utterance_with_child_spoken_block_is_suppressed() {
        let mut collector = SegmentCollector::default();
        let utterance = element_with_xml_id(U, "u1");
        let utterance_frame = frame(U, "/TEI/text/body/u[1]");
        let paragraph = element_with_xml_id(P, "p1");
        let paragraph_frame = frame(P, "/TEI/text/body/u[1]/p[1]");

        collector
            .maybe_start_segment(U, &utterance, &utterance_frame)
            .expect("utterance should start");
        collector.push_text("Outer", true, false);
        collector
            .maybe_start_segment(P, &paragraph, &paragraph_frame)
            .expect("paragraph should start");
        collector.push_text("Inner", true, false);
        collector
            .finish_active_segment()
            .expect("paragraph should finish");
        collector
            .finish_active_segment()
            .expect("utterance should finish");

        let segments = collector.finish().expect("collector should finish");
        assert_eq!(segments.len(), 1);
        let emitted_segment = segments.first().expect("one segment should be emitted");
        assert_eq!(emitted_segment.text(), "Inner");
        assert_eq!(emitted_segment.provenance().xml_id(), Some("p1"));
    }

    #[test]
    fn empty_normalized_segments_are_not_emitted() {
        let mut collector = SegmentCollector::default();
        let paragraph = element(P);
        let paragraph_frame = frame(P, "/TEI/text/body/p[1]");

        collector
            .maybe_start_segment(P, &paragraph, &paragraph_frame)
            .expect("paragraph should start");
        collector.push_text("   ", true, false);
        collector.push_boundary();
        collector
            .finish_active_segment()
            .expect("empty paragraph should finish");

        let segments = collector.finish().expect("collector should finish");
        assert!(segments.is_empty());
    }

    #[test]
    fn finish_active_segment_errors_without_active_segment() {
        let mut collector = SegmentCollector::default();

        let error = collector
            .finish_active_segment()
            .expect_err("finishing without an active segment should fail");

        assert!(error.to_string().contains("no active spoken segment"));
    }

    #[test]
    fn finish_errors_with_unfinished_segment() {
        let mut collector = SegmentCollector::default();
        let paragraph = element(P);
        let paragraph_frame = frame(P, "/TEI/text/body/p[1]");

        collector
            .maybe_start_segment(P, &paragraph, &paragraph_frame)
            .expect("paragraph should start");
        let error = collector
            .finish()
            .expect_err("unfinished active segment should fail");

        assert!(error.to_string().contains("unfinished spoken segment"));
    }
}
