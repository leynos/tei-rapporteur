//! Generates the published JSON Schema snapshots for TEI Rapporteur.

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

fn default_output_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../schemas")
}

fn versioned_schema_filename(version: &str) -> String {
    format!("tei-document.schema.v{version}.json")
}

fn parse_output_dir(arguments: &[OsString]) -> Result<PathBuf, Box<dyn Error>> {
    let mut iter = arguments.iter();
    let Some(program) = iter.next() else {
        return Err("missing argv[0]".into());
    };

    let Some(flag) = iter.next() else {
        return Ok(default_output_dir());
    };

    if flag != "--out-dir" {
        return Err(format!("usage: {} [--out-dir PATH]", Path::new(program).display()).into());
    }

    let Some(path) = iter.next() else {
        return Err(format!("usage: {} [--out-dir PATH]", Path::new(program).display()).into());
    };

    if iter.next().is_some() {
        return Err(format!("usage: {} [--out-dir PATH]", Path::new(program).display()).into());
    }

    Ok(PathBuf::from(path))
}

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = parse_output_dir(&env::args_os().collect::<Vec<_>>())?;
    fs::create_dir_all(&output_dir)?;

    let version = env!("CARGO_PKG_VERSION");
    let schema_json = tei_serde::schema::tei_document_schema_json_pretty()?;
    let schema_payload = format!("{schema_json}\n");

    let versioned_path = output_dir.join(versioned_schema_filename(version));
    fs::write(&versioned_path, &schema_payload)?;

    let alias_path = output_dir.join("tei-document.schema.json");
    fs::write(&alias_path, &schema_payload)?;

    Ok(())
}
