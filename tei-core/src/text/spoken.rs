//! Domain values and normalization helpers for spoken-text extraction.

/// Stable source location for a spoken text segment.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct SpokenTextProvenance {
    xml_id: Option<String>,
    locator: String,
}

impl SpokenTextProvenance {
    /// Builds provenance from an optional `xml:id` and stable locator.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::SpokenTextProvenance;
    ///
    /// let provenance = SpokenTextProvenance::new(
    ///     Some("line-1".to_owned()),
    ///     "/TEI/text/body/p[1]".to_owned(),
    /// );
    /// assert_eq!(provenance.xml_id(), Some("line-1"));
    /// assert_eq!(provenance.locator(), "/TEI/text/body/p[1]");
    /// ```
    #[must_use]
    pub const fn new(xml_id: Option<String>, locator: String) -> Self {
        Self { xml_id, locator }
    }

    /// Returns the optional `xml:id` of the source element.
    #[must_use]
    pub fn xml_id(&self) -> Option<&str> {
        self.xml_id.as_deref()
    }

    /// Returns the stable XPath-like source locator.
    #[must_use]
    pub fn locator(&self) -> &str {
        &self.locator
    }
}

/// One normalized text segment intended to be performed aloud.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct SpokenTextSegment {
    text: String,
    provenance: SpokenTextProvenance,
}

impl SpokenTextSegment {
    /// Builds a spoken segment from normalized text and provenance.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::{SpokenTextProvenance, SpokenTextSegment};
    ///
    /// let provenance = SpokenTextProvenance::new(
    ///     None,
    ///     "/TEI/text/body/p[1]".to_owned(),
    /// );
    /// let segment = SpokenTextSegment::new("Hello.".to_owned(), provenance);
    /// assert_eq!(segment.text(), "Hello.");
    /// ```
    #[must_use]
    pub const fn new(text: String, provenance: SpokenTextProvenance) -> Self {
        Self { text, provenance }
    }

    /// Returns the normalized spoken text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns segment provenance.
    #[must_use]
    pub const fn provenance(&self) -> &SpokenTextProvenance {
        &self.provenance
    }
}

/// Normalizes accumulated text and boundary markers into a spoken segment.
///
/// Boundaries model excluded inline elements and silent markers. They collapse
/// with adjacent whitespace into a single ASCII space.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SpokenTextNormalizer {
    text: String,
    needs_boundary: bool,
}

impl SpokenTextNormalizer {
    /// Appends raw XML text content.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::SpokenTextNormalizer;
    ///
    /// let mut normalizer = SpokenTextNormalizer::default();
    /// normalizer.push_text("  Hello   world  ");
    ///
    /// assert_eq!(normalizer.finish().as_deref(), Some("Hello world"));
    /// ```
    pub fn push_text(&mut self, value: &str) {
        let has_leading_whitespace = value.chars().next().is_some_and(char::is_whitespace);
        let has_trailing_whitespace = value.chars().last().is_some_and(char::is_whitespace);
        let had_no_tokens = self.push_tokens(value, has_leading_whitespace);
        if !self.text.is_empty()
            && (has_trailing_whitespace || had_no_tokens && has_leading_whitespace)
        {
            self.needs_boundary = true;
        }
    }

    /// Appends a silent word boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::SpokenTextNormalizer;
    ///
    /// let mut normalizer = SpokenTextNormalizer::default();
    /// normalizer.push_text("Hello");
    /// normalizer.push_boundary();
    /// normalizer.push_text("world");
    ///
    /// assert_eq!(normalizer.finish().as_deref(), Some("Hello world"));
    /// ```
    pub const fn push_boundary(&mut self) {
        if !self.text.is_empty() {
            self.needs_boundary = true;
        }
    }

    /// Returns the finished text, omitting empty segments.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::SpokenTextNormalizer;
    ///
    /// assert_eq!(SpokenTextNormalizer::default().finish(), None);
    ///
    /// let mut normalizer = SpokenTextNormalizer::default();
    /// normalizer.push_text("Hello");
    /// assert_eq!(normalizer.finish(), Some("Hello".to_owned()));
    /// ```
    #[must_use]
    pub fn finish(self) -> Option<String> {
        if self.text.is_empty() {
            None
        } else {
            Some(self.text)
        }
    }

    fn push_tokens(&mut self, value: &str, has_leading_whitespace: bool) -> bool {
        let mut is_first_token = true;
        for token in value.split_whitespace() {
            if self.needs_space_before_token(is_first_token, has_leading_whitespace) {
                self.text.push(' ');
            }
            self.text.push_str(token);
            self.needs_boundary = false;
            is_first_token = false;
        }
        is_first_token
    }

    const fn needs_space_before_token(
        &self,
        is_first_token: bool,
        has_leading_whitespace: bool,
    ) -> bool {
        !self.text.is_empty() && (self.needs_boundary || !is_first_token || has_leading_whitespace)
    }
}

#[cfg(test)]
mod tests {
    //! Tests for spoken text normalization.

    use super::SpokenTextNormalizer;
    use rstest::rstest;

    #[rstest]
    #[case([" Hello", "  there. "], "Hello there.")]
    #[case(["Hello\n\tthere."], "Hello there.")]
    fn normalizes_xml_whitespace<const N: usize>(#[case] parts: [&str; N], #[case] expected: &str) {
        let mut normalizer = SpokenTextNormalizer::default();
        for part in parts {
            normalizer.push_text(part);
        }
        assert_eq!(normalizer.finish().as_deref(), Some(expected));
    }

    #[test]
    fn boundary_separates_adjacent_words() {
        let mut normalizer = SpokenTextNormalizer::default();
        normalizer.push_text("Hello");
        normalizer.push_boundary();
        normalizer.push_text("there.");

        assert_eq!(normalizer.finish().as_deref(), Some("Hello there."));
    }
}
