//! Document-level validation for TEI structures.
//!
//! The routines in this module enforce invariants that the type system does
//! not capture, such as uniqueness of `xml:id` values and the relationship
//! between utterance speaker references and the header cast list.

use std::collections::HashSet;

use thiserror::Error;

use crate::{
    TeiDocument,
    header::{EncodingDesc, SpeakerName},
    text::BodyBlock,
};

/// Errors raised when validating a [`TeiDocument`].
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ValidationError {
    /// A repeated `xml:id` was detected.
    #[error("duplicate xml:id '{id}'")]
    DuplicateXmlId {
        /// The duplicated identifier.
        id: String,
    },

    /// An utterance referenced an undeclared speaker.
    #[error("utterance speaker '{speaker}' is not declared in the profile")]
    UnknownSpeaker {
        /// Speaker reference captured from the utterance.
        speaker: String,
    },
}

/// Validates invariants that span the full TEI document.
pub(crate) fn validate_document(document: &TeiDocument) -> Result<(), ValidationError> {
    let mut seen_ids: HashSet<&str> = HashSet::new();

    collect_ids_and_validate_speakers(document, &mut seen_ids)
}

fn collect_ids_and_validate_speakers<'doc>(
    document: &'doc TeiDocument,
    sink: &mut HashSet<&'doc str>,
) -> Result<(), ValidationError> {
    if let Some(encoding) = document.header().encoding_desc() {
        collect_annotation_system_ids(encoding, sink)?;
    }

    let known_speakers = extract_known_speakers(document);

    for block in document.text().body().blocks() {
        process_body_block(block, sink, known_speakers.as_ref())?;
    }

    Ok(())
}

fn extract_known_speakers(document: &TeiDocument) -> Option<HashSet<&str>> {
    document.header().profile_desc().map(|profile| {
        profile
            .speakers()
            .iter()
            .map(SpeakerName::as_str)
            .collect::<HashSet<_>>()
    })
}

fn process_body_block<'doc>(
    block: &'doc BodyBlock,
    sink: &mut HashSet<&'doc str>,
    known_speakers: Option<&HashSet<&'doc str>>,
) -> Result<(), ValidationError> {
    match block {
        BodyBlock::Paragraph(paragraph) => process_paragraph(paragraph, sink),
        BodyBlock::Utterance(utterance) => process_utterance(utterance, sink, known_speakers),
    }
}

fn process_paragraph<'doc>(
    paragraph: &'doc crate::text::P,
    sink: &mut HashSet<&'doc str>,
) -> Result<(), ValidationError> {
    if let Some(identifier) = paragraph.id() {
        record_id(identifier.as_str(), sink)?;
    }
    Ok(())
}

fn process_utterance<'doc>(
    utterance: &'doc crate::text::Utterance,
    sink: &mut HashSet<&'doc str>,
    known_speakers: Option<&HashSet<&'doc str>>,
) -> Result<(), ValidationError> {
    if let Some(identifier) = utterance.id() {
        record_id(identifier.as_str(), sink)?;
    }

    validate_speaker_reference(utterance, known_speakers)?;

    Ok(())
}

fn validate_speaker_reference(
    utterance: &crate::text::Utterance,
    known_speakers: Option<&HashSet<&str>>,
) -> Result<(), ValidationError> {
    // Early return if no cast list exists - speakers are allowed without validation
    let Some(speakers) = known_speakers else {
        return Ok(());
    };

    // Early return if utterance has no speaker reference
    let Some(speaker) = utterance.speaker() else {
        return Ok(());
    };

    if speakers.is_empty() {
        return Ok(());
    }

    // Check if speaker is declared in the cast
    if is_speaker_declared(speakers, speaker.as_str()) {
        return Ok(());
    }

    Err(ValidationError::UnknownSpeaker {
        speaker: speaker.as_str().to_owned(),
    })
}

/// Returns true if the speaker is declared in a non-empty cast list.
fn is_speaker_declared(speakers: &HashSet<&str>, speaker: &str) -> bool {
    !speakers.is_empty() && speakers.contains(speaker)
}

fn collect_annotation_system_ids<'doc>(
    encoding: &'doc EncodingDesc,
    sink: &mut HashSet<&'doc str>,
) -> Result<(), ValidationError> {
    for system in encoding.annotation_systems() {
        record_id(system.identifier().as_str(), sink)?;
    }

    Ok(())
}

