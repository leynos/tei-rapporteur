//! Internal parser frame types for spoken-text extraction.

use std::collections::BTreeMap;

/// XML element stack frame with locator state.
#[derive(Clone, Debug)]
pub(crate) struct ElementFrame {
    /// Local element name.
    pub(crate) name: String,
    /// Stable XPath-like locator for this element.
    pub(crate) locator: String,
    /// Per-child-name counters used to assign locator indexes.
    pub(crate) child_counts: BTreeMap<String, usize>,
    /// Whether this element and descendants are excluded from speech.
    pub(crate) is_excluded: bool,
}

impl ElementFrame {
    /// Builds a stack frame for a newly entered XML element.
    pub(crate) const fn new(name: String, locator: String, is_excluded: bool) -> Self {
        Self {
            name,
            locator,
            child_counts: BTreeMap::new(),
            is_excluded,
        }
    }
}
