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

pub(super) const MAX_DIV_DEPTH: usize = 128;

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

    /// A nested structural container exceeded the supported recursion limit.
    #[error("{container} nesting exceeds maximum supported depth of {max_depth}")]
    TooDeep {
        /// Name of the container whose nesting exceeded the limit.
        container: &'static str,
        /// Maximum supported nesting depth.
        max_depth: usize,
    },
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
            BodyBlock::Div(div) => {
                validate_div(div, seen_ids, known_speakers, 0)?;
            }
        }
    }

    Ok(())
}

fn validate_div(
    div: &crate::Div,
    seen_ids: &mut HashSet<String>,
    known_speakers: Option<&HashSet<String>>,
    current_depth: usize,
) -> Result<(), ValidationError> {
    use crate::DivContent;

    ensure_within_max_depth("div", current_depth)?;

    if let Some(identifier) = div.id() {
        identifiers::record_id(identifier.as_str(), seen_ids)?;
    }

    for content in div.content() {
        match content {
            DivContent::Paragraph(paragraph) => {
                if let Some(identifier) = paragraph.id() {
                    identifiers::record_id(identifier.as_str(), seen_ids)?;
                }
            }
            DivContent::Utterance(utterance) => {
                if let Some(identifier) = utterance.id() {
                    identifiers::record_id(identifier.as_str(), seen_ids)?;
                }
                validate_speaker_reference(utterance, known_speakers)?;
            }
            DivContent::List(list) => validate_list(list, seen_ids, current_depth + 1)?,
            DivContent::Div(nested_div) => {
                validate_div(nested_div, seen_ids, known_speakers, current_depth + 1)?;
            }
        }
    }

    Ok(())
}

fn validate_list(
    list: &crate::List,
    seen_ids: &mut HashSet<String>,
    current_depth: usize,
) -> Result<(), ValidationError> {
    ensure_within_max_depth("list", current_depth)?;

    if let Some(identifier) = list.id() {
        identifiers::record_id(identifier.as_str(), seen_ids)?;
    }

    for item in list.items() {
        if let Some(identifier) = item.id() {
            identifiers::record_id(identifier.as_str(), seen_ids)?;
        }
    }

    Ok(())
}

const fn ensure_within_max_depth(
    container: &'static str,
    current_depth: usize,
) -> Result<(), ValidationError> {
    if current_depth >= MAX_DIV_DEPTH {
        return Err(ValidationError::TooDeep {
            container,
            max_depth: MAX_DIV_DEPTH,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests for document-wide validation invariants.

    use rstest::{fixture, rstest};

    use super::*;
    use crate::{
        Certainty, CiteData, CiteStructure, Div, EncodingDesc, FileDesc, Pointer, PointerList,
        RefsDecl, Span, SpanGroup, StandOff, TeiBody, TeiHeader, TeiText, Utterance,
    };

    #[fixture]
    fn document_with_stand_off() -> Result<TeiDocument, Box<dyn std::error::Error>> {
        let header = TeiHeader::new(FileDesc::from_title_str("Fixture")?);
        let mut utterance = Utterance::from_text_segments(Some("host"), ["Hello"])?;
        utterance.set_id("u1")?;

        let mut span = Span::new();
        span.set_id("sp1")?;
        span.set_target(PointerList::new(["#u1"])?);
        span.set_cert(Certainty::new("high")?);
        span.set_from(Pointer::new("#u1")?);

        let mut span_group = SpanGroup::new("citation")?;
        span_group.set_id("grp1")?;
        span_group.add_span(span);

        let mut stand_off = StandOff::new();
        stand_off.add_span_group(span_group);

        Ok(TeiDocument::new(
            header,
            TeiText::new(TeiBody::new([BodyBlock::Utterance(utterance)])),
        )
        .with_stand_off(stand_off))
    }

    #[rstest]
    fn accepts_resolved_stand_off_pointers(
        #[from(document_with_stand_off)] document_res: Result<
            TeiDocument,
            Box<dyn std::error::Error>,
        >,
    ) {
        let document = document_res.expect("fixture document");
        assert!(validate_document(&document).is_ok());
    }

    #[test]
    fn rejects_divisions_that_exceed_maximum_depth() {
        let header = TeiHeader::new(
            FileDesc::from_title_str("Too deep").unwrap_or_else(|error| panic!("title: {error}")),
        );
        let mut root_div = Div::new("section").unwrap_or_else(|error| panic!("root div: {error}"));

        for _ in 0..MAX_DIV_DEPTH {
            let mut wrapper =
                Div::new("section").unwrap_or_else(|error| panic!("wrapper div: {error}"));
            wrapper.push_div(root_div);
            root_div = wrapper;
        }

        let document = TeiDocument::new(
            header,
            TeiText::new(TeiBody::new([BodyBlock::Div(root_div)])),
        );

        assert_eq!(
            validate_document(&document),
            Err(ValidationError::TooDeep {
                container: "div",
                max_depth: MAX_DIV_DEPTH,
            })
        );
    }

    #[rstest]
    fn rejects_unresolved_internal_pointers(
        #[from(document_with_stand_off)] document_res: Result<
            TeiDocument,
            Box<dyn std::error::Error>,
        >,
    ) {
        let mut document = document_res.expect("fixture document");
        let stand_off = document
            .stand_off_mut()
            .expect("document should contain standOff");
        let group = stand_off
            .find_span_group_mut("grp1")
            .expect("span group should exist");
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
