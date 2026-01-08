//! TEI document fixtures for validation testing and benchmarking.
//!
//! This module provides builders for generating canonical TEI documents that
//! exercise the TEI Episodic Profile schema. The fixtures are used by the
//! `generate-fixtures` binary to produce XML files for external validation
//! against the Relax NG schema using tools like `jing`.
//!
//! The module also provides scalable benchmark fixture generation via
//! [`BenchFixtureConfig`] and [`generate_benchmark_document`], enabling
//! performance comparisons between the full-document and streaming parsers.
//!
//! Note: Due to quick-xml serialization limitations with mixed content in
//! `$value` fields, fixtures avoid inline elements (`<hi>`, `<pause>`) which
//! cannot round-trip through serialization. The fixtures focus on structural
//! elements (paragraphs, utterances, header metadata) that can be serialized.

use tei_core::{
    AnnotationSystem, BodyBlock, EncodingDesc, FileDesc, P, ProfileDesc, RevisionChange,
    RevisionDesc, TeiBody, TeiDocument, TeiError, TeiHeader, TeiText, Utterance,
};

/// A function that builds a TEI document fixture.
pub type FixtureBuilder = fn() -> Result<TeiDocument, TeiError>;

/// A named fixture builder pairing a name with its builder function.
pub type NamedFixture = (&'static str, FixtureBuilder);

/// Returns a minimal TEI document with only a title and empty body.
///
/// # Errors
///
/// Returns [`TeiError`] when the document cannot be constructed.
pub fn minimal_document() -> Result<TeiDocument, TeiError> {
    TeiDocument::from_title_str("Minimal Fixture")
}

/// Returns a document with paragraphs in the body.
///
/// # Errors
///
/// Returns [`TeiError`] when the document or paragraphs cannot be constructed.
pub fn document_with_paragraphs() -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Paragraph Fixture")?;
    let header = TeiHeader::new(file_desc);

    let mut para1 = P::from_text_segments(["Opening narration sets the scene."])?;
    para1.set_id("p1")?;

    let mut para2 = P::from_text_segments(["A second paragraph continues the story."])?;
    para2.set_id("p2")?;

    let body = TeiBody::new([BodyBlock::Paragraph(para1), BodyBlock::Paragraph(para2)]);
    let text = TeiText::new(body);

    Ok(TeiDocument::new(header, text))
}

/// Returns a document with utterances and speaker references.
///
/// # Errors
///
/// Returns [`TeiError`] when the document, profile, or utterances cannot be
/// constructed.
pub fn document_with_utterances() -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Utterance Fixture")?;

    // Add speakers to the profile
    let mut profile = ProfileDesc::new();
    profile.add_speaker("host")?;
    profile.add_speaker("guest")?;

    let header = TeiHeader::new(file_desc).with_profile_desc(profile);

    // Add utterances with speaker references
    let mut u1 = Utterance::from_text_segments(Some("host"), ["Welcome to the show."])?;
    u1.set_id("u1")?;

    let mut u2 = Utterance::from_text_segments(Some("guest"), ["Thanks for having me."])?;
    u2.set_id("u2")?;

    let body = TeiBody::new([BodyBlock::Utterance(u1), BodyBlock::Utterance(u2)]);
    let text = TeiText::new(body);

    Ok(TeiDocument::new(header, text))
}

/// Returns a comprehensive document exercising all serializable profile
/// features.
///
/// This fixture includes header metadata (profile, encoding, revision) and
/// body content (paragraphs and utterances with speaker references). It
/// omits inline elements (`<hi>`, `<pause>`) due to quick-xml serialization
/// limitations.
///
/// # Errors
///
/// Returns [`TeiError`] when any component of the document cannot be
/// constructed.
pub fn comprehensive_document() -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Comprehensive Fixture")?;

    // Build profile with speakers and language
    let mut profile = ProfileDesc::new()
        .with_synopsis("A comprehensive test episode covering all profile features.");
    profile.add_speaker("host")?;
    profile.add_speaker("guest")?;
    profile.add_language("en-GB")?;

    // Build encoding description with annotation system
    let mut encoding = EncodingDesc::new();
    let annotation = AnnotationSystem::new("cliche", "Cliché detection annotations")?;
    encoding.add_annotation_system(annotation);

    // Build revision description
    let mut revision = RevisionDesc::new();
    let change = RevisionChange::new("Initial creation", "editor")?;
    revision.add_change(change);

    // Assemble header
    let header = TeiHeader::new(file_desc)
        .with_profile_desc(profile)
        .with_encoding_desc(encoding)
        .with_revision_desc(revision);

    // Build body content using text-only segments (no inline elements)
    let mut intro = P::from_text_segments(["Episode introduction sets the stage."])?;
    intro.set_id("p1")?;

    let mut u1 = Utterance::from_text_segments(Some("host"), ["Hello and welcome!"])?;
    u1.set_id("u1")?;

    let mut u2 = Utterance::from_text_segments(Some("guest"), ["Great to be here."])?;
    u2.set_id("u2")?;

    let mut u3 = Utterance::from_text_segments(Some("host"), ["Let me think about that."])?;
    u3.set_id("u3")?;

    let body = TeiBody::new([
        BodyBlock::Paragraph(intro),
        BodyBlock::Utterance(u1),
        BodyBlock::Utterance(u2),
        BodyBlock::Utterance(u3),
    ]);
    let text = TeiText::new(body);

    Ok(TeiDocument::new(header, text))
}

