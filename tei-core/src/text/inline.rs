//! Inline TEI content such as emphasised runs and pauses.
//!
//! Mixed content is modelled as an [`Inline`] enum so paragraphs and utterances
//! can hold either plain text or nested inline elements.

use super::body::{BodyContentError, ensure_container_content, push_validated_inline};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

/// Inline content occurring inside paragraphs and utterances.
///
/// # Examples
///
/// ```
/// use tei_core::{Hi, Inline, P};
///
/// let emphasis = Inline::Hi(Hi::new([Inline::text("important")]));
/// let paragraph = P::from_inline([Inline::text("An "), emphasis]).expect("valid paragraph");
///
/// assert_eq!(paragraph.content().len(), 2);
/// ```
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Inline {
    /// Plain text content.
    Text(String),
    /// Emphasised content wrapped in `<hi>`.
    Hi(Hi),
    /// A pause marker rendered as `<pause/>`.
    Pause(Pause),
}

impl Inline {
    /// Builds a plain text inline node.
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text(content.into())
    }

    /// Builds an emphasised inline node.
    #[must_use]
    pub fn hi(content: impl IntoIterator<Item = Self>) -> Self {
        Self::Hi(Hi::new(content))
    }

    /// Builds a pause marker.
    #[must_use]
    pub const fn pause() -> Self {
        Self::Pause(Pause::new())
    }

    /// Returns the contained text when this variant is [`Inline::Text`].
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on the current MSRV."
    )]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value.as_str()),
            _ => None,
        }
    }
}

/// Emphasised inline element corresponding to `<hi>`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename = "hi")]
pub struct Hi {
    #[serde(rename = "rend", skip_serializing_if = "Option::is_none", default)]
    rend: Option<String>,
    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
}

impl<'de> Deserialize<'de> for Hi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawHi {
            #[serde(rename = "rend", default)]
            rend: Option<String>,
            #[serde(rename = "$value", default)]
            content: Vec<Inline>,
        }

        let raw = RawHi::deserialize(deserializer)?;
        ensure_container_content(&raw.content, "hi").map_err(de::Error::custom)?;

        Ok(Self {
            rend: raw.rend,
            content: raw.content,
        })
    }
}

impl Hi {
    /// Builds an emphasised inline element without validating the content.
    #[must_use]
    pub fn new(content: impl IntoIterator<Item = Inline>) -> Self {
        Self::from_parts(None, content.into_iter().collect())
    }

    /// Builds an emphasised inline element with a rendering hint without
    /// validating the content.
    #[must_use]
    pub fn with_rend(rend: impl Into<String>, content: impl IntoIterator<Item = Inline>) -> Self {
        Self::from_parts(Some(rend.into()), content.into_iter().collect())
    }

    /// Builds an emphasised inline element, validating that content contains
    /// visible segments.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when all inline children are
    /// empty after trimming or when nested emphasis elements contain no
    /// meaningful content.
    pub fn try_new(content: impl IntoIterator<Item = Inline>) -> Result<Self, BodyContentError> {
        let collected: Vec<Inline> = content.into_iter().collect();
        ensure_container_content(&collected, "hi")?;

        Ok(Self::from_parts(None, collected))
    }

    /// Builds an emphasised inline element with a rendering hint, validating
    /// that content contains visible segments.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when all inline children are
    /// empty after trimming or when nested emphasis elements contain no
    /// meaningful content.
    pub fn try_with_rend(
        rend: impl Into<String>,
        content: impl IntoIterator<Item = Inline>,
    ) -> Result<Self, BodyContentError> {
        let collected: Vec<Inline> = content.into_iter().collect();
        ensure_container_content(&collected, "hi")?;

        Ok(Self::from_parts(Some(rend.into()), collected))
    }

    /// Returns the optional rendering hint.
    #[must_use]
    pub fn rend(&self) -> Option<&str> {
        self.rend.as_deref()
    }

    /// Assigns a rendering hint.
    pub fn set_rend(&mut self, rend: impl Into<String>) {
        self.rend = Some(rend.into());
    }

    /// Removes the rendering hint.
    pub fn clear_rend(&mut self) {
        self.rend = None;
    }

    /// Returns the inline children.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec::as_slice is not const-stable on the current MSRV."
    )]
    pub fn content(&self) -> &[Inline] {
        self.content.as_slice()
    }

    /// Appends an inline child.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the inline text lacks
    /// visible characters. Returns [`BodyContentError::EmptyContent`] when a
    /// nested inline element has no meaningful children.
    pub fn push_inline(&mut self, inline: Inline) -> Result<(), BodyContentError> {
        push_validated_inline(&mut self.content, inline, "hi")
    }

    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec values are not const-constructible on the current MSRV."
    )]
    fn from_parts(rend: Option<String>, content: Vec<Inline>) -> Self {
        Self { rend, content }
    }
}