fn record_id<'doc>(value: &'doc str, sink: &mut HashSet<&'doc str>) -> Result<(), ValidationError> {
    if sink.insert(value) {
        Ok(())
    } else {
        Err(ValidationError::DuplicateXmlId {
            id: value.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        header::{EncodingDesc, FileDesc, ProfileDesc, TeiHeader},
        text::{BodyBlock, P, TeiBody, TeiText, Utterance},
        title::DocumentTitle,
    };
    use rstest::{fixture, rstest};

    #[fixture]
    fn document_title() -> DocumentTitle {
        DocumentTitle::new("Intro").expect("title should validate")
    }

    #[fixture]
    fn base_header(document_title: DocumentTitle) -> TeiHeader {
        TeiHeader::new(FileDesc::new(document_title))
    }

    #[fixture]
    fn encoding() -> EncodingDesc {
        let mut encoding = EncodingDesc::new();
        let system = crate::AnnotationSystem::new("sys1", "annotations")
            .expect("annotation system id should validate");
        encoding.add_annotation_system(system);
        encoding
    }

    #[fixture]
    fn paragraph_with_id() -> P {
        let mut paragraph =
            P::from_text_segments(["Hello"]).expect("paragraph should accept content");
        paragraph
            .set_id("p1")
            .expect("paragraph identifier should validate");
        paragraph
    }

    #[fixture]
    fn utterance_with_id() -> Utterance {
        let mut utterance = Utterance::from_text_segments(Some("host"), ["Hi there"])
            .expect("utterance should accept content");
        utterance
            .set_id("u1")
            .expect("utterance identifier should validate");
        utterance
    }

    #[rstest]
    fn accepts_unique_identifiers_and_known_speakers(
        mut base_header: TeiHeader,
        encoding: EncodingDesc,
        paragraph_with_id: P,
        utterance_with_id: Utterance,
    ) {
        let mut profile = ProfileDesc::new();
        profile
            .add_speaker("host")
            .expect("speaker should validate");
        base_header = base_header.with_profile_desc(profile);
        base_header = base_header.with_encoding_desc(encoding);

        let text = TeiText::new(TeiBody::new([
            BodyBlock::Paragraph(paragraph_with_id),
            BodyBlock::Utterance(utterance_with_id),
        ]));

        let document = TeiDocument::new(base_header, text);

        assert!(validate_document(&document).is_ok());
    }

    #[rstest]
    fn rejects_duplicate_xml_ids(base_header: TeiHeader) {
        let mut first = P::from_text_segments(["Hello"]).expect("paragraph should accept content");
        first
            .set_id("shared")
            .expect("paragraph identifier should validate");

        let mut second = Utterance::from_text_segments(Some("host"), ["Hi there"])
            .expect("utterance should accept content");
        second
            .set_id("shared")
            .expect("utterance identifier should validate");

        let text = TeiText::new(TeiBody::new([
            BodyBlock::Paragraph(first),
            BodyBlock::Utterance(second),
        ]));

        let document = TeiDocument::new(base_header, text);

        let result = validate_document(&document);

        assert_eq!(
            result,
            Err(ValidationError::DuplicateXmlId {
                id: String::from("shared"),
            })
        );
    }

    #[rstest]
    fn rejects_unknown_speaker_references_when_profile_exists(mut base_header: TeiHeader) {
        let mut profile = ProfileDesc::new();
        profile
            .add_speaker("known")
            .expect("speaker should validate");
        base_header = base_header.with_profile_desc(profile);

        let utterance = Utterance::from_text_segments(Some("unknown"), ["Hi there"])
            .expect("utterance should accept content");
        let text = TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)]));

        let document = TeiDocument::new(base_header, text);

        let result = validate_document(&document);

        assert_eq!(
            result,
            Err(ValidationError::UnknownSpeaker {
                speaker: String::from("unknown"),
            })
        );
    }

    #[rstest]
    fn treats_empty_cast_as_unknown(mut base_header: TeiHeader) {
        base_header = base_header.with_profile_desc(ProfileDesc::new());

        let utterance = Utterance::from_text_segments(Some("ghost"), ["Boo"])
            .expect("utterance should accept content");
        let text = TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)]));

        let document = TeiDocument::new(base_header, text);

        assert!(validate_document(&document).is_ok());
    }

    #[rstest]
    fn ignores_speaker_references_when_cast_is_missing(base_header: TeiHeader) {
        let utterance = Utterance::from_text_segments(Some("ghost"), ["Boo"])
            .expect("utterance should accept content");
        let text = TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)]));

        let document = TeiDocument::new(base_header, text);

        assert!(validate_document(&document).is_ok());
    }
}
