//! Document-level validation for TEI structures.
//!
//! The routines in this module enforce invariants that the type system does
//! not capture, such as uniqueness of `xml:id` values and the relationship
//! between utterance speaker references and the header cast list.

use std::collections::HashSet;

use thiserror::Error;

use crate::{
    TeiDocument,
    header::{EncodingDesc, SpeakerName, TeiHeader},
    text::{BodyBlock, TeiText, Utterance},
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
    let mut seen_ids = HashSet::new();

    collect_header_ids(document.header(), &mut seen_ids)?;
    collect_body_ids(document.text(), &mut seen_ids)?;
    validate_speaker_references(document)?;

    Ok(())
}

fn collect_header_ids(
    header: &TeiHeader,
    sink: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    if let Some(encoding) = header.encoding_desc() {
        add_annotation_system_ids(encoding, sink)?;
    }

    Ok(())
}

fn add_annotation_system_ids(
    encoding: &EncodingDesc,
    sink: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    for system in encoding.annotation_systems() {
        record_id(system.identifier().as_str(), sink)?;
    }

    Ok(())
}

fn collect_body_ids(text: &TeiText, sink: &mut HashSet<String>) -> Result<(), ValidationError> {
    for block in text.body().blocks() {
        match block {
            BodyBlock::Paragraph(paragraph) => {
                if let Some(identifier) = paragraph.id() {
                    record_id(identifier.as_str(), sink)?;
                }
            }
            BodyBlock::Utterance(utterance) => {
                if let Some(identifier) = utterance.id() {
                    record_id(identifier.as_str(), sink)?;
                }
            }
        }
    }

    Ok(())
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

fn validate_speaker_references(document: &TeiDocument) -> Result<(), ValidationError> {
    let Some(profile) = document.header().profile_desc() else {
        return Ok(());
    };

    if profile.speakers().is_empty() {
        return Ok(());
    }

    let known_speakers: HashSet<&str> =
        profile.speakers().iter().map(SpeakerName::as_str).collect();

    for utterance in document.text().body().utterances() {
        check_utterance_speaker(utterance, &known_speakers)?;
    }

    Ok(())
}

fn check_utterance_speaker(
    utterance: &Utterance,
    known_speakers: &HashSet<&str>,
) -> Result<(), ValidationError> {
    let Some(speaker) = utterance.speaker() else {
        return Ok(());
    };

    if known_speakers.contains(speaker.as_str()) {
        Ok(())
    } else {
        Err(ValidationError::UnknownSpeaker {
            speaker: speaker.as_str().to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        header::{EncodingDesc, FileDesc, ProfileDesc, TeiHeader},
        text::{P, TeiBody, TeiText, Utterance},
        title::DocumentTitle,
    };

    #[test]
    fn accepts_unique_identifiers_and_known_speakers() {
        let title = DocumentTitle::new("Intro").expect("title should validate");
        let mut header = TeiHeader::new(FileDesc::new(title));
        let mut profile = ProfileDesc::new();
        profile
            .add_speaker("host")
            .expect("speaker should validate");
        header = header.with_profile_desc(profile);
        header = header.with_encoding_desc(build_encoding("sys1"));

        let mut paragraph =
            P::from_text_segments(["Hello"]).expect("paragraph should accept content");
        paragraph
            .set_id("p1")
            .expect("paragraph identifier should validate");

        let mut utterance = Utterance::from_text_segments(Some("host"), ["Hi there"])
            .expect("utterance should accept content");
        utterance
            .set_id("u1")
            .expect("utterance identifier should validate");

        let text = TeiText::new(TeiBody::new([
            BodyBlock::Paragraph(paragraph),
            BodyBlock::Utterance(utterance),
        ]));

        let document = TeiDocument::new(header, text);

        assert!(validate_document(&document).is_ok());
    }

    #[test]
    fn rejects_duplicate_xml_ids() {
        let title = DocumentTitle::new("Intro").expect("title should validate");
        let header = TeiHeader::new(FileDesc::new(title));

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

        let document = TeiDocument::new(header, text);

        let result = validate_document(&document);

        assert_eq!(
            result,
            Err(ValidationError::DuplicateXmlId {
                id: String::from("shared"),
            })
        );
    }

    #[test]
    fn rejects_unknown_speaker_references_when_profile_exists() {
        let title = DocumentTitle::new("Intro").expect("title should validate");
        let mut header = TeiHeader::new(FileDesc::new(title));
        let mut profile = ProfileDesc::new();
        profile
            .add_speaker("known")
            .expect("speaker should validate");
        header = header.with_profile_desc(profile);

        let utterance = Utterance::from_text_segments(Some("unknown"), ["Hi there"])
            .expect("utterance should accept content");
        let text = TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)]));

        let document = TeiDocument::new(header, text);

        let result = validate_document(&document);

        assert_eq!(
            result,
            Err(ValidationError::UnknownSpeaker {
                speaker: String::from("unknown"),
            })
        );
    }

    #[test]
    fn ignores_speaker_references_when_cast_is_missing() {
        let title = DocumentTitle::new("Intro").expect("title should validate");
        let header = TeiHeader::new(FileDesc::new(title));
        let utterance = Utterance::from_text_segments(Some("ghost"), ["Boo"])
            .expect("utterance should accept content");
        let text = TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)]));

        let document = TeiDocument::new(header, text);

        assert!(validate_document(&document).is_ok());
    }

    fn build_encoding(id: &str) -> EncodingDesc {
        let mut encoding = EncodingDesc::new();
        let system = crate::AnnotationSystem::new(id, "annotations")
            .expect("annotation system id should validate");
        encoding.add_annotation_system(system);

        encoding
    }
}
