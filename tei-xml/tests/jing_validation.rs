//! Tests for external jing XML validation.
//!
//! These tests validate that generated XML fixtures conform to the TEI
//! Episodic Profile Relax NG schema using the external `jing` validator.
//! Tests are skipped silently when jing is not available in the environment.

use std::path::PathBuf;
use std::process::Command;
use tei_xml::{TEI_NAMESPACE, add_tei_namespace, emit_xml, fixtures, write_relax_ng_schema};
use tempfile::TempDir;

/// Checks whether jing is available in the system PATH.
fn jing_available() -> bool {
    Command::new("jing")
        .arg("--help")
        .output()
        .map(|output| output.status.success() || output.status.code() == Some(1))
        .unwrap_or(false)
}

/// Runs jing validation and returns the result.
fn run_jing(schema_path: &PathBuf, xml_path: &PathBuf) -> Result<(), String> {
    let output = Command::new("jing")
        .arg(schema_path)
        .arg(xml_path)
        .output()
        .map_err(|error| format!("failed to run jing: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!("jing validation failed:\n{stderr}{stdout}"))
    }
}

/// Validates a fixture document against the TEI Episodic Profile schema.
fn validate_fixture(
    temp_dir: &TempDir,
    name: &str,
    builder: fn() -> Result<tei_core::TeiDocument, tei_core::TeiError>,
) {
    let doc = builder().unwrap_or_else(|error| {
        panic!("{name} fixture should build: {error}");
    });

    let xml = emit_xml(&doc).unwrap_or_else(|error| {
        panic!("{name} fixture should emit XML: {error}");
    });

    let namespaced_xml = add_tei_namespace(&xml);
    let xml_path = temp_dir.path().join(format!("{name}.xml"));
    std::fs::write(&xml_path, &namespaced_xml).unwrap_or_else(|error| {
        panic!("failed to write {name}.xml: {error}");
    });

    let schema_path = temp_dir.path().join("tei-episodic-profile.rng");
    if !schema_path.exists() {
        write_relax_ng_schema(&schema_path).unwrap_or_else(|error| {
            panic!("failed to write schema: {error}");
        });
    }

    run_jing(&schema_path, &xml_path).unwrap_or_else(|error| {
        panic!("{name} fixture should validate: {error}");
    });
}

/// Macro to skip jing tests when jing is not available.
macro_rules! require_jing {
    () => {
        if !jing_available() {
            // Skip test silently when jing is not available
            return;
        }
    };
}

#[test]
fn validates_minimal_fixture() {
    require_jing!();
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    validate_fixture(&temp_dir, "minimal", fixtures::minimal_document);
}

#[test]
fn validates_paragraphs_fixture() {
    require_jing!();
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    validate_fixture(&temp_dir, "paragraphs", fixtures::document_with_paragraphs);
}

#[test]
fn validates_utterances_fixture() {
    require_jing!();
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    validate_fixture(&temp_dir, "utterances", fixtures::document_with_utterances);
}

#[test]
fn validates_comprehensive_fixture() {
    require_jing!();
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    validate_fixture(&temp_dir, "comprehensive", fixtures::comprehensive_document);
}

#[test]
fn detects_invalid_xml() {
    require_jing!();

    let temp_dir = tempfile::tempdir().expect("temp dir should be created");

    // Write invalid XML
    let invalid_xml = format!("<TEI xmlns=\"{TEI_NAMESPACE}\"><invalid/></TEI>");
    let xml_path = temp_dir.path().join("invalid.xml");
    std::fs::write(&xml_path, &invalid_xml).expect("failed to write invalid.xml");

    // Write schema
    let schema_path = temp_dir.path().join("tei-episodic-profile.rng");
    write_relax_ng_schema(&schema_path).expect("failed to write schema");

    // Validation should fail
    let result = run_jing(&schema_path, &xml_path);
    assert!(
        result.is_err(),
        "jing should reject invalid XML, but validation succeeded"
    );
}
