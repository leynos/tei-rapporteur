//! Stand-off structure validation helpers.

use std::collections::HashSet;

use crate::{Span, SpanGroup, TeiDocument};

use super::{ValidationError, identifiers::record_id};

pub(super) fn validate_stand_off_structure(
    document: &TeiDocument,
    seen_ids: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    let Some(stand_off) = document.stand_off() else {
        return Ok(());
    };

    for span_group in stand_off.span_groups() {
        validate_span_group_structure(span_group, seen_ids)?;
    }

    Ok(())
}

pub(super) fn validate_span_group_structure(
    span_group: &SpanGroup,
    seen_ids: &mut HashSet<String>,
) -> Result<(), ValidationError> {
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

    Ok(())
}

#[expect(
    clippy::missing_const_for_fn,
    reason = "review requested dropping const to avoid an over-promising validation API"
)]
pub(super) fn validate_span_structure(span: &Span) -> Result<(), ValidationError> {
    if span.target().is_none() && span.from().is_none() {
        return Err(ValidationError::SpanMissingAnchor);
    }

    if span.to().is_some() && span.from().is_none() {
        return Err(ValidationError::SpanToWithoutFrom);
    }

    Ok(())
}

pub(super) fn validate_non_empty_field(
    value: &str,
    field: &'static str,
) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError::EmptyField { field })
    } else {
        Ok(())
    }
}
