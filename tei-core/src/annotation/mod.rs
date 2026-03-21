//! Stand-off annotation structures for citation and provenance overlays.
//!
//! These types keep many-to-many analytical markup outside the primary text
//! flow while reusing the same validated TEI pointer and certainty wrappers as
//! body-level provenance attributes.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::text::{Certainty, IdentifierValidationError, Pointer, PointerList, XmlId};

/// Errors raised when validating stand-off annotation metadata.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum AnnotationValidationError {
    /// The `spanGrp @type` label trimmed to an empty string.
    #[error("spanGrp @type must not be empty")]
    EmptySpanGroupKind,
}

/// Root TEI stand-off annotation container.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "standOff")]
pub struct StandOff {
    #[serde(rename = "spanGrp", default, skip_serializing_if = "Vec::is_empty")]
    span_groups: Vec<SpanGroup>,
}

impl StandOff {
    /// Creates an empty stand-off container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a span group.
    pub fn add_span_group(&mut self, span_group: SpanGroup) {
        self.span_groups.push(span_group);
    }

    /// Returns the span groups recorded in the stand-off layer.
    #[must_use]
    pub const fn span_groups(&self) -> &[SpanGroup] {
        self.span_groups.as_slice()
    }

    /// Finds a mutable span group by its `xml:id`.
    pub fn find_span_group_mut(&mut self, id: &str) -> Option<&mut SpanGroup> {
        self.span_groups
            .iter_mut()
            .find(|span_group| span_group.id().is_some_and(|value| value.as_str() == id))
    }

    /// Reports whether the stand-off layer is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.span_groups.is_empty()
    }
}

/// A logical grouping of analytical spans.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "spanGrp")]
pub struct SpanGroup {
    #[serde(
        rename = "@xml:id",
        alias = "@id",
        skip_serializing_if = "Option::is_none",
        default
    )]
    id: Option<XmlId>,
    #[serde(rename = "@type")]
    kind: String,
    #[serde(rename = "@resp", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    resp: Option<PointerList>,
    #[serde(rename = "@corresp", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    corresp: Option<PointerList>,
    #[serde(rename = "@ana", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    ana: Option<PointerList>,
    #[serde(rename = "span", default, skip_serializing_if = "Vec::is_empty")]
    spans: Vec<Span>,
}

impl SpanGroup {
    /// Creates a span group with the provided `@type`.
    ///
    /// # Errors
    ///
    /// Returns [`AnnotationValidationError::EmptySpanGroupKind`] when the
    /// supplied type label trims to an empty string.
    pub fn new(kind: impl Into<String>) -> Result<Self, AnnotationValidationError> {
        let normalized_kind = kind.into().trim().to_owned();
        if normalized_kind.is_empty() {
            return Err(AnnotationValidationError::EmptySpanGroupKind);
        }

        Ok(Self {
            id: None,
            kind: normalized_kind,
            resp: None,
            corresp: None,
            ana: None,
            spans: Vec::new(),
        })
    }

    /// Returns the optional identifier.
    #[must_use]
    pub const fn id(&self) -> Option<&XmlId> {
        self.id.as_ref()
    }

    /// Returns the group type label.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the responsibility pointers, when present.
    #[must_use]
    pub const fn resp(&self) -> Option<&PointerList> {
        self.resp.as_ref()
    }

    /// Returns the correspondence pointers, when present.
    #[must_use]
    pub const fn corresp(&self) -> Option<&PointerList> {
        self.corresp.as_ref()
    }

    /// Returns the analysis pointers, when present.
    #[must_use]
    pub const fn ana(&self) -> Option<&PointerList> {
        self.ana.as_ref()
    }

    /// Returns the contained spans.
    #[must_use]
    pub const fn spans(&self) -> &[Span] {
        self.spans.as_slice()
    }

    /// Assigns an `xml:id`.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierValidationError`] when the identifier is invalid.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), IdentifierValidationError> {
        self.id = Some(XmlId::new(id)?);
        Ok(())
    }

    /// Sets the responsibility pointers.
    pub fn set_resp(&mut self, resp: PointerList) {
        self.resp = Some(resp);
    }

    /// Sets the correspondence pointers.
    pub fn set_corresp(&mut self, corresp: PointerList) {
        self.corresp = Some(corresp);
    }

    /// Sets the analysis pointers.
    pub fn set_ana(&mut self, ana: PointerList) {
        self.ana = Some(ana);
    }

    /// Appends a span to the group.
    pub fn add_span(&mut self, span: Span) {
        self.spans.push(span);
    }
}

