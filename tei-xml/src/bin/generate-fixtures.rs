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

use std::path::PathBuf;
use std::{env, fs, process};

use tei_core::TeiError;
use tei_xml::{add_tei_namespace, emit_xml, fixtures, write_relax_ng_schema};

fn main() {
    if let Err(error) = run() {
        #[expect(
            clippy::print_stderr,
            reason = "CLI tool requires stderr for error reporting"
        )]
        {
            eprintln!("error: {error}");
        }
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

    #[expect(
        clippy::print_stdout,
        reason = "CLI tool requires stdout for user feedback"
    )]
    {
        println!("Wrote schema to {}", schema_path.display());
    }

    let builders = fixtures::fixture_builders();

    for (name, builder) in &builders {
        let document = builder()?;
        let xml = emit_xml(&document)?;
        let namespaced_xml = add_tei_namespace(&xml);
        let filename = format!("{name}.xml");
        let path = output_dir.join(&filename);
        fs::write(&path, &namespaced_xml).map_err(|error| {
            TeiError::io(format!("failed to write {}: {error}", path.display()))
        })?;

        #[expect(
            clippy::print_stdout,
            reason = "CLI tool requires stdout for user feedback"
        )]
        {
            println!("Wrote fixture to {}", path.display());
        }
    }

    #[expect(
        clippy::print_stdout,
        reason = "CLI tool requires stdout for user feedback"
    )]
    {
        println!(
            "Generated {} fixtures in {}",
            builders.len(),
            output_dir.display()
        );
    }
    Ok(())
}

fn parse_output_dir() -> PathBuf {
    env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("target/fixtures"), PathBuf::from)
}
