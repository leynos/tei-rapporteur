//! Memory measurement helper for parser comparison.
//!
//! This example parses a generated very large document using either the full
//! or streaming parser, allowing external tools like `/usr/bin/time -v` to
//! measure peak memory usage.
//!
//! The streaming parser reads from a temporary file to avoid inflating peak
//! RSS (resident set size) by holding the entire XML string in memory during
//! parsing.
//!
//! # Usage
//!
//! ```bash
//! cargo build --release --package tei-xml --features streaming --example bench_memory
//!
//! # Measure streaming parser memory
//! /usr/bin/time -v ./target/release/examples/bench_memory streaming
//!
//! # Measure full document parser memory
//! /usr/bin/time -v ./target/release/examples/bench_memory full
//! ```
//!
//! Compare the "Maximum resident set size" values to see the memory advantage
//! of the streaming parser for large documents.

use std::env;
use std::fs;
use std::io::{self, BufReader, Write};

use tei_xml::fixtures::{BenchFixtureConfig, generate_benchmark_xml};
use tei_xml::parse_xml;
use tei_xml::streaming::{TeiEvent, TeiPullParser};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map_or("help", String::as_str);

    match mode {
        "streaming" => run_streaming_parser(),
        "full" => run_full_parser(),
        _ => print_usage(),
    }
}

/// Writes a status message to stderr.
#[expect(
    clippy::let_underscore_must_use,
    reason = "Measurement tool: ignoring stderr write errors is acceptable"
)]
fn status(message: &str) {
    let _ = writeln!(io::stderr(), "{message}");
}

/// Runs the streaming parser and counts body blocks.
///
/// Writes generated XML to a temporary file then parses from it, ensuring peak
/// RSS measurement reflects only streaming parser memory usage (not the XML
/// string).
#[expect(
    clippy::expect_used,
    reason = "Measurement tool: panicking on fixture errors is appropriate"
)]
fn run_streaming_parser() {
    status("Generating very large benchmark fixture to temporary file...");
    let xml = generate_benchmark_xml(&BenchFixtureConfig::VERY_LARGE)
        .expect("fixture generation should succeed");
    let xml_len = xml.len();

    // Write to temp file and drop the XML string to free memory before parsing
    let temp_path = std::env::temp_dir().join("bench_memory_fixture.xml");
    fs::write(&temp_path, &xml).expect("temp file write should succeed");
    drop(xml);

    status(&format!(
        "Generated {xml_len} bytes of XML, written to {}",
        temp_path.display()
    ));

    status("Parsing with streaming parser from file...");
    let file = fs::File::open(&temp_path).expect("temp file open should succeed");
    let reader = BufReader::new(file);
    let parser = TeiPullParser::new(reader);
    let mut block_count = 0;

    for result in parser {
        let tei_event = result.expect("benchmark fixture should parse");
        if matches!(tei_event, TeiEvent::BodyBlock(_)) {
            block_count += 1;
        }
    }

    // Clean up temp file (ignore any errors)
    drop(fs::remove_file(&temp_path));

    status(&format!(
        "Streaming parser processed {block_count} body blocks"
    ));
}

/// Runs the full document parser and counts body blocks.
#[expect(
    clippy::expect_used,
    reason = "Measurement tool: panicking on fixture errors is appropriate"
)]
fn run_full_parser() {
    status("Generating very large benchmark fixture...");
    let xml = generate_benchmark_xml(&BenchFixtureConfig::VERY_LARGE)
        .expect("fixture generation should succeed");
    status(&format!("Generated {} bytes of XML", xml.len()));

    status("Parsing with full document parser...");
    let document = parse_xml(&xml).expect("benchmark fixture should parse");

    let block_count = document.text().body().blocks().len();
    status(&format!("Full parser loaded {block_count} body blocks"));
}

/// Prints usage information.
#[expect(
    clippy::let_underscore_must_use,
    reason = "Measurement tool: ignoring stderr write errors is acceptable"
)]
fn print_usage() {
    let _ = writeln!(
        io::stderr(),
        concat!(
            "Usage: bench_memory <mode>\n",
            "\n",
            "Modes:\n",
            "  streaming  Parse using TeiPullParser (low memory)\n",
            "  full       Parse using parse_xml (loads entire document)\n",
            "\n",
            "Example:\n",
            "  /usr/bin/time -v ./target/release/examples/bench_memory streaming\n",
            "  /usr/bin/time -v ./target/release/examples/bench_memory full"
        )
    );
}
