//! Document-level validation for TEI structures.
//!
//! The routines in this module enforce invariants that the type system does
//! not capture, such as uniqueness of `xml:id` values, the relationship
//! between utterance speaker references and the header cast list, and
//! resolution of internal TEI pointers.

use std::collections::HashSet;

use thiserror::Error;

use crate::{BodyBlock, CiteStructure, Pointer, PointerList, Span, TeiDocument, Utterance};

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

fn validate_stand_off_structure(
    document: &TeiDocument,
    seen_ids: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    let Some(stand_off) = document.stand_off() else {
        return Ok(());
    };

    for span_group in stand_off.span_groups() {
        validate_non_empty_field(span_group.kind(), "spanGrp @type")?;
        if let Some(identifier) = span_group.id() {
            record_id(identifier.as_str(), seen_ids)?;
        }

        for span in span_group.spans() {
            if let Some(identifier) = span.id() {
                record_id(identifier.as_str(), seen_ids)?;
            }
            validate_span_structure(span)?;
        }
    }

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
    }
    Ok(())
}

fn validate_refs_decl(document: &TeiDocument) -> Result<(), ValidationError> {
    let Some(encoding) = document.header().encoding_desc() else {
        return Ok(());
    };
    let Some(refs_decl) = encoding.refs_decl() else {
        return Ok(());
    };

    for cite_structure in refs_decl.cite_structures() {
        validate_cite_structure(cite_structure)?;
    }

    Ok(())
}

fn validate_cite_structure(cite_structure: &CiteStructure) -> Result<(), ValidationError> {
    validate_non_empty_field(cite_structure.match_expr(), "citeStructure @match")?;

    for cite_data in cite_structure.cite_data() {
        validate_non_empty_field(cite_data.property(), "citeData @property")?;
    }

    for child in cite_structure.children() {
        validate_cite_structure(child)?;
    }

    Ok(())
}

fn validate_internal_pointers(
    document: &TeiDocument,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    for block in document.text().body().blocks() {
        if let BodyBlock::Utterance(utterance) = block {
            validate_utterance_pointers(utterance, known_ids)?;
        }
    }

    let Some(stand_off) = document.stand_off() else {
        return Ok(());
    };

    for span_group in stand_off.span_groups() {
        validate_pointer_list("@resp", span_group.resp(), known_ids)?;
        validate_pointer_list("@corresp", span_group.corresp(), known_ids)?;
        validate_pointer_list("@ana", span_group.ana(), known_ids)?;

        for span in span_group.spans() {
            validate_pointer_list("@target", span.target(), known_ids)?;
            validate_pointer("@from", span.from(), known_ids)?;
            validate_pointer("@to", span.to(), known_ids)?;
            validate_pointer_list("@source", span.source(), known_ids)?;
            validate_pointer_list("@resp", span.resp(), known_ids)?;
            validate_pointer_list("@corresp", span.corresp(), known_ids)?;
            validate_pointer_list("@ana", span.ana(), known_ids)?;
        }
    }

    Ok(())
}

fn validate_utterance_pointers(
    utterance: &Utterance,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    validate_pointer_list("@source", utterance.source(), known_ids)?;
    validate_pointer_list("@resp", utterance.resp(), known_ids)?;
    validate_pointer_list("@corresp", utterance.corresp(), known_ids)?;
    validate_pointer_list("@ana", utterance.ana(), known_ids)?;
    Ok(())
}

fn validate_pointer_list(
    attribute: &'static str,
    pointer_list: Option<&PointerList>,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    let Some(values) = pointer_list else {
        return Ok(());
    };

    for pointer in values.iter() {
        validate_pointer(attribute, Some(pointer), known_ids)?;
    }

    Ok(())
}

fn validate_pointer(
    attribute: &'static str,
    candidate: Option<&Pointer>,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    let Some(pointer) = candidate else {
        return Ok(());
    };

    let Some(target_id) = pointer.internal_id() else {
        return Ok(());
    };

    if known_ids.contains(target_id) {
        Ok(())
    } else {
        Err(ValidationError::UnresolvedPointer {
            attribute,
            pointer: pointer.as_str().to_owned(),
        })
    }
}

const fn validate_span_structure(span: &Span) -> Result<(), ValidationError> {
    if span.target().is_none() && span.from().is_none() {
        return Err(ValidationError::SpanMissingAnchor);
    }

    if span.to().is_some() && span.from().is_none() {
        return Err(ValidationError::SpanToWithoutFrom);
    }

    Ok(())
}

fn validate_non_empty_field(value: &str, field: &'static str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError::EmptyField { field })
    } else {
        Ok(())
    }
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
    utterance: &Utterance,
    known_speakers: Option<&HashSet<String>>,
) -> Result<(), ValidationError> {
    let Some(speakers) = known_speakers else {
        return Ok(());
    };
    let Some(speaker) = utterance.speaker() else {
        return Ok(());
    };

    if speakers.contains(speaker.as_str()) {
        Ok(())
    } else {
        Err(ValidationError::UnknownSpeaker {
            speaker: speaker.as_str().to_owned(),
        })
    }
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
    //! Unit tests for document-wide validation invariants.

    use super::*;
    use crate::{
        Certainty, CiteData, CiteStructure, EncodingDesc, FileDesc, Pointer, PointerList, RefsDecl,
        Span, SpanGroup, StandOff, TeiBody, TeiHeader, TeiText, Utterance,
    };

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

        let mut span_group = SpanGroup::new("citation");
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

    #[test]
    fn accepts_resolved_stand_off_pointers() {
        let document = document_with_stand_off();
        assert!(validate_document(&document).is_ok());
    }

    #[test]
    fn rejects_unresolved_internal_pointers() {
        let mut document = document_with_stand_off();
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
}