/// Stand-off analytical span.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "span")]
pub struct Span {
    #[serde(
        rename = "@xml:id",
        alias = "@id",
        skip_serializing_if = "Option::is_none",
        default
    )]
    id: Option<XmlId>,
    #[serde(rename = "@target", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    target: Option<PointerList>,
    #[serde(rename = "@from", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    from: Option<Pointer>,
    #[serde(rename = "@to", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    to: Option<Pointer>,
    #[serde(rename = "@source", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    source: Option<PointerList>,
    #[serde(rename = "@resp", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    resp: Option<PointerList>,
    #[serde(rename = "@cert", skip_serializing_if = "Option::is_none", default)]
    cert: Option<Certainty>,
    #[serde(rename = "@corresp", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    corresp: Option<PointerList>,
    #[serde(rename = "@ana", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    ana: Option<PointerList>,
}

impl Span {
    /// Creates an empty span that can then be populated with pointers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the optional identifier.
    #[must_use]
    pub const fn id(&self) -> Option<&XmlId> {
        self.id.as_ref()
    }

    /// Returns the many-target attribute, when present.
    #[must_use]
    pub const fn target(&self) -> Option<&PointerList> {
        self.target.as_ref()
    }

    /// Returns the optional range start.
    #[must_use]
    pub const fn from(&self) -> Option<&Pointer> {
        self.from.as_ref()
    }

    /// Returns the optional range end.
    #[must_use]
    pub const fn to(&self) -> Option<&Pointer> {
        self.to.as_ref()
    }

    /// Returns the optional source pointers.
    #[must_use]
    pub const fn source(&self) -> Option<&PointerList> {
        self.source.as_ref()
    }

    /// Returns the optional responsibility pointers.
    #[must_use]
    pub const fn resp(&self) -> Option<&PointerList> {
        self.resp.as_ref()
    }

    /// Returns the optional certainty value.
    #[must_use]
    pub const fn cert(&self) -> Option<&Certainty> {
        self.cert.as_ref()
    }

    /// Returns the optional correspondence pointers.
    #[must_use]
    pub const fn corresp(&self) -> Option<&PointerList> {
        self.corresp.as_ref()
    }

    /// Returns the optional analysis pointers.
    #[must_use]
    pub const fn ana(&self) -> Option<&PointerList> {
        self.ana.as_ref()
    }

    /// Assigns an `xml:id`.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierValidationError`] when the identifier is invalid.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), IdentifierValidationError> {
        self.id = Some(XmlId::new(id)?);
        Ok(())
    }

    /// Sets the many-target attribute.
    pub fn set_target(&mut self, target: PointerList) {
        self.target = Some(target);
    }

    /// Sets the range start pointer.
    pub fn set_from(&mut self, from: Pointer) {
        self.from = Some(from);
    }

    /// Sets the range end pointer.
    pub fn set_to(&mut self, to: Pointer) {
        self.to = Some(to);
    }

    /// Sets the source pointers.
    pub fn set_source(&mut self, source: PointerList) {
        self.source = Some(source);
    }

    /// Sets the responsibility pointers.
    pub fn set_resp(&mut self, resp: PointerList) {
        self.resp = Some(resp);
    }

    /// Sets the certainty value.
    pub fn set_cert(&mut self, cert: Certainty) {
        self.cert = Some(cert);
    }

    /// Sets the correspondence pointers.
    pub fn set_corresp(&mut self, corresp: PointerList) {
        self.corresp = Some(corresp);
    }

    /// Sets the analysis pointers.
    pub fn set_ana(&mut self, ana: PointerList) {
        self.ana = Some(ana);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for stand-off annotation helpers.

    use super::*;

    #[test]
    fn stand_off_tracks_groups_by_identifier() {
        let mut stand_off = StandOff::new();
        let mut group =
            SpanGroup::new("citation").unwrap_or_else(|error| panic!("group kind: {error}"));
        group
            .set_id("grp1")
            .unwrap_or_else(|error| panic!("group id should validate: {error}"));
        stand_off.add_span_group(group);

        assert!(stand_off.find_span_group_mut("grp1").is_some());
        assert!(stand_off.find_span_group_mut("missing").is_none());
    }

    #[test]
    fn span_records_pointer_metadata() {
        let mut span = Span::new();
        span.set_target(
            PointerList::new(["#u1", "#u2"])
                .unwrap_or_else(|error| panic!("target pointers should validate: {error}")),
        );
        span.set_cert(
            Certainty::new("high")
                .unwrap_or_else(|error| panic!("certainty should validate: {error}")),
        );

        assert_eq!(
            span.target().map(PointerList::to_strings),
            Some(vec![String::from("#u1"), String::from("#u2")])
        );
        assert_eq!(span.cert().map(Certainty::as_str), Some("high"));
    }

    #[test]
    fn span_group_rejects_blank_kind() {
        assert_eq!(
            SpanGroup::new("   "),
            Err(AnnotationValidationError::EmptySpanGroupKind)
        );
    }
}
