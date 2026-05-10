//! Element classification predicates for spoken-text extraction.

use super::element_names::{
    AB, BIBL, BODY, BREAK, DIV, GAP, HEAD, HI, ITEM, L, LABEL, LIST, NOTE, P, PAUSE, PTR, REF, SEG,
    SP, SPEAKER, STAGE, STAND_OFF, TEI_HEADER, U,
};

/// Reports whether an element and descendants are excluded from spoken text.
pub(crate) fn is_excluded_element(name: &str) -> bool {
    matches!(
        name,
        SPEAKER
            | STAGE
            | NOTE
            | LIST
            | ITEM
            | LABEL
            | HEAD
            | REF
            | PTR
            | BIBL
            | TEI_HEADER
            | STAND_OFF
    )
}

/// Reports whether an element contributes a silent word boundary.
pub(crate) fn is_silent_boundary_element(name: &str) -> bool {
    matches!(name, PAUSE | GAP | BREAK)
}

/// Reports whether an element is accepted in the body profile.
pub(crate) fn is_body_element(name: &str) -> bool {
    matches!(
        name,
        BODY | DIV
            | SP
            | SPEAKER
            | STAGE
            | P
            | U
            | AB
            | L
            | SEG
            | HI
            | NOTE
            | LIST
            | ITEM
            | LABEL
            | HEAD
            | REF
            | PTR
            | BIBL
    ) || is_silent_boundary_element(name)
}
