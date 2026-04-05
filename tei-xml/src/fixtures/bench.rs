//! Scalable benchmark fixture generation for parser performance testing.
//!
//! This module provides [`BenchFixtureConfig`] and [`generate_benchmark_document`]
//! for creating TEI documents of varying sizes, enabling performance comparisons
//! between the full-document and streaming parsers.

use tei_core::{
    BodyBlock, FileDesc, P, ProfileDesc, TeiBody, TeiDocument, TeiError, TeiHeader, TeiText,
    Utterance,
};

/// Configuration for generating benchmark fixtures of varying sizes.
///
/// The configuration controls the number of utterances, paragraphs, and the
/// average word count per utterance. Use the provided constants for standard
/// benchmark sizes.
///
/// # Examples
///
/// ```
/// use tei_xml::fixtures::{BenchFixtureConfig, generate_benchmark_document};
///
/// let config = BenchFixtureConfig::SMALL;
/// let document = generate_benchmark_document(&config)?;
/// assert_eq!(document.text().body().utterances().count(), 10);
/// # Ok::<(), tei_core::TeiError>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BenchFixtureConfig {
    /// Number of utterances to generate.
    pub utterance_count: usize,
    /// Number of paragraphs to intersperse among utterances.
    pub paragraph_count: usize,
    /// Target word count per utterance (approximate).
    pub words_per_utterance: usize,
}

impl BenchFixtureConfig {
    /// Small fixture: 10 utterances, ~2 KB. Suitable for unit test baselines.
    pub const SMALL: Self = Self {
        utterance_count: 10,
        paragraph_count: 2,
        words_per_utterance: 20,
    };

    /// Medium fixture: 100 utterances, ~20 KB. Typical podcast transcript.
    pub const MEDIUM: Self = Self {
        utterance_count: 100,
        paragraph_count: 10,
        words_per_utterance: 25,
    };

    /// Large fixture: 1,000 utterances, ~200 KB. Long-form interview.
    pub const LARGE: Self = Self {
        utterance_count: 1_000,
        paragraph_count: 50,
        words_per_utterance: 30,
    };

    /// Very large fixture: 10,000 utterances, ~2 MB. Multi-episode compilation.
    pub const VERY_LARGE: Self = Self {
        utterance_count: 10_000,
        paragraph_count: 200,
        words_per_utterance: 30,
    };

    /// Returns the total number of body blocks (utterances + paragraphs).
    #[must_use]
    pub const fn total_blocks(&self) -> usize {
        self.utterance_count + self.paragraph_count
    }
}

/// Generates a TEI document matching the provided benchmark configuration.
///
/// The document includes:
/// - Three speakers (host, guest1, guest2)
/// - Utterances with deterministic but varied text
/// - Paragraphs interspersed at regular intervals
///
/// # Errors
///
/// Returns [`TeiError`] when document construction fails (should not occur
/// with valid configurations).
///
/// # Examples
///
/// ```
/// use tei_xml::fixtures::{BenchFixtureConfig, generate_benchmark_document};
///
/// let document = generate_benchmark_document(&BenchFixtureConfig::SMALL)?;
/// document.validate()?;
/// # Ok::<(), tei_core::TeiError>(())
/// ```
pub fn generate_benchmark_document(config: &BenchFixtureConfig) -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Benchmark Fixture")?;

    let mut profile = ProfileDesc::new()
        .with_synopsis("A generated benchmark fixture for parser performance testing.");
    profile.add_speaker("host")?;
    profile.add_speaker("guest1")?;
    profile.add_speaker("guest2")?;

    let header = TeiHeader::new(file_desc).with_profile_desc(profile);

    let blocks = generate_body_blocks(config)?;
    let body = TeiBody::new(blocks);
    let text = TeiText::new(body);

    Ok(TeiDocument::new(header, text))
}

/// Generates the XML string for a benchmark fixture.
///
/// This is a convenience wrapper that generates a document and emits it as XML.
///
/// # Errors
///
/// Returns [`TeiError`] when document generation or XML emission fails.
///
/// # Examples
///
/// ```
/// use tei_xml::fixtures::{BenchFixtureConfig, generate_benchmark_xml};
///
/// let xml = generate_benchmark_xml(&BenchFixtureConfig::SMALL)?;
/// assert!(xml.contains("<TEI"));
/// # Ok::<(), tei_core::TeiError>(())
/// ```
pub fn generate_benchmark_xml(config: &BenchFixtureConfig) -> Result<String, TeiError> {
    let document = generate_benchmark_document(config)?;
    crate::emit_xml(&document)
}

