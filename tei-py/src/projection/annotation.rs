//! Python-facing projection helpers for stand-off annotations and TEI pointer
//! attributes.

use serde::{Deserialize, Serialize};
use tei_core::{Certainty, Pointer, PointerList, Span, SpanGroup, StandOff, TeiError};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyStandOff {
    #[serde(rename = "span_groups", skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) span_groups: Vec<PySpanGroup>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PySpanGroup {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) xml_id: Option<String>,
    #[serde(rename = "kind")]
    pub(crate) kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) resp: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) corresp: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) ana: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) spans: Vec<PySpan>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PySpan {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) xml_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) target: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "from")]
    pub(crate) from_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "to")]
    pub(crate) to_ref: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) source: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) resp: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) cert: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) corresp: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) ana: Vec<String>,
}

pub(crate) fn pointer_list_to_vec(value: Option<&PointerList>) -> Vec<String> {
    value.map_or_else(Vec::new, PointerList::to_strings)
}

pub(crate) fn pointer_list_from_vec(values: Vec<String>) -> Result<Option<PointerList>, TeiError> {
    if values.is_empty() {
        Ok(None)
    } else {
        PointerList::new(values).map(Some).map_err(TeiError::from)
    }
}

pub(crate) fn pointer_from_option(value: Option<String>) -> Result<Option<Pointer>, TeiError> {
    value.map(Pointer::new).transpose().map_err(TeiError::from)
}

pub(crate) fn certainty_to_string(value: Option<&Certainty>) -> Option<String> {
    value.map(|certainty| certainty.as_str().to_owned())
}

pub(crate) fn certainty_from_option(value: Option<String>) -> Result<Option<Certainty>, TeiError> {
    value
        .map(Certainty::new)
        .transpose()
        .map_err(TeiError::from)
}

impl From<&StandOff> for PyStandOff {
    fn from(value: &StandOff) -> Self {
        Self {
            span_groups: value.span_groups().iter().map(PySpanGroup::from).collect(),
        }
    }
}

impl TryFrom<PyStandOff> for StandOff {
    type Error = TeiError;

    fn try_from(value: PyStandOff) -> Result<Self, Self::Error> {
        let mut stand_off = Self::new();
        for span_group in value.span_groups {
            stand_off.add_span_group(SpanGroup::try_from(span_group)?);
        }
        Ok(stand_off)
    }
}

impl From<&SpanGroup> for PySpanGroup {
    fn from(value: &SpanGroup) -> Self {
        Self {
            xml_id: value.id().map(|id| id.as_str().to_owned()),
            kind: value.kind().to_owned(),
            resp: pointer_list_to_vec(value.resp()),
            corresp: pointer_list_to_vec(value.corresp()),
            ana: pointer_list_to_vec(value.ana()),
            spans: value.spans().iter().map(PySpan::from).collect(),
        }
    }
}

impl TryFrom<PySpanGroup> for SpanGroup {
    type Error = TeiError;

    fn try_from(value: PySpanGroup) -> Result<Self, Self::Error> {
        let mut span_group = Self::new(value.kind);
        if let Some(id) = value.xml_id {
            span_group.set_id(id)?;
        }
        if let Some(resp) = pointer_list_from_vec(value.resp)? {
            span_group.set_resp(resp);
        }
        if let Some(corresp) = pointer_list_from_vec(value.corresp)? {
            span_group.set_corresp(corresp);
        }
        if let Some(ana) = pointer_list_from_vec(value.ana)? {
            span_group.set_ana(ana);
        }
        for span in value.spans {
            span_group.add_span(Span::try_from(span)?);
        }
        Ok(span_group)
    }
}

impl From<&Span> for PySpan {
    fn from(value: &Span) -> Self {
        Self {
            xml_id: value.id().map(|id| id.as_str().to_owned()),
            target: pointer_list_to_vec(value.target()),
            from_ref: value.from().map(|pointer| pointer.as_str().to_owned()),
            to_ref: value.to().map(|pointer| pointer.as_str().to_owned()),
            source: pointer_list_to_vec(value.source()),
            resp: pointer_list_to_vec(value.resp()),
            cert: certainty_to_string(value.cert()),
            corresp: pointer_list_to_vec(value.corresp()),
            ana: pointer_list_to_vec(value.ana()),
        }
    }
}

impl TryFrom<PySpan> for Span {
    type Error = TeiError;

    fn try_from(value: PySpan) -> Result<Self, Self::Error> {
        let mut span = Self::new();
        if let Some(id) = value.xml_id {
            span.set_id(id)?;
        }
        if let Some(target) = pointer_list_from_vec(value.target)? {
            span.set_target(target);
        }
        if let Some(from_ref) = pointer_from_option(value.from_ref)? {
            span.set_from(from_ref);
        }
        if let Some(to_ref) = pointer_from_option(value.to_ref)? {
            span.set_to(to_ref);
        }
        apply_optional_span_list(&mut span, value.source, Self::set_source)?;
        apply_optional_span_list(&mut span, value.resp, Self::set_resp)?;
        if let Some(certainty) = certainty_from_option(value.cert)? {
            span.set_cert(certainty);
        }
        apply_optional_span_list(&mut span, value.corresp, Self::set_corresp)?;
        apply_optional_span_list(&mut span, value.ana, Self::set_ana)?;
        Ok(span)
    }
}

fn apply_optional_span_list(
    span: &mut Span,
    values: Vec<String>,
    setter: impl FnOnce(&mut Span, PointerList),
) -> Result<(), TeiError> {
    if let Some(pointer_list) = pointer_list_from_vec(values)? {
        setter(span, pointer_list);
    }
    Ok(())
}
