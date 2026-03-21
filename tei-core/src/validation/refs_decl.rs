//! Citation-declaration validation helpers.

use crate::{CiteStructure, TeiDocument};

use super::{ValidationError, stand_off::validate_non_empty_field};

pub(super) fn validate_refs_decl(document: &TeiDocument) -> Result<(), ValidationError> {
    let Some(encoding) = document.header().encoding_desc() else {
        return Ok(());
    };
    let Some(refs_decl) = encoding.refs_decl() else {
        return Ok(());
    };

    for cite_structure in refs_decl.cite_structures() {
        validate_cite_structure(cite_structure)?;
    }

    Ok(())
}

fn validate_cite_structure(cite_structure: &CiteStructure) -> Result<(), ValidationError> {
    validate_non_empty_field(cite_structure.match_expr(), "citeStructure @match")?;

    for cite_data in cite_structure.cite_data() {
        validate_non_empty_field(cite_data.property(), "citeData @property")?;
    }

    for child in cite_structure.children() {
        validate_cite_structure(child)?;
    }

    Ok(())
}
