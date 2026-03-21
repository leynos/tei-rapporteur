//! Identifier registration and annotation-system validation helpers.

use std::collections::HashSet;

use crate::TeiDocument;

use super::ValidationError;

pub(super) fn validate_annotation_systems(
    document: &TeiDocument,
    seen_ids: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    let Some(encoding) = document.header().encoding_desc() else {
        return Ok(());
    };

    for system in encoding.annotation_systems() {
        record_id(system.identifier().as_str(), seen_ids)?;
    }

    Ok(())
}

pub(super) fn record_id(value: &str, sink: &mut HashSet<String>) -> Result<(), ValidationError> {
    if sink.insert(value.to_owned()) {
        Ok(())
    } else {
        Err(ValidationError::DuplicateXmlId {
            id: value.to_owned(),
        })
    }
}