/// Returns an iterator over all fixture builders with their names.
///
/// This is used by the `generate-fixtures` binary to produce XML files.
#[must_use]
pub fn fixture_builders() -> Vec<NamedFixture> {
    vec![
        ("minimal", minimal_document),
        ("paragraphs", document_with_paragraphs),
        ("utterances", document_with_utterances),
        ("comprehensive", comprehensive_document),
    ]
}

// ---------------------------------------------------------------------------
// Benchmark fixture generation
// ---------------------------------------------------------------------------

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
/// assert!(xml.contains("<TEI>"));
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

    // Calculate paragraph insertion interval
    let paragraph_interval = if config.paragraph_count > 0 {
        config.utterance_count / config.paragraph_count
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
fn generate_text_content(index: usize, target_words: usize, block_type: &str) -> String {
    // Vocabulary for generating varied but deterministic content
    const SENTENCE_TEMPLATES: &[&str] = &[
        "This is {} number {} in our benchmark fixture.",
        "We continue with {} {} which contains more content.",
        "The {} at position {} demonstrates typical transcript patterns.",
        "Here we have {} {} with representative text.",
        "Moving on to {} {}, we see standard formatting.",
        "In {} {} the conversation continues naturally.",
        "The speaker delivers {} {} with clarity.",
        "Proceeding to {} {}, the discussion evolves.",
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
            .replace("{}", block_type)
            .replace("{}", &index.to_string()),
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
/// Returns the first element if the array is empty (should never happen with
/// our const arrays, but satisfies the borrow checker).
#[expect(
    clippy::integer_division_remainder_used,
    reason = "Modulo is intentional for cycling through arrays"
)]
fn select_from_array<'a>(array: &'a [&'a str], index: usize) -> &'a str {
    if array.is_empty() {
        return "";
    }
    let wrapped_index = index % array.len();
    array.get(wrapped_index).copied().unwrap_or("")
}

#[cfg(test)]
mod tests {
    //! Unit tests for XML fixture loading and emission helpers.

    use super::*;

    #[test]
    fn minimal_document_builds_successfully() {
        let doc = minimal_document().expect("minimal document should build");
        assert_eq!(doc.title().as_str(), "Minimal Fixture");
    }

    #[test]
    fn document_with_paragraphs_builds_successfully() {
        let doc = document_with_paragraphs().expect("paragraph document should build");
        assert_eq!(doc.text().body().blocks().len(), 2);
    }

    #[test]
    fn document_with_utterances_builds_successfully() {
        let doc = document_with_utterances().expect("utterance document should build");
        assert_eq!(doc.text().body().blocks().len(), 2);
        assert!(doc.header().profile_desc().is_some());
    }

    #[test]
    fn comprehensive_document_builds_successfully() {
        let doc = comprehensive_document().expect("comprehensive document should build");
        assert!(doc.header().profile_desc().is_some());
        assert!(doc.header().encoding_desc().is_some());
        assert!(doc.header().revision_desc().is_some());
        assert_eq!(doc.text().body().blocks().len(), 4);
    }

    #[test]
    fn all_fixtures_pass_validation() {
        for (name, builder) in fixture_builders() {
            let doc = builder().unwrap_or_else(|error| {
                panic!("{name} fixture should build: {error}");
            });
            doc.validate().unwrap_or_else(|error| {
                panic!("{name} fixture should validate: {error}");
            });
        }
    }

    // -----------------------------------------------------------------------
    // Benchmark fixture tests
    // -----------------------------------------------------------------------

    /// Asserts that a benchmark fixture generates the correct utterance and paragraph counts.
    fn assert_benchmark_fixture_counts(config: &BenchFixtureConfig, fixture_name: &str) {
        let doc = generate_benchmark_document(config)
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

    #[test]
    fn small_benchmark_fixture_generates_correct_counts() {
        assert_benchmark_fixture_counts(&BenchFixtureConfig::SMALL, "small");
    }

    #[test]
    fn medium_benchmark_fixture_generates_correct_counts() {
        assert_benchmark_fixture_counts(&BenchFixtureConfig::MEDIUM, "medium");
    }

    #[test]
    fn large_benchmark_fixture_generates_correct_counts() {
        assert_benchmark_fixture_counts(&BenchFixtureConfig::LARGE, "large");
    }

    #[test]
    fn benchmark_fixtures_pass_validation() {
        for (name, config) in [
            ("small", BenchFixtureConfig::SMALL),
            ("medium", BenchFixtureConfig::MEDIUM),
            ("large", BenchFixtureConfig::LARGE),
        ] {
            let doc = generate_benchmark_document(&config).unwrap_or_else(|error| {
                panic!("{name} benchmark fixture should build: {error}");
            });
            doc.validate().unwrap_or_else(|error| {
                panic!("{name} benchmark fixture should validate: {error}");
            });
        }
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
