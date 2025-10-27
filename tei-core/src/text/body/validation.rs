use crate::text::{
    Inline,
    types::{IdentifierValidationError, Speaker, SpeakerValidationError, XmlId},
};

use super::BodyContentError;

pub(crate) fn ensure_container_content(
    content: &[Inline],
    container: &'static str,
) -> Result<(), BodyContentError> {
    if content.is_empty() {
        return Err(BodyContentError::EmptyContent { container });
    }

    let mut contains_meaningful = false;
    for inline in content {
        contains_meaningful |= validate_inline(inline, container)?;
    }

    if !contains_meaningful {
        return Err(BodyContentError::EmptyContent { container });
    }

    Ok(())
}

pub(crate) fn normalise_optional_speaker<S>(
    speaker: Option<S>,
) -> Result<Option<Speaker>, BodyContentError>
where
    S: Into<String>,
{
    speaker
        .map(Into::into)
        .map_or(Ok(None), |value| match Speaker::try_from(value) {
            Ok(parsed) => Ok(Some(parsed)),
            Err(SpeakerValidationError::Empty) => Err(BodyContentError::EmptySpeaker),
        })
}

pub(crate) fn trim_preserving_original(value: String) -> String {
    let trimmed = value.trim();

    if trimmed.len() == value.len() {
        value
    } else {
        trimmed.to_owned()
    }
}

pub(crate) fn set_optional_identifier(
    field: &mut Option<XmlId>,
    value: impl Into<String>,
    container: &'static str,
) -> Result<(), BodyContentError> {
    match XmlId::try_from(value.into()) {
        Ok(identifier) => {
            *field = Some(identifier);
            Ok(())
        }
        Err(IdentifierValidationError::Empty) => {
            Err(BodyContentError::EmptyIdentifier { container })
        }
        Err(IdentifierValidationError::ContainsWhitespace) => {
            Err(BodyContentError::InvalidIdentifier { container })
        }
    }
}

pub(crate) fn push_validated_text_segment(
    content: &mut Vec<Inline>,
    segment: impl Into<String>,
    container: &'static str,
) -> Result<(), BodyContentError> {
    let inline = Inline::text(segment.into());
    validate_inline(&inline, container)?;
    content.push(inline);

    Ok(())
}

pub(crate) fn push_validated_inline(
    content: &mut Vec<Inline>,
    inline: Inline,
    container: &'static str,
) -> Result<(), BodyContentError> {
    validate_inline(&inline, container)?;
    content.push(inline);

    Ok(())
}

fn validate_inline(inline: &Inline, container: &'static str) -> Result<bool, BodyContentError> {
    match inline {
        Inline::Text(text) => {
            if text.trim().is_empty() {
                return Err(BodyContentError::EmptySegment { container });
            }

            Ok(true)
        }
        Inline::Hi(hi) => ensure_nested_content(hi.content(), container),
        Inline::Pause(_) => Ok(true),
    }
}

fn ensure_nested_content(
    content: &[Inline],
    container: &'static str,
) -> Result<bool, BodyContentError> {
    ensure_container_content(content, container)?;
    Ok(true)
}
