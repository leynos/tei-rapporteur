//! Canonical citation declarations used from `<encodingDesc>`.
//!
//! The TEI Episodic profile uses `refsDecl` to declare canonical citation and
//! metadata extraction structures without forcing callers to model the whole
//! TEI header universe.

use serde::de::Error as DeError;
use serde::{Deserialize, Serialize};

fn trim_required_string(value: impl Into<String>) -> String {
    value.into().trim().to_owned()
}

fn trim_optional_string(value: impl Into<String>) -> Option<String> {
    let trimmed = trim_required_string(value);
    (!trimmed.is_empty()).then_some(trimmed)
}

fn deserialize_trimmed_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let trimmed = trim_required_string(value);
    if trimmed.is_empty() {
        return Err(DeError::custom("value must not be empty"));
    }

    Ok(trimmed)
}

fn deserialize_trimmed_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.and_then(trim_optional_string))
}

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
    #[serde(
        rename = "@unit",
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "deserialize_trimmed_option"
    )]
    unit: Option<String>,
    #[serde(rename = "@match", deserialize_with = "deserialize_trimmed_string")]
    match_expr: String,
    #[serde(
        rename = "@use",
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "deserialize_trimmed_option"
    )]
    use_expr: Option<String>,
    #[serde(
        rename = "@delim",
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "deserialize_trimmed_option"
    )]
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
            match_expr: trim_required_string(match_expr),
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
        self.unit = trim_optional_string(unit);
        self
    }

    /// Assigns the optional use expression.
    #[must_use]
    pub fn with_use_expr(mut self, use_expr: impl Into<String>) -> Self {
        self.use_expr = trim_optional_string(use_expr);
        self
    }

    /// Assigns the optional delimiter.
    #[must_use]
    pub fn with_delim(mut self, delim: impl Into<String>) -> Self {
        self.delim = trim_optional_string(delim);
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
    #[serde(rename = "@property", deserialize_with = "deserialize_trimmed_string")]
    property: String,
    #[serde(
        rename = "@use",
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "deserialize_trimmed_option"
    )]
    use_expr: Option<String>,
}

impl CiteData {
    /// Creates a `citeData` entry for the named property.
    #[must_use]
    pub fn new(property: impl Into<String>) -> Self {
        Self {
            property: trim_required_string(property),
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
        self.use_expr = trim_optional_string(use_expr);
        self
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for citation declaration helpers.

    use super::*;
    use tei_serde::json::from_str as from_json;

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

    #[test]
    fn deserialization_trims_required_and_optional_citation_attributes() {
        let cite_structure: CiteStructure =
            from_json(r#"{"@match":" //u ","@unit":" utterance ","@use":"  ","@delim":" / "}"#)
                .unwrap_or_else(|error| panic!("citeStructure JSON: {error}"));

        assert_eq!(cite_structure.match_expr(), "//u");
        assert_eq!(cite_structure.unit(), Some("utterance"));
        assert_eq!(cite_structure.use_expr(), None);
        assert_eq!(cite_structure.delim(), Some("/"));
    }

    #[test]
    fn deserialization_trims_cite_data_attributes() {
        let cite_data: CiteData = from_json(r#"{"@property":" speaker ","@use":"  @who  "}"#)
            .unwrap_or_else(|error| panic!("citeData JSON: {error}"));

        assert_eq!(cite_data.property(), "speaker");
        assert_eq!(cite_data.use_expr(), Some("@who"));
    }
}
