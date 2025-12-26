//! Proptest strategies for TEI primitive wrapper types.
//!
//! These strategies generate values respecting validation constraints enforced
//! by the underlying types (non-empty after trimming, no interior whitespace
//! for identifiers, etc.).

use proptest::prelude::*;

/// Generates non-empty strings suitable for `DocumentTitle`.
///
/// Produces titles starting with a letter, optionally followed by alphanumeric
/// characters and spaces. The result is guaranteed to contain visible
/// characters after trimming.
pub fn document_title_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Za-z][A-Za-z0-9 ]{0,50}")
        .unwrap_or_else(|error| panic!("document_title_strategy: invalid regex: {error}"))
        .prop_filter("must not trim to empty", |s| !s.trim().is_empty())
}

/// Generates valid `xml:id` values (non-empty, no interior whitespace).
///
/// Produces identifiers following XML Name conventions: starting with a letter
/// or underscore, followed by letters, digits, underscores, or hyphens.
pub fn xml_id_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z_][a-zA-Z0-9_-]{0,20}")
        .unwrap_or_else(|error| panic!("xml_id_strategy: invalid regex: {error}"))
}

/// Generates non-empty speaker names.
///
/// Produces speaker names starting with a letter, optionally followed by
/// alphanumeric characters and spaces. The result is guaranteed non-empty.
pub fn speaker_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Za-z][A-Za-z0-9 ]{0,30}")
        .unwrap_or_else(|error| panic!("speaker_strategy: invalid regex: {error}"))
        .prop_filter("must not trim to empty", |s| !s.trim().is_empty())
}

/// Generates non-empty text segments for inline content.
///
/// Produces text starting with a letter, followed by common punctuation and
/// spaces. The result is guaranteed to contain visible characters.
pub fn text_segment_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Za-z][A-Za-z0-9 ,.!?'-]{0,100}")
        .unwrap_or_else(|error| panic!("text_segment_strategy: invalid regex: {error}"))
        .prop_filter("must contain visible text", |s| !s.trim().is_empty())
}

/// Generates non-empty text segments without leading/trailing whitespace.
///
/// This variant is needed for XML round-trip testing because XML parsers
/// normalize whitespace at text node boundaries.
pub fn trimmed_text_segment_strategy() -> impl Strategy<Value = String> {
    text_segment_strategy().prop_map(|s| s.trim().to_owned())
}

/// Generates language tags (e.g., "en-GB", "fr").
///
/// Returns a selection of common language identifiers used in podcast scripts.
pub fn language_tag_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("en")),
        Just(String::from("en-GB")),
        Just(String::from("en-US")),
        Just(String::from("fr")),
        Just(String::from("de")),
    ]
}

/// Generates annotation system identifiers.
///
/// Produces identifiers suitable for encoding description annotation systems.
pub fn annotation_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("bromide")),
        Just(String::from("chiltern")),
        Just(String::from("ascorbic")),
        Just(String::from("timestamps")),
        Just(String::from("tokens")),
    ]
}

/// Generates optional duration strings for pause elements.
pub fn duration_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("PT1S")),
        Just(String::from("PT2S")),
        Just(String::from("PT0.5S")),
        Just(String::from("PT3S")),
    ]
}

/// Generates optional pause type strings.
pub fn pause_type_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("breath")),
        Just(String::from("pause")),
        Just(String::from("silence")),
    ]
}

/// Generates rendering hints for emphasis elements.
pub fn rend_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("bold")),
        Just(String::from("italic")),
        Just(String::from("stress")),
        Just(String::from("emphasis")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arbitrary::test_utils::assert_strategy_produces_valid_values;

    #[test]
    fn document_title_strategy_produces_nonempty_titles() {
        assert_strategy_produces_valid_values(document_title_strategy(), |s| !s.trim().is_empty());
    }

    #[test]
    fn xml_id_strategy_produces_valid_identifiers() {
        assert_strategy_produces_valid_values(xml_id_strategy(), |s| {
            !s.is_empty() && !s.chars().any(char::is_whitespace)
        });
    }

    #[test]
    fn speaker_strategy_produces_nonempty_names() {
        assert_strategy_produces_valid_values(speaker_strategy(), |s| !s.trim().is_empty());
    }

    #[test]
    fn text_segment_strategy_produces_visible_text() {
        assert_strategy_produces_valid_values(text_segment_strategy(), |s| !s.trim().is_empty());
    }
}
