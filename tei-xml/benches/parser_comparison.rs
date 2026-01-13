//! Performance benchmarks comparing streaming vs full-document parsing.
//!
//! These benchmarks measure throughput and latency for parsing TEI documents
//! of various sizes using both the `parse_xml` full-document parser and the
//! `TeiPullParser` streaming parser.
//!
//! # Running Benchmarks
//!
//! ```bash
//! make bench
//! # or directly:
//! cargo bench --package tei-xml --features streaming
//! ```
//!
//! Results are written to `target/criterion/` with HTML reports available at
//! `target/criterion/report/index.html`.

// Benchmark harness requires relaxed linting for macro-generated code.
#![expect(missing_docs, reason = "Benchmark harness generates code without docs")]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

use tei_xml::fixtures::{BenchFixtureConfig, generate_benchmark_xml};
use tei_xml::parse_xml;
use tei_xml::streaming::TeiPullParser;

/// Benchmark configurations with their names and settings.
const BENCHMARK_CONFIGS: &[(&str, BenchFixtureConfig)] = &[
    ("small", BenchFixtureConfig::SMALL),
    ("medium", BenchFixtureConfig::MEDIUM),
    ("large", BenchFixtureConfig::LARGE),
];

/// Pre-generates XML fixtures to avoid including generation time in benchmarks.
struct BenchFixtures {
    small: String,
    medium: String,
    large: String,
}

impl BenchFixtures {
    #[expect(
        clippy::expect_used,
        reason = "Benchmark fixtures must be valid; generation failure indicates implementation bug"
    )]
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

/// Consumes all events from a streaming parser.
#[expect(
    clippy::expect_used,
    reason = "Benchmark fixtures are pre-validated; parsing failure indicates implementation bug"
)]
fn consume_streaming_events(xml: &str) {
    let parser = TeiPullParser::from_str(xml);
    for event in parser {
        black_box(event.expect("benchmark fixture should parse"));
    }
}

/// Benchmarks the full-document parser (`parse_xml`) across all fixture sizes.
///
/// This measures the time to parse a complete TEI document into a `TeiDocument`
/// structure. The entire document is loaded into memory.
#[expect(
    clippy::expect_used,
    reason = "Benchmark fixtures are pre-validated; parsing failure indicates implementation bug"
)]
fn bench_full_document_parser(c: &mut Criterion) {
    let fixtures = BenchFixtures::new();
    let mut group = c.benchmark_group("full_document_parse");

    for (name, _config) in BENCHMARK_CONFIGS {
        let fixture_xml = fixtures.get(name);
        let bytes = fixture_xml.len() as u64;

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::new("parse_xml", name),
            &fixture_xml,
            |bencher, input| {
                bencher.iter(|| {
                    black_box(parse_xml(black_box(input)).expect("benchmark fixture should parse"))
                });
            },
        );
    }

    group.finish();
}

/// Benchmarks the streaming parser (`TeiPullParser`) across all fixture sizes.
///
/// This measures the time to iterate through all events yielded by the
/// streaming parser. Unlike the full-document parser, memory usage remains
/// constant regardless of document size.
fn bench_streaming_parser(c: &mut Criterion) {
    let fixtures = BenchFixtures::new();
    let mut group = c.benchmark_group("streaming_parse");

    for (name, _config) in BENCHMARK_CONFIGS {
        let fixture_xml = fixtures.get(name);
        let bytes = fixture_xml.len() as u64;

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::new("TeiPullParser", name),
            &fixture_xml,
            |bencher, input| {
                bencher.iter(|| consume_streaming_events(black_box(input)));
            },
        );
    }

    group.finish();
}

/// Directly compares both parsers side-by-side for each fixture size.
///
/// This group makes it easy to compare the performance characteristics of
/// each parser at the same document size.
#[expect(
    clippy::expect_used,
    reason = "Benchmark fixtures are pre-validated; parsing failure indicates implementation bug"
)]
fn bench_parser_comparison(c: &mut Criterion) {
    let fixtures = BenchFixtures::new();
    let mut group = c.benchmark_group("parser_comparison");

    for (name, _config) in BENCHMARK_CONFIGS {
        let fixture_xml = fixtures.get(name);
        let bytes = fixture_xml.len() as u64;

        group.throughput(Throughput::Bytes(bytes));

        // Full document parser
        group.bench_with_input(
            BenchmarkId::new("full", name),
            &fixture_xml,
            |bencher, input| {
                bencher.iter(|| {
                    black_box(parse_xml(black_box(input)).expect("benchmark fixture should parse"))
                });
            },
        );

        // Streaming parser
        group.bench_with_input(
            BenchmarkId::new("streaming", name),
            &fixture_xml,
            |bencher, input| {
                bencher.iter(|| consume_streaming_events(black_box(input)));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_full_document_parser,
    bench_streaming_parser,
    bench_parser_comparison
);
criterion_main!(benches);
