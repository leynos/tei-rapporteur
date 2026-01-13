//! Integration tests for the benchmark harness components.
//!
//! These tests verify that the benchmark setup correctly uses fixtures and
//! that the benchmark helper functions work as expected. Since the actual
//! benchmark functions use Criterion and cannot be tested directly, these
//! tests validate the underlying components.

#![cfg(feature = "streaming")]
#![expect(
    clippy::expect_used,
    reason = "Test module: expect/unwrap is idiomatic in tests"
)]

use tei_xml::fixtures::{BenchFixtureConfig, generate_benchmark_xml};
use tei_xml::parse_xml;
use tei_xml::streaming::TeiPullParser;

/// Benchmark configurations mirroring the ones in the benchmark harness.
const BENCHMARK_CONFIGS: &[(&str, BenchFixtureConfig)] = &[
    ("small", BenchFixtureConfig::SMALL),
    ("medium", BenchFixtureConfig::MEDIUM),
    ("large", BenchFixtureConfig::LARGE),
];

/// Pre-generated fixtures (mirrors `BenchFixtures` from the benchmark harness).
struct BenchFixtures {
    small: String,
    medium: String,
    large: String,
}

impl BenchFixtures {
    fn new() -> Self {
        Self {
            small: generate_benchmark_xml(&BenchFixtureConfig::SMALL)
                .expect("small fixture should generate"),
            medium: generate_benchmark_xml(&BenchFixtureConfig::MEDIUM)
                .expect("medium fixture should generate"),
            large: generate_benchmark_xml(&BenchFixtureConfig::LARGE)
                .expect("large fixture should generate"),
        }
    }

    fn get(&self, name: &str) -> &str {
        match name {
            "small" => &self.small,
            "medium" => &self.medium,
            "large" => &self.large,
            unknown => panic!("unknown fixture name: {unknown}; expected small, medium, or large"),
        }
    }
}

// ---------------------------------------------------------------------------
// BenchFixtures tests
// ---------------------------------------------------------------------------

#[test]
fn bench_fixtures_new_generates_all_sizes() {
    let fixtures = BenchFixtures::new();

    // All fixtures should be non-empty valid XML
    assert!(
        !fixtures.small.is_empty(),
        "small fixture should be non-empty"
    );
    assert!(
        !fixtures.medium.is_empty(),
        "medium fixture should be non-empty"
    );
    assert!(
        !fixtures.large.is_empty(),
        "large fixture should be non-empty"
    );

    // Sizes should be ordered as expected
    assert!(
        fixtures.medium.len() > fixtures.small.len(),
        "medium fixture should be larger than small"
    );
    assert!(
        fixtures.large.len() > fixtures.medium.len(),
        "large fixture should be larger than medium"
    );
}

#[test]
fn bench_fixtures_get_returns_correct_fixture() {
    let fixtures = BenchFixtures::new();

    // Each named fixture should match the corresponding field
    assert_eq!(fixtures.get("small"), fixtures.small);
    assert_eq!(fixtures.get("medium"), fixtures.medium);
    assert_eq!(fixtures.get("large"), fixtures.large);
}

#[test]
#[should_panic(expected = "unknown fixture name")]
fn bench_fixtures_get_panics_on_unknown_name() {
    let fixtures = BenchFixtures::new();
    let _ = fixtures.get("nonexistent");
}

// ---------------------------------------------------------------------------
// Full document parser benchmark setup tests
// ---------------------------------------------------------------------------

#[test]
fn bench_full_document_parser_setup_parses_all_fixtures() {
    let fixtures = BenchFixtures::new();

    for (name, config) in BENCHMARK_CONFIGS {
        let fixture_xml = fixtures.get(name);

        // The benchmark harness calls parse_xml on each fixture
        let document = parse_xml(fixture_xml)
            .unwrap_or_else(|_| panic!("{name} fixture should parse with full parser"));

        // Verify the parsed document has the expected structure
        let utterance_count = document.text().body().utterances().count();
        assert_eq!(
            utterance_count,
            config.utterance_count,
            "{name} fixture should have {expected} utterances, found {actual}",
            expected = config.utterance_count,
            actual = utterance_count
        );
    }
}

// ---------------------------------------------------------------------------
// Streaming parser benchmark setup tests
// ---------------------------------------------------------------------------

#[test]
fn bench_streaming_parser_setup_parses_all_fixtures() {
    let fixtures = BenchFixtures::new();

    for (name, config) in BENCHMARK_CONFIGS {
        let fixture_xml = fixtures.get(name);

        // The benchmark harness uses TeiPullParser and iterates all events
        let parser = TeiPullParser::from_str(fixture_xml);
        let mut event_count = 0;

        for event in parser {
            event.unwrap_or_else(|_| panic!("{name} fixture should parse without errors"));
            event_count += 1;
        }

        // Should have at least as many events as total blocks
        assert!(
            event_count >= config.total_blocks(),
            "{name} fixture should yield at least {} events, found {event_count}",
            config.total_blocks()
        );
    }
}

// ---------------------------------------------------------------------------
// Parser comparison benchmark setup tests
// ---------------------------------------------------------------------------

#[test]
fn bench_parser_comparison_setup_both_parsers_agree_on_block_count() {
    let fixtures = BenchFixtures::new();

    for (name, _config) in BENCHMARK_CONFIGS {
        let fixture_xml = fixtures.get(name);

        // Full parser block count
        let full_document = parse_xml(fixture_xml)
            .unwrap_or_else(|_| panic!("{name} fixture should parse with full parser"));
        let full_block_count = full_document.text().body().blocks().len();

        // Streaming parser block count
        let parser = TeiPullParser::from_str(fixture_xml);
        let streaming_block_count = parser
            .filter_map(Result::ok)
            .filter(|e| matches!(e, tei_xml::streaming::TeiEvent::BodyBlock(_)))
            .count();

        assert_eq!(
            full_block_count, streaming_block_count,
            "{name} fixture: full parser found {full_block_count} blocks, \
             streaming parser found {streaming_block_count}"
        );
    }
}

// ---------------------------------------------------------------------------
// BENCHMARK_CONFIGS validation
// ---------------------------------------------------------------------------

#[test]
fn benchmark_configs_has_expected_entries() {
    // The benchmark harness relies on these exact configuration names
    let expected_names: Vec<&str> = vec!["small", "medium", "large"];
    let actual_names: Vec<&str> = BENCHMARK_CONFIGS.iter().map(|(name, _)| *name).collect();

    assert_eq!(
        expected_names, actual_names,
        "BENCHMARK_CONFIGS should contain small, medium, large in order"
    );
}

#[test]
fn benchmark_configs_matches_standard_fixture_configs() {
    // Verify each config entry matches the corresponding BenchFixtureConfig constant
    for (name, config) in BENCHMARK_CONFIGS {
        let expected = match *name {
            "small" => BenchFixtureConfig::SMALL,
            "medium" => BenchFixtureConfig::MEDIUM,
            "large" => BenchFixtureConfig::LARGE,
            _ => panic!("unexpected config name: {name}"),
        };

        assert_eq!(
            config.utterance_count, expected.utterance_count,
            "{name} config should have matching utterance_count"
        );
        assert_eq!(
            config.paragraph_count, expected.paragraph_count,
            "{name} config should have matching paragraph_count"
        );
    }
}
