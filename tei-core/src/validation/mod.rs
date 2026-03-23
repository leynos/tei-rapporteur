//! Document-level validation for TEI structures.
//!
//! The routines in this module enforce invariants that the type system does
//! not capture, such as uniqueness of `xml:id` values, the relationship
//! between utterance speaker references and the header cast list, and
//! resolution of internal TEI pointers.

mod identifiers;
mod pointers;
mod refs_decl;
mod speakers;
mod stand_off;

use std::collections::HashSet;

use thiserror::Error;

use crate::{BodyBlock, TeiDocument};

use identifiers::validate_annotation_systems;
use pointers::validate_internal_pointers;
use refs_decl::validate_refs_decl;
use speakers::{extract_known_speakers, validate_speaker_reference};
use stand_off::validate_stand_off_structure;

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

    /// A required field was blank after normalization.
    #[error("{field} must not be empty")]
    EmptyField {
        /// Name of the empty field.
        field: &'static str,
    },

    /// An internal pointer target could not be resolved.
    #[error("internal pointer '{pointer}' in {attribute} does not resolve")]
    UnresolvedPointer {
        /// Name of the TEI attribute that carried the pointer.
        attribute: &'static str,
        /// Original pointer token.
        pointer: String,
    },

    /// A stand-off span omitted both `@target` and `@from`.
    #[error("span must define @target or @from")]
    SpanMissingAnchor,

    /// A stand-off span declared `@to` without `@from`.
    #[error("span @to requires @from")]
    SpanToWithoutFrom,
}

/// Validates document-wide invariants for a [`TeiDocument`].
///
/// # Errors
///
/// Returns [`ValidationError`] when duplicated identifiers, invalid citation
/// declarations, unresolved internal pointers, or unknown speakers are
/// detected.
pub(crate) fn validate_document(document: &TeiDocument) -> Result<(), ValidationError> {
    let mut known_ids: HashSet<String> = HashSet::new();

    validate_annotation_systems(document, &mut known_ids)?;
    validate_stand_off_structure(document, &mut known_ids)?;

    let known_speakers = extract_known_speakers(document);
    validate_body_blocks(document, &mut known_ids, known_speakers.as_ref())?;
    validate_refs_decl(document)?;
    validate_internal_pointers(document, &known_ids)?;

    Ok(())
}

fn validate_body_blocks(
    document: &TeiDocument,
    seen_ids: &mut HashSet<String>,
    known_speakers: Option<&HashSet<String>>,
) -> Result<(), ValidationError> {
    for block in document.text().body().blocks() {
        match block {
            BodyBlock::Paragraph(paragraph) => {
                if let Some(identifier) = paragraph.id() {
                    identifiers::record_id(identifier.as_str(), seen_ids)?;
                }
            }
            BodyBlock::Utterance(utterance) => {
                if let Some(identifier) = utterance.id() {
                    identifiers::record_id(identifier.as_str(), seen_ids)?;
                }
                validate_speaker_reference(utterance, known_speakers)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests for document-wide validation invariants.

    use rstest::{fixture, rstest};

    use super::*;
    use crate::{
        Certainty, CiteData, CiteStructure, EncodingDesc, FileDesc, Pointer, PointerList, RefsDecl,
        Span, SpanGroup, StandOff, TeiBody, TeiHeader, TeiText, Utterance,
    };

    #[fixture]
    fn document_with_stand_off() -> TeiDocument {
        let header = TeiHeader::new(
            FileDesc::from_title_str("Fixture").unwrap_or_else(|error| panic!("title: {error}")),
        );
        let mut utterance = Utterance::from_text_segments(Some("host"), ["Hello"])
            .unwrap_or_else(|error| panic!("utterance: {error}"));
        utterance
            .set_id("u1")
            .unwrap_or_else(|error| panic!("utterance id: {error}"));

        let mut span = Span::new();
        span.set_id("sp1")
            .unwrap_or_else(|error| panic!("span id: {error}"));
        span.set_target(
            PointerList::new(["#u1"]).unwrap_or_else(|error| panic!("target pointers: {error}")),
        );
        span.set_cert(Certainty::new("high").unwrap_or_else(|error| panic!("certainty: {error}")));
        span.set_from(Pointer::new("#u1").unwrap_or_else(|error| panic!("from pointer: {error}")));

        let mut span_group =
            SpanGroup::new("citation").unwrap_or_else(|error| panic!("group kind: {error}"));
        span_group
            .set_id("grp1")
            .unwrap_or_else(|error| panic!("group id: {error}"));
        span_group.add_span(span);

        let mut stand_off = StandOff::new();
        stand_off.add_span_group(span_group);

        TeiDocument::new(
            header,
            TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)])),
        )
        .with_stand_off(stand_off)
    }

    #[rstest]
    fn accepts_resolved_stand_off_pointers(document_with_stand_off: TeiDocument) {
        let document = document_with_stand_off;
        assert!(validate_document(&document).is_ok());
    }

    #[rstest]
    fn rejects_unresolved_internal_pointers(document_with_stand_off: TeiDocument) {
        let mut document = document_with_stand_off;
        let stand_off = document
            .stand_off_mut()
            .unwrap_or_else(|| panic!("document should contain standOff"));
        let group = stand_off
            .find_span_group_mut("grp1")
            .unwrap_or_else(|| panic!("span group should exist"));
        let mut span = Span::new();
        span.set_target(
            PointerList::new(["#missing"])
                .unwrap_or_else(|error| panic!("target pointers: {error}")),
        );
        group.add_span(span);

        assert_eq!(
            validate_document(&document),
            Err(ValidationError::UnresolvedPointer {
                attribute: "@target",
                pointer: String::from("#missing"),
            })
        );
    }

    #[test]
    fn rejects_blank_citation_properties() {
        let mut refs_decl = RefsDecl::new();
        let mut cite_structure = CiteStructure::new("//u");
        cite_structure.add_cite_data(CiteData::new("   "));
        refs_decl.add_cite_structure(cite_structure);

        let header = TeiHeader::new(
            FileDesc::from_title_str("Fixture").unwrap_or_else(|error| panic!("title: {error}")),
        )
        .with_encoding_desc(EncodingDesc::new().with_refs_decl(refs_decl));
        let document = TeiDocument::new(header, TeiText::empty());

        assert_eq!(
            validate_document(&document),
            Err(ValidationError::EmptyField {
                field: "citeData @property",
            })
        );
    }

    #[test]
    fn rejects_empty_refs_decl() {
        let document: TeiDocument = tei_serde::json::from_value(tei_serde::serde_json::json!({
            "teiHeader": {
                "fileDesc": { "title": "Fixture" },
                "encodingDesc": { "refsDecl": {} }
            },
            "text": { "body": {} }
        }))
        .unwrap_or_else(|error| panic!("document JSON: {error}"));

        assert_eq!(
            validate_document(&document),
            Err(ValidationError::EmptyField { field: "refsDecl" })
        );
    }
}
