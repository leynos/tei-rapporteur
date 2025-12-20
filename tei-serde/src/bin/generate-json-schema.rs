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

fn usage(program: &OsString) -> String {
    format!("usage: {} [--out-dir PATH]", Path::new(program).display())
}

fn parse_output_dir() -> Result<PathBuf, Box<dyn Error>> {
    let mut args = env::args_os();
    let program = args.next().ok_or("missing argv[0]")?;

    match (args.next(), args.next(), args.next()) {
        (None, None, None) => Ok(default_output_dir()),
        (Some(flag), Some(path), None) if flag == "--out-dir" => Ok(PathBuf::from(path)),
        _ => Err(usage(&program).into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = parse_output_dir()?;
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