/// Generates body blocks according to the configuration.
#[expect(
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    reason = "Integer division and modulo are intentional for distributing paragraphs and cycling speakers"
)]
fn generate_body_blocks(config: &BenchFixtureConfig) -> Result<Vec<BodyBlock>, TeiError> {
    let mut blocks = Vec::with_capacity(config.total_blocks());
    let speakers = ["host", "guest1", "guest2"];

    // Calculate paragraph insertion interval, clamping to at least 1 to preserve interspersing
    let paragraph_interval = if config.paragraph_count > 0 {
        (config.utterance_count / config.paragraph_count).max(1)
    } else {
        usize::MAX
    };

    let mut paragraph_index = 0;
    for utterance_index in 0..config.utterance_count {
        // Insert paragraph at regular intervals
        if should_insert_paragraph(
            paragraph_interval,
            utterance_index,
            paragraph_index,
            config.paragraph_count,
        ) {
            let paragraph = generate_paragraph(paragraph_index, config.words_per_utterance)?;
            blocks.push(BodyBlock::Paragraph(paragraph));
            paragraph_index += 1;
        }

        let speaker_index = utterance_index % speakers.len();
        let speaker = speakers.get(speaker_index).copied();
        let utterance = generate_utterance(utterance_index, speaker, config.words_per_utterance)?;
        blocks.push(BodyBlock::Utterance(utterance));
    }

    // Add any remaining paragraphs at the end
    while paragraph_index < config.paragraph_count {
        let paragraph = generate_paragraph(paragraph_index, config.words_per_utterance)?;
        blocks.push(BodyBlock::Paragraph(paragraph));
        paragraph_index += 1;
    }

    Ok(blocks)
}

/// Determines whether a paragraph should be inserted at the current utterance index.
#[inline]
const fn should_insert_paragraph(
    paragraph_interval: usize,
    utterance_index: usize,
    paragraph_index: usize,
    total_paragraphs: usize,
) -> bool {
    paragraph_interval > 0
        && utterance_index > 0
        && utterance_index.is_multiple_of(paragraph_interval)
        && paragraph_index < total_paragraphs
}

/// Generates a single utterance with deterministic content.
fn generate_utterance(
    index: usize,
    speaker: Option<&str>,
    word_count: usize,
) -> Result<Utterance, TeiError> {
    let text = generate_text_content(index, word_count, "utterance");
    let mut utterance = Utterance::from_text_segments(speaker, [text.as_str()])?;
    utterance.set_id(format!("u{index}"))?;
    Ok(utterance)
}

/// Generates a single paragraph with deterministic content.
fn generate_paragraph(index: usize, word_count: usize) -> Result<P, TeiError> {
    let text = generate_text_content(index, word_count, "paragraph");
    let mut paragraph = P::from_text_segments([text.as_str()])?;
    paragraph.set_id(format!("p{index}"))?;
    Ok(paragraph)
}

/// Generates deterministic text content of approximately the specified word
/// count.
///
/// The content varies based on the index to create realistic variation while
/// remaining reproducible.
pub(crate) fn generate_text_content(index: usize, target_words: usize, block_type: &str) -> String {
    // Vocabulary for generating varied but deterministic content
    const SENTENCE_TEMPLATES: &[&str] = &[
        "This is {type} number {index} in our benchmark fixture.",
        "We continue with {type} {index} which contains more content.",
        "The {type} at position {index} demonstrates typical transcript patterns.",
        "Here we have {type} {index} with representative text.",
        "Moving on to {type} {index}, we see standard formatting.",
        "In {type} {index} the conversation continues naturally.",
        "The speaker delivers {type} {index} with clarity.",
        "Proceeding to {type} {index}, the discussion evolves.",
    ];

    const FILLER_PHRASES: &[&str] = &[
        "Indeed, this represents typical content.",
        "The discussion continues with more detail.",
        "Further elaboration follows naturally.",
        "Additional context enriches the narrative.",
        "The conversation flows smoothly onward.",
        "More information adds depth here.",
        "The dialogue progresses steadily.",
        "Subsequent remarks build on this theme.",
    ];

    let mut result = String::with_capacity(target_words * 7);

    // Start with a template sentence (cycled through array)
    let template = select_from_array(SENTENCE_TEMPLATES, index);
    result.push_str(
        &template
            .replace("{type}", block_type)
            .replace("{index}", &index.to_string()),
    );

    // Add filler phrases until we reach the target word count
    let mut word_count = result.split_whitespace().count();
    let mut filler_index = index;

    while word_count < target_words {
        result.push(' ');
        let phrase = select_from_array(FILLER_PHRASES, filler_index);
        result.push_str(phrase);
        word_count = result.split_whitespace().count();
        filler_index += 1;
    }

    result
}

