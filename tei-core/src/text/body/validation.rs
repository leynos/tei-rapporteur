use crate::text::types::{IdentifierValidationError, Speaker, SpeakerValidationError, XmlId};

use super::BodyContentError;

pub(crate) fn ensure_content(
    segments: &[String],
    container: &'static str,
) -> Result<(), BodyContentError> {
    if segments.is_empty() || segments.iter().all(|segment| segment.trim().is_empty()) {
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

pub(crate) fn push_validated_segment(
    segments: &mut Vec<String>,
    segment: impl Into<String>,
    container: &'static str,
) -> Result<(), BodyContentError> {
    let candidate = segment.into();

    if candidate.trim().is_empty() {
        return Err(BodyContentError::EmptySegment { container });
    }

    segments.push(candidate);
    Ok(())
}
