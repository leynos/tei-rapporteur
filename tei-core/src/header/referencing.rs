//! Canonical citation declarations used from `<encodingDesc>`.
//!
//! The TEI Episodic profile uses `refsDecl` to declare canonical citation and
//! metadata extraction structures without forcing callers to model the whole
//! TEI header universe.

use serde::{Deserialize, Serialize};

/// Citation declaration container.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "refsDecl")]
pub struct RefsDecl {
    #[serde(
        rename = "citeStructure",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    cite_structures: Vec<CiteStructure>,
}

impl RefsDecl {
    /// Creates an empty citation declaration container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a citation structure.
    pub fn add_cite_structure(&mut self, cite_structure: CiteStructure) {
        self.cite_structures.push(cite_structure);
    }

    /// Returns the declared citation structures.
    #[must_use]
    pub const fn cite_structures(&self) -> &[CiteStructure] {
        self.cite_structures.as_slice()
    }

    /// Reports whether the declaration contains any structures.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cite_structures.is_empty()
    }
}

/// Nested canonical citation structure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "citeStructure")]
pub struct CiteStructure {
    #[serde(rename = "@unit", skip_serializing_if = "Option::is_none", default)]
    unit: Option<String>,
    #[serde(rename = "@match")]
    match_expr: String,
    #[serde(rename = "@use", skip_serializing_if = "Option::is_none", default)]
    use_expr: Option<String>,
    #[serde(rename = "@delim", skip_serializing_if = "Option::is_none", default)]
    delim: Option<String>,
    #[serde(rename = "citeData", default, skip_serializing_if = "Vec::is_empty")]
    cite_data: Vec<CiteData>,
    #[serde(
        rename = "citeStructure",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    children: Vec<CiteStructure>,
}

impl CiteStructure {
    /// Creates a citation structure with the required `@match` expression.
    #[must_use]
    pub fn new(match_expr: impl Into<String>) -> Self {
        Self {
            unit: None,
            match_expr: match_expr.into().trim().to_owned(),
            use_expr: None,
            delim: None,
            cite_data: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Returns the optional unit label.
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    /// Returns the required match expression.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on the current MSRV."
    )]
    pub fn match_expr(&self) -> &str {
        self.match_expr.as_str()
    }

    /// Returns the optional use expression.
    #[must_use]
    pub fn use_expr(&self) -> Option<&str> {
        self.use_expr.as_deref()
    }

    /// Returns the optional delimiter.
    #[must_use]
    pub fn delim(&self) -> Option<&str> {
        self.delim.as_deref()
    }

    /// Returns the declared `citeData` entries.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Keep accessor constness aligned with adjacent string-backed accessors."
    )]
    pub fn cite_data(&self) -> &[CiteData] {
        self.cite_data.as_slice()
    }

    /// Returns child citation structures.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Keep accessor constness aligned with adjacent string-backed accessors."
    )]
    pub fn children(&self) -> &[Self] {
        self.children.as_slice()
    }

    /// Assigns the optional unit label.
    #[must_use]
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        let trimmed = unit.into().trim().to_owned();
        self.unit = (!trimmed.is_empty()).then_some(trimmed);
        self
    }

    /// Assigns the optional use expression.
    #[must_use]
    pub fn with_use_expr(mut self, use_expr: impl Into<String>) -> Self {
        let trimmed = use_expr.into().trim().to_owned();
        self.use_expr = (!trimmed.is_empty()).then_some(trimmed);
        self
    }

    /// Assigns the optional delimiter.
    #[must_use]
    pub fn with_delim(mut self, delim: impl Into<String>) -> Self {
        let trimmed = delim.into().trim().to_owned();
        self.delim = (!trimmed.is_empty()).then_some(trimmed);
        self
    }

    /// Appends a `citeData` entry.
    pub fn add_cite_data(&mut self, cite_data: CiteData) {
        self.cite_data.push(cite_data);
    }

    /// Appends a nested citation structure.
    pub fn add_child(&mut self, child: Self) {
        self.children.push(child);
    }
}

/// Property extraction entry nested inside a citation structure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "citeData")]
pub struct CiteData {
    #[serde(rename = "@property")]
    property: String,
    #[serde(rename = "@use", skip_serializing_if = "Option::is_none", default)]
    use_expr: Option<String>,
}

impl CiteData {
    /// Creates a `citeData` entry for the named property.
    #[must_use]
    pub fn new(property: impl Into<String>) -> Self {
        Self {
            property: property.into().trim().to_owned(),
            use_expr: None,
        }
    }

    /// Returns the property name.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on the current MSRV."
    )]
    pub fn property(&self) -> &str {
        self.property.as_str()
    }

    /// Returns the optional use expression.
    #[must_use]
    pub fn use_expr(&self) -> Option<&str> {
        self.use_expr.as_deref()
    }

    /// Assigns the optional use expression.
    #[must_use]
    pub fn with_use_expr(mut self, use_expr: impl Into<String>) -> Self {
        let trimmed = use_expr.into().trim().to_owned();
        self.use_expr = (!trimmed.is_empty()).then_some(trimmed);
        self
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for citation declaration helpers.

    use super::*;

    #[test]
    fn refs_decl_tracks_nested_structures() {
        let mut root = CiteStructure::new("//u[@xml:id]");
        root.add_cite_data(CiteData::new("speaker"));
        root.add_child(CiteStructure::new(".//span"));

        let mut refs_decl = RefsDecl::new();
        refs_decl.add_cite_structure(root);

        let cite_structure = refs_decl
            .cite_structures()
            .first()
            .expect("refsDecl should contain a citeStructure");

        assert_eq!(refs_decl.cite_structures().len(), 1);
        assert_eq!(cite_structure.cite_data().len(), 1);
        assert_eq!(cite_structure.children().len(), 1);
    }
}