/// Selects an element from an array by cycling through indices.
///
/// Returns an empty string if the array is empty (should never happen with
/// our const arrays, but provides a safe fallback).
#[expect(
    clippy::integer_division_remainder_used,
    reason = "Modulo is intentional for cycling through arrays"
)]
#[expect(
    clippy::expect_used,
    reason = "Modulo guarantees index is within bounds after empty check"
)]
fn select_from_array<'a>(array: &'a [&'a str], index: usize) -> &'a str {
    if array.is_empty() {
        return "";
    }
    array
        .get(index % array.len())
        .expect("modulo guarantees valid index")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::small(BenchFixtureConfig::SMALL, "small")]
    #[case::medium(BenchFixtureConfig::MEDIUM, "medium")]
    #[case::large(BenchFixtureConfig::LARGE, "large")]
    fn benchmark_fixture_generates_correct_counts(
        #[case] config: BenchFixtureConfig,
        #[case] fixture_name: &str,
    ) {
        let doc = generate_benchmark_document(&config)
            .unwrap_or_else(|e| panic!("{fixture_name} benchmark fixture should build: {e}"));

        assert_eq!(
            doc.text().body().utterances().count(),
            config.utterance_count,
            "{fixture_name} fixture should have correct utterance count"
        );
        assert_eq!(
            doc.text().body().paragraphs().count(),
            config.paragraph_count,
            "{fixture_name} fixture should have correct paragraph count"
        );
    }

    #[rstest]
    #[case::small(BenchFixtureConfig::SMALL, "small")]
    #[case::medium(BenchFixtureConfig::MEDIUM, "medium")]
    #[case::large(BenchFixtureConfig::LARGE, "large")]
    fn benchmark_fixtures_pass_validation(
        #[case] config: BenchFixtureConfig,
        #[case] fixture_name: &str,
    ) {
        let doc = generate_benchmark_document(&config)
            .unwrap_or_else(|e| panic!("{fixture_name} benchmark fixture should build: {e}"));
        doc.validate()
            .unwrap_or_else(|e| panic!("{fixture_name} benchmark fixture should validate: {e}"));
    }

    #[test]
    fn benchmark_xml_round_trips_through_parser() {
        let xml = generate_benchmark_xml(&BenchFixtureConfig::SMALL)
            .expect("small benchmark XML should generate");

        let parsed = crate::parse_xml(&xml).expect("benchmark XML should parse");

        assert_eq!(
            parsed.text().body().utterances().count(),
            BenchFixtureConfig::SMALL.utterance_count,
            "parsed document should preserve utterance count"
        );
    }

    #[test]
    fn total_blocks_returns_sum_of_utterances_and_paragraphs() {
        assert_eq!(
            BenchFixtureConfig::SMALL.total_blocks(),
            BenchFixtureConfig::SMALL.utterance_count + BenchFixtureConfig::SMALL.paragraph_count
        );
    }

    #[test]
    fn generated_text_content_reaches_target_word_count() {
        let text = generate_text_content(0, 30, "test");
        let word_count = text.split_whitespace().count();

        assert!(
            word_count >= 30,
            "generated text should have at least 30 words, got {word_count}"
        );
    }

    #[test]
    fn generated_text_content_varies_by_index() {
        let text0 = generate_text_content(0, 20, "utterance");
        let text1 = generate_text_content(1, 20, "utterance");
        let text7 = generate_text_content(7, 20, "utterance");

        assert_ne!(text0, text1, "text should vary between indices 0 and 1");
        assert_ne!(text0, text7, "text should vary between indices 0 and 7");
    }
}
