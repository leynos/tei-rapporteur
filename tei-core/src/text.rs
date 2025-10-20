/// Placeholder for the body of a TEI document.
///
/// The text model will be expanded in future steps. For now the structure
/// records linear text segments so fixtures can stash script fragments.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TeiText {
    segments: Vec<String>,
}

impl TeiText {
    /// Builds a `TeiText` from the provided segments.
    #[must_use]
    pub fn new<S>(segments: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns an empty text node.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Appends a new text segment.
    pub fn push_segment<S>(&mut self, segment: S)
    where
        S: Into<String>,
    {
        self.segments.push(segment.into());
    }

    /// Returns the stored text segments.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        self.segments.as_slice()
    }

    /// Reports whether any text has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_emptiness() {
        let mut text = TeiText::empty();
        assert!(text.is_empty());

        text.push_segment("intro");
        assert!(!text.is_empty());
    }

    #[test]
    fn collects_segments() {
        let text = TeiText::new(["one", "two"]);
        assert_eq!(text.segments(), ["one", "two"]);
    }
}
