//! Canonical TEI element names used by spoken-text extraction.
//!
//! This module centralizes TEI XML element name strings as compile-time
//! constants so parser code avoids magic strings and typo-prone duplication.
//! The constants hold exact TEI P5 local names and are shared by the streaming
//! parser and its classification helpers.

pub(crate) const AB: &str = "ab";
pub(crate) const BIBL: &str = "bibl";
pub(crate) const BODY: &str = "body";
pub(crate) const BREAK: &str = "break";
pub(crate) const DIV: &str = "div";
pub(crate) const GAP: &str = "gap";
pub(crate) const HEAD: &str = "head";
pub(crate) const HI: &str = "hi";
pub(crate) const ITEM: &str = "item";
pub(crate) const L: &str = "l";
pub(crate) const LABEL: &str = "label";
pub(crate) const LIST: &str = "list";
pub(crate) const NOTE: &str = "note";
pub(crate) const P: &str = "p";
pub(crate) const PAUSE: &str = "pause";
pub(crate) const PTR: &str = "ptr";
pub(crate) const REF: &str = "ref";
pub(crate) const SEG: &str = "seg";
pub(crate) const SP: &str = "sp";
pub(crate) const SPEAKER: &str = "speaker";
pub(crate) const STAGE: &str = "stage";
pub(crate) const STAND_OFF: &str = "standOff";
pub(crate) const TEI: &str = "TEI";
pub(crate) const TEI_HEADER: &str = "teiHeader";
pub(crate) const TEXT: &str = "text";
pub(crate) const U: &str = "u";
