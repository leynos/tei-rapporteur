//! Inline TEI content such as emphasised runs and pauses.
//!
//! Mixed content is modelled as an [`Inline`] enum so paragraphs and utterances
//! can hold either plain text or nested inline elements.

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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "hi")]
pub struct Hi {
    #[serde(rename = "rend", skip_serializing_if = "Option::is_none", default)]
    rend: Option<String>,
    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
}

impl Hi {
    /// Builds an emphasised inline element.
    #[must_use]
    pub fn new(content: impl IntoIterator<Item = Inline>) -> Self {
        Self {
            rend: None,
            content: content.into_iter().collect(),
        }
    }

    /// Builds an emphasised inline element with a rendering hint.
    #[must_use]
    pub fn with_rend(rend: impl Into<String>, content: impl IntoIterator<Item = Inline>) -> Self {
        Self {
            rend: Some(rend.into()),
            content: content.into_iter().collect(),
        }
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
    pub fn push_inline(&mut self, inline: Inline) {
        self.content.push(inline);
    }
}

/// Pause marker rendered as `<pause/>`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "pause")]
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

    #[test]
    fn hi_records_children() {
        let hi = Hi::new([Inline::text("emphasis")]);

        let content = hi.content();
        assert_eq!(content.len(), 1);
        assert_eq!(content.first().and_then(Inline::as_text), Some("emphasis"));
    }

    #[test]
    fn pause_records_duration_and_kind() {
        let mut pause = Pause::new();
        pause.set_duration("PT1S");
        pause.set_kind("breath");

        assert_eq!(pause.duration(), Some("PT1S"));
        assert_eq!(pause.kind(), Some("breath"));
    }
}
