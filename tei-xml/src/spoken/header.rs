//! TEI header recording and validation for spoken-text extraction.

use quick_xml::events::BytesStart;
use tei_core::{TeiError, TeiHeader};

use super::element_names::TEI_HEADER;

/// Buffers a streamed `<teiHeader>` subtree and validates it against the
/// canonical profiled header model.
#[derive(Debug, Default)]
pub(super) struct HeaderRecorder {
    xml: Vec<u8>,
    depth: usize,
    validated: bool,
}

impl HeaderRecorder {
    // `record_start` and `record_empty` intentionally share the
    // `record_element_with_post_step` guard/buffer shell while keeping their
    // post-processing visible: start tags increment depth, and empty root
    // headers validate immediately. Extracting further would hide that split.

    /// Records a start element when it belongs to the TEI header subtree.
    pub(super) fn record_start(
        &mut self,
        name: &str,
        element: &BytesStart<'_>,
    ) -> Result<(), TeiError> {
        self.record_element_with_post_step(name, |s, _n| {
            append_element_with_attributes(&mut s.xml, element, b">")?;
            s.depth += 1;
            Ok(())
        })
    }

    /// Records an empty element when it belongs to the TEI header subtree.
    pub(super) fn record_empty(
        &mut self,
        name: &str,
        element: &BytesStart<'_>,
    ) -> Result<(), TeiError> {
        self.record_element_with_post_step(name, |s, n| {
            append_element_with_attributes(&mut s.xml, element, b"/>")?;
            if n == TEI_HEADER {
                s.validate()?;
            }
            Ok(())
        })
    }

    /// Records a closing element when it belongs to the TEI header subtree.
    pub(super) fn record_end(&mut self, name: &str) -> Result<(), TeiError> {
        if self.depth == 0 {
            return Ok(());
        }
        append_end_element(&mut self.xml, name);
        self.depth -= 1;
        if self.depth == 0 {
            self.validate()?;
        }
        Ok(())
    }

    /// Records raw text bytes when the parser is inside the TEI header.
    pub(super) fn record_raw_text(&mut self, text: &[u8]) {
        if self.depth > 0 {
            self.xml.extend_from_slice(text);
        }
    }

    /// Records CDATA when the parser is inside the TEI header.
    pub(super) fn record_cdata(&mut self, cdata: &[u8]) {
        if self.depth > 0 {
            self.xml.extend_from_slice(b"<![CDATA[");
            self.xml.extend_from_slice(cdata);
            self.xml.extend_from_slice(b"]]>");
        }
    }

    /// Records a general entity reference when the parser is inside the TEI
    /// header.
    pub(super) fn record_general_ref(&mut self, reference: &[u8]) {
        if self.depth > 0 {
            self.xml.push(b'&');
            self.xml.extend_from_slice(reference);
            self.xml.push(b';');
        }
    }

    /// Returns whether the recorded TEI header has passed profile validation.
    pub(super) const fn is_validated(&self) -> bool {
        self.validated
    }

    /// Validates buffered header XML as UTF-8 and as the canonical
    /// `TeiHeader`, returning [`TeiError`] for either failure.
    fn validate(&mut self) -> Result<(), TeiError> {
        let xml = std::str::from_utf8(&self.xml)
            .map_err(|error| TeiError::xml(format!("invalid UTF-8 in teiHeader: {error}")))?;
        // This recorder only validates streaming input; callers that need a
        // `TeiHeader` object must parse the canonical document path.
        quick_xml::de::from_str::<TeiHeader>(xml)
            .map_err(|error| TeiError::xml(format!("invalid teiHeader: {error}")))?;
        self.validated = true;
        Ok(())
    }

    /// Clears the buffer when a new root `<teiHeader>` starts.
    fn reset_if_header_root(&mut self, name: &str) {
        if name == TEI_HEADER && self.depth == 0 {
            self.xml.clear();
            self.validated = false;
        }
    }

    /// Applies the out-of-scope guard, resets the buffer when a new root
    /// `<teiHeader>` starts, and delegates serialization and any post-step to
    /// `on_enter`.
    fn record_element_with_post_step<F>(&mut self, name: &str, on_enter: F) -> Result<(), TeiError>
    where
        F: FnOnce(&mut Self, &str) -> Result<(), TeiError>,
    {
        if name != TEI_HEADER && self.depth == 0 {
            return Ok(());
        }
        self.reset_if_header_root(name);
        on_enter(self, name)?;
        Ok(())
    }
}

