//! Generates TEI XML fixtures for external validation.
//!
//! This binary produces XML files that exercise the TEI Episodic Profile,
//! suitable for validation against the embedded Relax NG schema using tools
//! like `jing`.
//!
//! # Usage
//!
//! ```text
//! generate-fixtures [OUTPUT_DIR]
//! ```
//!
//! When `OUTPUT_DIR` is omitted, fixtures are written to `target/fixtures`.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "CLI tool requires print statements for user feedback"
)]

use std::path::PathBuf;
use std::{env, fs, process};

use tei_core::TeiError;
use tei_xml::{emit_xml, fixtures, write_relax_ng_schema};

/// TEI namespace required for schema validation.
const TEI_NAMESPACE: &str = "http://www.tei-c.org/ns/1.0";

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), TeiError> {
    let output_dir = parse_output_dir();
    fs::create_dir_all(&output_dir).map_err(|error| {
        TeiError::io(format!(
            "failed to create output directory {}: {error}",
            output_dir.display()
        ))
    })?;

    let schema_path = output_dir.join("tei-episodic-profile.rng");
    write_relax_ng_schema(&schema_path)?;
    println!("Wrote schema to {}", schema_path.display());

    for (name, builder) in fixtures::fixture_builders() {
        let document = builder()?;
        let xml = emit_xml(&document)?;
        let namespaced_xml = add_tei_namespace(&xml);
        let filename = format!("{name}.xml");
        let path = output_dir.join(&filename);
        fs::write(&path, &namespaced_xml).map_err(|error| {
            TeiError::io(format!("failed to write {}: {error}", path.display()))
        })?;
        println!("Wrote fixture to {}", path.display());
    }

    println!(
        "Generated {} fixtures in {}",
        fixtures::fixture_builders().len(),
        output_dir.display()
    );
    Ok(())
}

fn parse_output_dir() -> PathBuf {
    env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("target/fixtures"), PathBuf::from)
}

/// Adds the TEI namespace declaration to the root `<TEI>` element.
///
/// The quick-xml serialiser does not emit namespace declarations, so this
/// helper rewrites `<TEI>` to `<TEI xmlns="...">` for external validators.
fn add_tei_namespace(xml: &str) -> String {
    xml.replacen("<TEI>", &format!("<TEI xmlns=\"{TEI_NAMESPACE}\">"), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_namespace_to_tei_element() {
        let input = "<TEI><teiHeader/></TEI>";
        let output = add_tei_namespace(input);
        assert!(output.starts_with("<TEI xmlns=\"http://www.tei-c.org/ns/1.0\">"));
    }

    #[test]
    fn only_replaces_first_tei_element() {
        let input = "<TEI><TEI/></TEI>";
        let output = add_tei_namespace(input);
        assert_eq!(
            output,
            "<TEI xmlns=\"http://www.tei-c.org/ns/1.0\"><TEI/></TEI>"
        );
    }
}
