//! Python-facing header projection types and conversions.

use serde::{Deserialize, Serialize};
use tei_core::{
    AnnotationSystem, AnnotationSystemId, CiteData, CiteStructure, EncodingDesc,
    HeaderValidationError, LanguageTag, ProfileDesc, RefsDecl, RevisionChange, RevisionDesc,
    SpeakerName, TeiHeader,
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyProfileDesc {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) synopsis: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default, rename = "speakers")]
    pub(crate) speakers: Vec<SpeakerName>,
    #[serde(skip_serializing_if = "Vec::is_empty", default, rename = "languages")]
    pub(crate) languages: Vec<LanguageTag>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyAnnotationSystem {
    pub(crate) xml_id: AnnotationSystemId,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "desc")]
    pub(crate) desc: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyEncodingDesc {
    #[serde(
        rename = "annotation_systems",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub(crate) annotation_systems: Vec<PyAnnotationSystem>,
    #[serde(rename = "refs_decl", skip_serializing_if = "Option::is_none", default)]
    pub(crate) refs_decl: Option<PyRefsDecl>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyRefsDecl {
    #[serde(
        rename = "cite_structures",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub(crate) cite_structures: Vec<PyCiteStructure>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyCiteStructure {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) unit: Option<String>,
    #[serde(rename = "match_expr")]
    pub(crate) match_expr: String,
    #[serde(rename = "use_expr", skip_serializing_if = "Option::is_none", default)]
    pub(crate) use_expr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) delim: Option<String>,
    #[serde(rename = "cite_data", skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) cite_data: Vec<PyCiteData>,
    #[serde(
        rename = "cite_structures",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub(crate) cite_structures: Vec<PyCiteStructure>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyCiteData {
    pub(crate) property: String,
    #[serde(rename = "use_expr", skip_serializing_if = "Option::is_none", default)]
    pub(crate) use_expr: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyRevisionChange {
    #[serde(rename = "desc")]
    pub(crate) desc: String,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "resp")]
    pub(crate) resp: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyRevisionDesc {
    #[serde(rename = "change", skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) changes: Vec<PyRevisionChange>,
}

#[expect(
    clippy::struct_field_names,
    reason = "Field names mirror TEI sections and remain readable"
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyTeiHeader {
    #[serde(rename = "file_desc")]
    pub(crate) file_desc: tei_core::FileDesc,
    #[serde(rename = "profile_desc", skip_serializing_if = "Option::is_none")]
    pub(crate) profile_desc: Option<PyProfileDesc>,
    #[serde(rename = "encoding_desc", skip_serializing_if = "Option::is_none")]
    pub(crate) encoding_desc: Option<PyEncodingDesc>,
    #[serde(rename = "revision_desc", skip_serializing_if = "Option::is_none")]
    pub(crate) revision_desc: Option<PyRevisionDesc>,
}

impl From<&TeiHeader> for PyTeiHeader {
    fn from(header: &TeiHeader) -> Self {
        Self {
            file_desc: header.file_desc().clone(),
            profile_desc: header.profile_desc().map(PyProfileDesc::from),
            encoding_desc: header.encoding_desc().map(PyEncodingDesc::from),
            revision_desc: header.revision_desc().map(PyRevisionDesc::from),
        }
    }
}

impl TryFrom<PyTeiHeader> for TeiHeader {
    type Error = HeaderValidationError;

    fn try_from(value: PyTeiHeader) -> Result<Self, Self::Error> {
        let mut header = Self::new(value.file_desc);
        if let Some(profile) = value.profile_desc {
            header = header.with_profile_desc(ProfileDesc::try_from(profile)?);
        }
        if let Some(encoding) = value.encoding_desc {
            header = header.with_encoding_desc(EncodingDesc::try_from(encoding)?);
        }
        if let Some(revision) = value.revision_desc {
            header = header.with_revision_desc(RevisionDesc::try_from(revision)?);
        }
        Ok(header)
    }
}

impl From<&ProfileDesc> for PyProfileDesc {
    fn from(value: &ProfileDesc) -> Self {
        Self {
            synopsis: value.synopsis().map(str::to_owned),
            speakers: value.speakers().to_vec(),
            languages: value.languages().to_vec(),
        }
    }
}

impl TryFrom<PyProfileDesc> for ProfileDesc {
    type Error = HeaderValidationError;

    fn try_from(value: PyProfileDesc) -> Result<Self, Self::Error> {
        let mut profile = Self::new();
        if let Some(synopsis) = value.synopsis {
            profile = profile.with_synopsis(synopsis);
        }
        for speaker in value.speakers {
            profile.add_speaker(speaker.as_str())?;
        }
        for language in value.languages {
            profile.add_language(language.as_str())?;
        }
        Ok(profile)
    }
}

impl From<&AnnotationSystem> for PyAnnotationSystem {
    fn from(system: &AnnotationSystem) -> Self {
        Self {
            xml_id: system.identifier().clone(),
            desc: system.description().map(str::to_owned),
        }
    }
}

impl TryFrom<PyAnnotationSystem> for AnnotationSystem {
    type Error = HeaderValidationError;

    fn try_from(system: PyAnnotationSystem) -> Result<Self, Self::Error> {
        let description = system.desc.unwrap_or_default();
        Self::new(system.xml_id.as_str(), description)
    }
}

impl From<&EncodingDesc> for PyEncodingDesc {
    fn from(value: &EncodingDesc) -> Self {
        Self {
            annotation_systems: value
                .annotation_systems()
                .iter()
                .map(PyAnnotationSystem::from)
                .collect(),
            refs_decl: value.refs_decl().map(PyRefsDecl::from),
        }
    }
}

impl TryFrom<PyEncodingDesc> for EncodingDesc {
    type Error = HeaderValidationError;

    fn try_from(value: PyEncodingDesc) -> Result<Self, Self::Error> {
        let mut encoding = Self::new();
        for system in value.annotation_systems {
            encoding.add_annotation_system(AnnotationSystem::try_from(system)?);
        }
        if let Some(refs_decl) = value.refs_decl {
            encoding.set_refs_decl(RefsDecl::try_from(refs_decl)?);
        }
        Ok(encoding)
    }
}

impl From<&RefsDecl> for PyRefsDecl {
    fn from(value: &RefsDecl) -> Self {
        Self {
            cite_structures: value
                .cite_structures()
                .iter()
                .map(PyCiteStructure::from)
                .collect(),
        }
    }
}

impl TryFrom<PyRefsDecl> for RefsDecl {
    type Error = HeaderValidationError;

    fn try_from(value: PyRefsDecl) -> Result<Self, Self::Error> {
        let mut refs_decl = Self::new();
        for cite_structure in value.cite_structures {
            refs_decl.add_cite_structure(CiteStructure::from(cite_structure));
        }
        Ok(refs_decl)
    }
}

impl From<&CiteStructure> for PyCiteStructure {
    fn from(value: &CiteStructure) -> Self {
        Self {
            unit: value.unit().map(str::to_owned),
            match_expr: value.match_expr().to_owned(),
            use_expr: value.use_expr().map(str::to_owned),
            delim: value.delim().map(str::to_owned),
            cite_data: value.cite_data().iter().map(PyCiteData::from).collect(),
            cite_structures: value.children().iter().map(Self::from).collect(),
        }
    }
}

impl From<PyCiteStructure> for CiteStructure {
    fn from(value: PyCiteStructure) -> Self {
        let mut cite_structure = Self::new(value.match_expr);
        if let Some(unit) = value.unit {
            cite_structure = cite_structure.with_unit(unit);
        }
        if let Some(use_expr) = value.use_expr {
            cite_structure = cite_structure.with_use_expr(use_expr);
        }
        if let Some(delim) = value.delim {
            cite_structure = cite_structure.with_delim(delim);
        }
        for cite_data in value.cite_data {
            cite_structure.add_cite_data(CiteData::from(cite_data));
        }
        for child in value.cite_structures {
            cite_structure.add_child(Self::from(child));
        }
        cite_structure
    }
}

impl From<&CiteData> for PyCiteData {
    fn from(value: &CiteData) -> Self {
        Self {
            property: value.property().to_owned(),
            use_expr: value.use_expr().map(str::to_owned),
        }
    }
}

impl From<PyCiteData> for CiteData {
    fn from(value: PyCiteData) -> Self {
        let property = value.property;
        value.use_expr.map_or_else(
            || Self::new(property.clone()),
            |use_expr| Self::new(property.clone()).with_use_expr(use_expr),
        )
    }
}

impl From<&RevisionChange> for PyRevisionChange {
    fn from(change: &RevisionChange) -> Self {
        Self {
            desc: change.description().to_owned(),
            resp: change.resp().map(|resp| resp.as_ref().to_owned()),
        }
    }
}

impl TryFrom<PyRevisionChange> for RevisionChange {
    type Error = HeaderValidationError;

    fn try_from(change: PyRevisionChange) -> Result<Self, Self::Error> {
        Self::new(change.desc, change.resp.unwrap_or_default())
    }
}

impl From<&RevisionDesc> for PyRevisionDesc {
    fn from(desc: &RevisionDesc) -> Self {
        Self {
            changes: desc.iter().map(PyRevisionChange::from).collect(),
        }
    }
}

impl TryFrom<PyRevisionDesc> for RevisionDesc {
    type Error = HeaderValidationError;

    fn try_from(desc: PyRevisionDesc) -> Result<Self, Self::Error> {
        let mut revision = Self::new();
        for change in desc.changes {
            revision.add_change(RevisionChange::try_from(change)?);
        }
        Ok(revision)
    }
}
