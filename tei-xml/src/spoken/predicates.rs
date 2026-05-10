//! Element classification predicates for spoken-text extraction.

/// Reports whether an element and descendants are excluded from spoken text.
pub(crate) fn is_excluded_element(name: &str) -> bool {
    matches!(
        name,
        "speaker"
            | "stage"
            | "note"
            | "list"
            | "item"
            | "label"
            | "head"
            | "ref"
            | "ptr"
            | "bibl"
            | "teiHeader"
            | "standOff"
    )
}

/// Reports whether an element contributes a silent word boundary.
pub(crate) fn is_silent_boundary_element(name: &str) -> bool {
    matches!(name, "pause" | "gap" | "break")
}

/// Reports whether an element is accepted in the body profile.
pub(crate) fn is_body_element(name: &str) -> bool {
    matches!(
        name,
        "body"
            | "div"
            | "sp"
            | "speaker"
            | "stage"
            | "p"
            | "u"
            | "ab"
            | "l"
            | "seg"
            | "hi"
            | "note"
            | "list"
            | "item"
            | "label"
            | "head"
            | "ref"
            | "ptr"
            | "bibl"
    ) || is_silent_boundary_element(name)
}