/// Serializes an XML element with attributes and a custom closing delimiter.
fn append_element_with_attributes(
    buffer: &mut Vec<u8>,
    element: &BytesStart<'_>,
    closing: &[u8],
) -> Result<(), TeiError> {
    buffer.push(b'<');
    buffer.extend_from_slice(element.name().as_ref());
    for attribute_result in element.attributes() {
        let attribute = attribute_result.map_err(|error| TeiError::xml(error.to_string()))?;
        buffer.push(b' ');
        buffer.extend_from_slice(attribute.key.as_ref());
        buffer.extend_from_slice(b"=\"");
        buffer.extend_from_slice(&attribute.value);
        buffer.push(b'"');
    }
    buffer.extend_from_slice(closing);
    Ok(())
}

/// Serializes an XML closing tag as `</name>`.
fn append_end_element(buffer: &mut Vec<u8>, name: &str) {
    buffer.extend_from_slice(b"</");
    buffer.extend_from_slice(name.as_bytes());
    buffer.push(b'>');
}

#[cfg(test)]
mod tests {
    //! Unit tests for header buffer management, depth tracking, and validation
    //! of streamed `<teiHeader>` subtrees.

    use super::*;

    fn element(name: &str) -> BytesStart<'_> {
        BytesStart::new(name)
    }

    fn start_element_with_attr<'a>(
        name: &'a str,
        attribute_name: &'a str,
        attribute_value: &'a str,
    ) -> BytesStart<'a> {
        let mut element = BytesStart::new(name);
        element.push_attribute((attribute_name, attribute_value));
        element
    }

    fn record_file_desc(recorder: &mut HeaderRecorder, title: &[u8]) -> Result<(), TeiError> {
        recorder.record_start("fileDesc", &element("fileDesc"))?;
        recorder.record_start("title", &element("title"))?;
        recorder.record_raw_text(title);
        recorder.record_end("title")?;
        recorder.record_end("fileDesc")?;
        Ok(())
    }

    fn record_encoding_desc_with_cite_structure(
        recorder: &mut HeaderRecorder,
        cite_match: &str,
        cite_property: &str,
    ) -> Result<(), TeiError> {
        let cite_structure = start_element_with_attr("citeStructure", "match", cite_match);
        let cite_data = start_element_with_attr("citeData", "property", cite_property);
        recorder.record_start("encodingDesc", &element("encodingDesc"))?;
        recorder.record_start("refsDecl", &element("refsDecl"))?;
        recorder.record_start("citeStructure", &cite_structure)?;
        recorder.record_empty("citeData", &cite_data)?;
        Ok(())
    }

    #[test]
    fn validates_when_root_header_closes() {
        let mut recorder = HeaderRecorder::default();

        recorder
            .record_start(TEI_HEADER, &element(TEI_HEADER))
            .expect("teiHeader starts");
        recorder
            .record_start("fileDesc", &element("fileDesc"))
            .expect("fileDesc starts");
        recorder
            .record_start("title", &element("title"))
            .expect("title starts");
        recorder.record_raw_text(b"Spoken Fixture");
        recorder.record_end("title").expect("title ends");
        recorder.record_end("fileDesc").expect("fileDesc ends");

        assert_eq!(recorder.depth, 1);
        assert!(!recorder.is_validated());

        recorder.record_end(TEI_HEADER).expect("teiHeader ends");

        assert_eq!(recorder.depth, 0);
        assert!(recorder.is_validated());
    }

    #[test]
    fn root_header_start_clears_stale_buffer() {
        let mut recorder = HeaderRecorder {
            xml: b"stale".to_vec(),
            validated: true,
            ..HeaderRecorder::default()
        };

        recorder
            .record_start(TEI_HEADER, &element(TEI_HEADER))
            .expect("teiHeader starts");

        assert_eq!(recorder.xml, b"<teiHeader>");
        assert!(!recorder.is_validated());
    }

    #[test]
    fn second_invalid_header_resets_validation_state() {
        let mut recorder = HeaderRecorder::default();

        recorder
            .record_start(TEI_HEADER, &element(TEI_HEADER))
            .expect("first teiHeader starts");
        record_file_desc(&mut recorder, b"First").expect("fileDesc records");
        recorder
            .record_end(TEI_HEADER)
            .expect("first teiHeader validates");
        assert!(recorder.is_validated());

        recorder
            .record_start(TEI_HEADER, &element(TEI_HEADER))
            .expect("second teiHeader starts");
        assert_eq!(recorder.depth, 1);
        assert!(!recorder.is_validated());
        recorder
            .record_end(TEI_HEADER)
            .expect_err("empty second teiHeader should fail validation");

        assert_eq!(recorder.depth, 0);
        assert!(!recorder.is_validated());
    }

    #[test]
    fn content_recorders_are_noops_outside_header() {
        let mut recorder = HeaderRecorder::default();

        recorder.record_raw_text(b"text");
        recorder.record_cdata(b"cdata");
        recorder.record_general_ref(b"amp");

        assert!(recorder.xml.is_empty());
    }

    #[test]
    fn content_recorders_append_inside_header() {
        let mut recorder = HeaderRecorder::default();

        recorder
            .record_start(TEI_HEADER, &element(TEI_HEADER))
            .expect("teiHeader starts");
        recorder.record_raw_text(b"text");
        recorder.record_cdata(b"cdata");
        recorder.record_general_ref(b"amp");

        assert_eq!(recorder.xml, b"<teiHeader>text<![CDATA[cdata]]>&amp;");
    }

    #[test]
    fn invalid_header_utf8_fails_validation() {
        let mut recorder = HeaderRecorder::default();

        recorder
            .record_start(TEI_HEADER, &element(TEI_HEADER))
            .expect("teiHeader starts");
        recorder.record_raw_text(&[0xFF]);
        let error = recorder
            .record_end(TEI_HEADER)
            .expect_err("invalid UTF-8 should fail header validation");

        assert!(error.to_string().contains("invalid UTF-8 in teiHeader"));
        assert!(!recorder.is_validated());
    }

    #[test]
    fn empty_header_fails_validation() {
        let mut recorder = HeaderRecorder::default();

        let error = recorder
            .record_empty(TEI_HEADER, &element(TEI_HEADER))
            .expect_err("empty teiHeader should fail validation");

        assert!(error.to_string().contains("invalid teiHeader"));
        assert!(!recorder.is_validated());
    }

    #[test]
    fn serializes_escaped_attribute_values() {
        let mut recorder = HeaderRecorder::default();

        recorder
            .record_start(TEI_HEADER, &element(TEI_HEADER))
            .expect("teiHeader starts");
        record_file_desc(&mut recorder, b"Spoken Fixture").expect("fileDesc records");
        record_encoding_desc_with_cite_structure(&mut recorder, "A&<>", "speaker")
            .expect("encodingDesc records");

        assert!(
            std::str::from_utf8(&recorder.xml)
                .expect("header buffer is valid UTF-8")
                .contains("match=\"A&amp;&lt;&gt;\"")
        );

        recorder
            .record_end("citeStructure")
            .expect("citeStructure ends");
        recorder.record_end("refsDecl").expect("refsDecl ends");
        recorder
            .record_end("encodingDesc")
            .expect("encodingDesc ends");
        recorder.record_end(TEI_HEADER).expect("teiHeader ends");

        assert!(recorder.is_validated());
    }

    #[test]
    fn deeply_nested_header_content_tracks_depth_until_root_closes() {
        let mut recorder = HeaderRecorder::default();

        recorder
            .record_start(TEI_HEADER, &element(TEI_HEADER))
            .expect("teiHeader starts");
        record_file_desc(&mut recorder, b"Spoken Fixture").expect("fileDesc records");
        record_encoding_desc_with_cite_structure(&mut recorder, "//u", "speaker")
            .expect("encodingDesc records");

        assert_eq!(recorder.depth, 4);
        assert!(!recorder.is_validated());

        recorder
            .record_end("citeStructure")
            .expect("citeStructure ends");
        assert_eq!(recorder.depth, 3);
        recorder.record_end("refsDecl").expect("refsDecl ends");
        assert_eq!(recorder.depth, 2);
        recorder
            .record_end("encodingDesc")
            .expect("encodingDesc ends");
        assert_eq!(recorder.depth, 1);
        recorder.record_end(TEI_HEADER).expect("teiHeader ends");

        assert_eq!(recorder.depth, 0);
        assert!(recorder.is_validated());
    }
}
