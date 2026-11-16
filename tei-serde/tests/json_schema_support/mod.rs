//! Validation helpers shared by the JSON Schema integration tests.

use anyhow::{Context, Result};

/// Compiles `schema` into a validator, failing when the schema is itself invalid.
pub fn compile_validator(schema: &serde_json::Value) -> Result<jsonschema::Validator> {
    jsonschema::validator_for(schema).context("compiled JSON schema must be valid")
}

/// One schema violation, reduced to the fields the scenarios assert on.
#[derive(Clone, Debug)]
pub struct ValidationErrorRecord {
    /// The violation's message.
    pub message: String,
    /// Where in the instance the violation was found.
    pub instance_path: String,
    /// Which schema keyword raised it.
    pub schema_path: String,
}

/// Collects every violation from a validator's error iterator.
pub fn collect_validation_errors<'i>(
    errors: impl Iterator<Item = jsonschema::ValidationError<'i>>,
) -> Vec<ValidationErrorRecord> {
    errors
        .map(|error| ValidationErrorRecord {
            message: error.to_string(),
            instance_path: error.instance_path().to_string(),
            schema_path: error.schema_path().to_string(),
        })
        .collect()
}
