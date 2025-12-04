//! Document-level validation for TEI structures.
//!
//! The routines in this module enforce invariants that the type system does
//! not capture, such as uniqueness of `xml:id` values and the relationship
//! between utterance speaker references and the header cast list.

use std::collections::HashSet;

use thiserror::Error;

use crate::{TeiDocument, text::BodyBlock};

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

/// Validates document-wide invariants for a [`TeiDocument`].
///
/// Checks that all `xml:id` values are unique across annotation systems,
/// paragraphs, and utterances, and that utterance speaker references match the
/// declared cast list when present.
///
/// # Errors
///
/// Returns [`ValidationError::DuplicateXmlId`] if any `xml:id` appears more
/// than once. Returns [`ValidationError::UnknownSpeaker`] if an utterance
/// references a speaker not declared in the profile description's cast list
/// (when a cast has been provided).
pub(crate) fn validate_document(document: &TeiDocument) -> Result<(), ValidationError> {
    let mut seen_ids: HashSet<String> = HashSet::new();

    validate_annotation_systems(document, &mut seen_ids)?;

    let known_speakers = extract_known_speakers(document);

    validate_body_blocks(document, &mut seen_ids, known_speakers.as_ref())?;

    Ok(())
}

fn validate_annotation_systems(
    document: &TeiDocument,
    seen_ids: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    let Some(encoding) = document.header().encoding_desc() else {
        return Ok(());
    };

    for system in encoding.annotation_systems() {
        record_id(system.identifier().as_str(), seen_ids)?;
    }

    Ok(())
}

fn validate_body_blocks(
    document: &TeiDocument,
    seen_ids: &mut HashSet<String>,
    known_speakers: Option<&HashSet<String>>,
) -> Result<(), ValidationError> {
    for block in document.text().body().blocks() {
        validate_body_block(block, seen_ids, known_speakers)?;
    }
    Ok(())
}

fn validate_body_block(
    block: &BodyBlock,
    seen_ids: &mut HashSet<String>,
    known_speakers: Option<&HashSet<String>>,
) -> Result<(), ValidationError> {
    match block {
        BodyBlock::Paragraph(paragraph) => {
            if let Some(identifier) = paragraph.id() {
                record_id(identifier.as_str(), seen_ids)?;
            }
        }
        BodyBlock::Utterance(utterance) => {
            if let Some(identifier) = utterance.id() {
                record_id(identifier.as_str(), seen_ids)?;
            }
            validate_speaker_reference(utterance, known_speakers)?;
        }
    }
    Ok(())
}

fn extract_known_speakers(document: &TeiDocument) -> Option<HashSet<String>> {
    document.header().profile_desc().map(|profile| {
        profile
            .speakers()
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect::<HashSet<_>>()
    })
}

fn validate_speaker_reference(
    utterance: &crate::text::Utterance,
    known_speakers: Option<&HashSet<String>>,
) -> Result<(), ValidationError> {
    // Early return if no cast list exists - speakers are allowed without validation
    let Some(speakers) = known_speakers else {
        return Ok(());
    };

    // Early return if utterance has no speaker reference
    let Some(speaker) = utterance.speaker() else {
        return Ok(());
    };

    if speakers.contains(speaker.as_str()) {
        return Ok(());
    }

    Err(ValidationError::UnknownSpeaker {
        speaker: speaker.as_str().to_owned(),
    })
}

fn record_id(value: &str, sink: &mut HashSet<String>) -> Result<(), ValidationError> {
    if sink.insert(value.to_owned()) {
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
        header::{AnnotationSystem, EncodingDesc, FileDesc, ProfileDesc, TeiHeader},
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
        base_header: TeiHeader,
        encoding: EncodingDesc,
        paragraph_with_id: P,
        utterance_with_id: Utterance,
    ) {
        let mut profile = ProfileDesc::new();
        profile
            .add_speaker("host")
            .expect("speaker should validate");
        let header_with_cast = base_header
            .with_profile_desc(profile)
            .with_encoding_desc(encoding);

        let text = TeiText::new(TeiBody::new([
            BodyBlock::Paragraph(paragraph_with_id),
            BodyBlock::Utterance(utterance_with_id),
        ]));

        let document = TeiDocument::new(header_with_cast, text);

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
    fn rejects_header_body_duplicate_xml_ids(base_header: TeiHeader) {
        let mut encoding = EncodingDesc::new();
        let system = AnnotationSystem::new("shared", "annotations")
            .expect("annotation system id should validate");
        encoding.add_annotation_system(system);
        let header = base_header.with_encoding_desc(encoding);

        let mut paragraph =
            P::from_text_segments(["Hello"]).expect("paragraph should accept content");
        paragraph
            .set_id("shared")
            .expect("paragraph identifier should validate");
        let text = TeiText::new(TeiBody::new([BodyBlock::Paragraph(paragraph)]));

        let document = TeiDocument::new(header, text);
        let result = validate_document(&document);

        assert_eq!(
            result,
            Err(ValidationError::DuplicateXmlId {
                id: String::from("shared"),
            })
        );
    }

    #[rstest]
    fn rejects_unknown_speaker_references_when_profile_exists(base_header: TeiHeader) {
        let mut profile = ProfileDesc::new();
        profile
            .add_speaker("known")
            .expect("speaker should validate");
        let header_with_cast = base_header.with_profile_desc(profile);

        let utterance = Utterance::from_text_segments(Some("unknown"), ["Hi there"])
            .expect("utterance should accept content");
        let text = TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)]));

        let document = TeiDocument::new(header_with_cast, text);

        let result = validate_document(&document);

        assert_eq!(
            result,
            Err(ValidationError::UnknownSpeaker {
                speaker: String::from("unknown"),
            })
        );
    }

    #[rstest]
    fn rejects_speakers_when_cast_is_empty(base_header: TeiHeader) {
        let header_with_empty_cast = base_header.with_profile_desc(ProfileDesc::new());

        let utterance = Utterance::from_text_segments(Some("ghost"), ["Boo"])
            .expect("utterance should accept content");
        let text = TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)]));

        let document = TeiDocument::new(header_with_empty_cast, text);

        let result = validate_document(&document);

        assert_eq!(
            result,
            Err(ValidationError::UnknownSpeaker {
                speaker: String::from("ghost"),
            })
        );
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