/// Pause marker rendered as `<pause/>`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "pause", deny_unknown_fields)]
pub struct Pause {
    #[serde(rename = "dur", skip_serializing_if = "Option::is_none", default)]
    duration: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pause_type: Option<String>,
}

impl Pause {
    /// Creates an empty pause marker.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            duration: None,
            pause_type: None,
        }
    }

    /// Returns the recorded duration.
    #[must_use]
    pub fn duration(&self) -> Option<&str> {
        self.duration.as_deref()
    }

    /// Assigns a duration value.
    pub fn set_duration(&mut self, duration: impl Into<String>) {
        self.duration = Some(duration.into());
    }

    /// Clears the recorded duration.
    pub fn clear_duration(&mut self) {
        self.duration = None;
    }

    /// Returns the pause classification.
    #[must_use]
    pub fn kind(&self) -> Option<&str> {
        self.pause_type.as_deref()
    }

    /// Assigns a pause classification.
    pub fn set_kind(&mut self, kind: impl Into<String>) {
        self.pause_type = Some(kind.into());
    }

    /// Clears the pause classification.
    pub fn clear_kind(&mut self) {
        self.pause_type = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::BodyContentError;
    use rstest::{fixture, rstest};
    use serde_json as json;

    #[fixture]
    fn emphasised_inline() -> Inline {
        Inline::text("emphasis")
    }

    #[fixture]
    fn empty_pause() -> Pause {
        Pause::new()
    }

    fn assert_inline_deserialisation_error(
        payload: &str,
        expected_error_substring: &str,
        description: &str,
    ) {
        let error = json::from_str::<Inline>(payload).expect_err(description);

        assert!(
            error.to_string().contains(expected_error_substring),
            "{description}: {error}"
        );
    }

    #[rstest]
    fn hi_records_children(emphasised_inline: Inline) {
        let hi = Hi::try_new([emphasised_inline.clone()]).expect("valid emphasis");

        let content = hi.content();
        assert_eq!(content.len(), 1);
        assert_eq!(content.first().and_then(Inline::as_text), Some("emphasis"));
    }

    #[rstest]
    fn pause_records_duration_and_kind(mut empty_pause: Pause) {
        empty_pause.set_duration("PT1S");
        empty_pause.set_kind("breath");

        assert_eq!(empty_pause.duration(), Some("PT1S"));
        assert_eq!(empty_pause.kind(), Some("breath"));
    }

    #[rstest]
    fn hi_try_with_rend_records_hint(emphasised_inline: Inline) {
        let hi = Hi::try_with_rend("stress", [emphasised_inline.clone()])
            .expect("valid emphasised inline");

        assert_eq!(hi.rend(), Some("stress"));
        let expected = [Inline::text("emphasis")];
        assert_eq!(hi.content(), expected.as_slice());
    }

    #[rstest]
    fn hi_try_new_rejects_empty_content() {
        let result = Hi::try_new(Vec::<Inline>::new());

        assert!(matches!(
            result,
            Err(BodyContentError::EmptyContent { container }) if container == "hi"
        ));
    }

    #[rstest]
    fn hi_push_inline_rejects_blank_text() {
        let mut hi = Hi::try_new([Inline::text("visible")]).expect("valid emphasis");

        let result = hi.push_inline(Inline::text("   "));

        assert!(matches!(
            result,
            Err(BodyContentError::EmptySegment { container }) if container == "hi"
        ));
    }

    #[rstest]
    fn inline_deserialisation_reports_type_mismatch() {
        assert_inline_deserialisation_error(
            "42",
            "did not match any variant of untagged enum Inline",
            "error message should describe variant mismatch",
        );
    }

    #[rstest]
    fn inline_deserialisation_reports_missing_hi_content() {
        assert_inline_deserialisation_error(
            r#"{"$value":[]}"#,
            "did not match any variant of untagged enum Inline",
            "error message should describe inline variant mismatch",
        );
    }

    #[test]
    fn hi_deserialisation_reports_empty_content() {
        let error = json::from_str::<Hi>(r#"{"$value":[]}"#).expect_err("empty hi should fail");

        assert!(
            error
                .to_string()
                .contains("content must include at least one non-empty segment"),
            "error message should describe empty hi content: {error}"
        );
    }
}
