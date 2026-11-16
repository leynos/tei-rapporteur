//! Validation helpers shared by the JSON Schema integration tests.

use anyhow::{Context, Result};

/// Compiles `schema` into a validator, failing when the schema is itself invalid.
///
/// # Examples
///
/// ```text
/// let schema = tei_serde::schema::tei_document_schema().to_value();
/// let validator = compile_validator(&schema)?;
/// let instance = tei_serde::json::to_value(&document)?;
/// assert!(validator.is_valid(&instance), "instance should satisfy the schema");
/// ```
///
/// An invalid schema, such as `{"type": 42}`, yields an `Err` whose message reads
/// "compiled JSON schema must be valid".
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
///
/// # Examples
///
/// ```text
/// let errors = collect_validation_errors(validator.iter_errors(&instance));
/// assert!(errors.is_empty(), "instance should satisfy the schema: {errors:?}");
/// ```
///
/// Each record carries the message, the instance path and the schema path of one violation.
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
