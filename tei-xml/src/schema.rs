//! Embedded Relax NG schema for the TEI Episodic Profile.
//!
//! The schema is derived from `schemas/tei-episodic-profile.odd` and committed
//! to the repository so that external validation is deterministic and does not
//! require TEI tooling at runtime.

use std::{fs, path::Path};

use tei_core::TeiError;

const RELAX_NG_SCHEMA: &str = include_str!("../resources/tei-episodic-profile.rng");

/// Returns the Relax NG schema describing the TEI Episodic Profile.
#[must_use]
pub const fn relax_ng_schema() -> &'static str {
    RELAX_NG_SCHEMA
}

/// Writes the embedded Relax NG schema to disk.
///
/// # Errors
///
/// Returns [`TeiError::Xml`] when the schema cannot be written to the supplied
/// path.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use tei_xml::write_relax_ng_schema;
///
/// let output = PathBuf::from("tei-episodic-profile.rng");
/// write_relax_ng_schema(output)?;
/// # Ok::<(), tei_core::TeiError>(())
/// ```
pub fn write_relax_ng_schema(path: impl AsRef<Path>) -> Result<(), TeiError> {
    fs::write(path.as_ref(), RELAX_NG_SCHEMA).map_err(|error| {
        TeiError::xml(format!(
            "failed to write Relax NG schema to {}: {error}",
            path.as_ref().display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn relax_ng_schema_is_non_empty_and_mentions_root_element() {
        let schema = relax_ng_schema();

        assert!(
            !schema.trim().is_empty(),
            "embedded schema should not be blank"
        );
        assert!(
            schema.contains("<grammar"),
            "schema should contain a grammar element"
        );
        assert!(
            schema.contains("<element name=\"TEI\">"),
            "schema should define the TEI root element"
        );
    }

    #[test]
    fn writes_schema_to_disk() {
        let dir = tempdir().expect("temp dir should be created");
        let path = dir.path().join("tei-episodic-profile.rng");

        write_relax_ng_schema(&path).expect("schema should write to temp path");

        let written = fs::read_to_string(&path).expect("schema should be readable");
        assert_eq!(written, relax_ng_schema());
    }

    #[test]
    fn reports_write_failures() {
        let dir = tempdir().expect("temp dir should be created");
        let path = dir
            .path()
            .join("missing-parent")
            .join("tei-episodic-profile.rng");

        let error = write_relax_ng_schema(&path).expect_err("writing should fail");
        let message = error.to_string();

        assert!(
            message.contains("failed to write Relax NG schema"),
            "unexpected error message: {message}"
        );
    }
}
