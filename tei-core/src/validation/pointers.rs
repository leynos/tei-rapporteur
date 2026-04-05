//! Internal-pointer validation helpers.

use std::collections::HashSet;

use crate::{BodyBlock, Pointer, PointerList, Span, SpanGroup, TeiDocument, Utterance};

use super::ValidationError;

pub(super) fn validate_internal_pointers(
    document: &TeiDocument,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    validate_body_utterance_pointers(document, known_ids)?;
    validate_stand_off_span_group_pointers(document, known_ids)?;
    Ok(())
}

/// Iterates `items`, calling `dispatch` for each one.
/// Centralises the for-loop-plus-early-return pattern shared by the body and
/// div pointer validators.
#[inline]
fn validate_block_iter<T>(
    items: impl IntoIterator<Item = T>,
    known_ids: &HashSet<String>,
    mut dispatch: impl FnMut(T, &HashSet<String>) -> Result<(), ValidationError>,
) -> Result<(), ValidationError> {
    for item in items {
        dispatch(item, known_ids)?;
    }
    Ok(())
}

fn validate_body_utterance_pointers(
    document: &TeiDocument,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    validate_block_iter(
        document.text().body().blocks(),
        known_ids,
        |block, ids| match block {
            BodyBlock::Utterance(utterance) => validate_utterance_pointers(utterance, ids),
            BodyBlock::Div(div) => validate_div_pointers(div, ids),
            BodyBlock::Paragraph(_) => Ok(()),
        },
    )
}

fn validate_div_pointers(
    div: &crate::Div,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    use crate::DivContent;
    validate_block_iter(div.content(), known_ids, |content, ids| match content {
        DivContent::Utterance(utterance) => validate_utterance_pointers(utterance, ids),
        DivContent::List(list) => validate_list_pointers(list, ids),
        DivContent::Paragraph(_) => Ok(()),
    })
}

fn validate_list_pointers(
    list: &crate::List,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    for item in list.items() {
        validate_pointer_list("@corresp", item.corresp(), known_ids)?;
    }
    Ok(())
}

fn validate_stand_off_span_group_pointers(
    document: &TeiDocument,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    let Some(stand_off) = document.stand_off() else {
        return Ok(());
    };
    for span_group in stand_off.span_groups() {
        validate_span_group_pointers(span_group, known_ids)?;
    }
    Ok(())
}

pub(super) fn validate_span_group_pointers(
    span_group: &SpanGroup,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    validate_pointer_list("@resp", span_group.resp(), known_ids)?;
    validate_pointer_list("@corresp", span_group.corresp(), known_ids)?;
    validate_pointer_list("@ana", span_group.ana(), known_ids)?;

    for span in span_group.spans() {
        validate_span_pointers(span, known_ids)?;
    }

    Ok(())
}

pub(super) fn validate_span_pointers(
    span: &Span,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    validate_pointer_list("@target", span.target(), known_ids)?;
    validate_pointer("@from", span.from(), known_ids)?;
    validate_pointer("@to", span.to(), known_ids)?;
    validate_pointer_list("@source", span.source(), known_ids)?;
    validate_pointer_list("@resp", span.resp(), known_ids)?;
    validate_pointer_list("@corresp", span.corresp(), known_ids)?;
    validate_pointer_list("@ana", span.ana(), known_ids)?;
    Ok(())
}

pub(super) fn validate_utterance_pointers(
    utterance: &Utterance,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    validate_pointer_list("@source", utterance.source(), known_ids)?;
    validate_pointer_list("@resp", utterance.resp(), known_ids)?;
    validate_pointer_list("@corresp", utterance.corresp(), known_ids)?;
    validate_pointer_list("@ana", utterance.ana(), known_ids)?;
    Ok(())
}

pub(super) fn validate_pointer_list(
    attribute: &'static str,
    pointer_list: Option<&PointerList>,
    known_ids: &HashSet<String>,
) -> Result<(), ValidationError> {
    let Some(values) = pointer_list else {
        return Ok(());
    };

    for pointer in values.iter() {
        let Some(target_id) = pointer.internal_id() else {
            continue;
        };
        if !known_ids.contains(target_id) {
            return Err(ValidationError::UnresolvedPointer {
                attribute,
                pointer: pointer.as_str().to_owned(),
            });
        }
    }

    Ok(())
}

pub(super) fn validate_pointer(
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
