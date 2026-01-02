//! Python-facing projection types with tagged unions for inline content and
//! body blocks.
//!
//! The core TEI model uses untagged Serde enums for inline content, which
//! prevents Python from defining fully typed `msgspec.Struct` unions. This
//! module introduces a parallel, internally tagged representation used at the
//! FFI boundary. All dictionary and `MessagePack` exchange now flows through
//! these projection types so that Python callers receive and submit stable,
//! unambiguous payloads.

use serde::{Deserialize, Serialize, de::Error as DeError};
use tei_core::{
    AnnotationSystem, AnnotationSystemId, BodyBlock, BodyContentError, EncodingDesc, FileDesc,
    Inline, LanguageTag, P, Pause, ProfileDesc, RevisionChange, RevisionDesc, SpeakerName, TeiBody,
    TeiDocument, TeiHeader, TeiText, Utterance,
};
use tei_serde::json::Value;

/// Tagged inline content for Python consumption.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub(crate) enum PyInline {
    #[serde(rename = "text")]
    Text { value: String },
    #[serde(rename = "hi")]
    Hi {
        #[serde(skip_serializing_if = "Option::is_none")]
        rend: Option<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "pause")]
    Pause {
        #[serde(skip_serializing_if = "Option::is_none")]
        dur: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
    },
}

/// Tagged body block union (paragraph or utterance) for Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub(crate) enum PyBodyBlock {
    #[serde(rename = "paragraph")]
    Paragraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "utterance")]
    Utterance {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        speaker: Option<String>,
        content: Vec<PyInline>,
    },
}

/// Tagged streaming event union surfaced to Python.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub(crate) enum PyEvent {
    #[serde(rename = "document_start")]
    DocumentStart,
    #[serde(rename = "header")]
    Header { header: PyTeiHeader },
    #[serde(rename = "paragraph")]
    Paragraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "utterance")]
    Utterance {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        speaker: Option<String>,
        content: Vec<PyInline>,
    },
    #[serde(rename = "document_end")]
    DocumentEnd,
}

/// Python projection of the TEI body.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyTeiBody {
    #[serde(default)]
    pub(crate) blocks: Vec<PyBodyBlock>,
}

/// Python projection of the `<text>` element.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyTeiText {
    pub(crate) body: PyTeiBody,
}

/// Python projection of the TEI header.
#[expect(
    clippy::struct_field_names,
    reason = "Field names mirror TEI sections and remain readable"
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyTeiHeader {
    #[serde(rename = "file_desc")]
    pub(crate) file_desc: FileDesc,
    #[serde(rename = "profile_desc", skip_serializing_if = "Option::is_none")]
    pub(crate) profile_desc: Option<PyProfileDesc>,
    #[serde(rename = "encoding_desc", skip_serializing_if = "Option::is_none")]
    pub(crate) encoding_desc: Option<PyEncodingDesc>,
    #[serde(rename = "revision_desc", skip_serializing_if = "Option::is_none")]
    pub(crate) revision_desc: Option<PyRevisionDesc>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyProfileDesc {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    synopsis: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default, rename = "speakers")]
    speakers: Vec<SpeakerName>,
    #[serde(skip_serializing_if = "Vec::is_empty", default, rename = "languages")]
    languages: Vec<LanguageTag>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyAnnotationSystem {
    #[serde(rename = "xml_id")]
    xml_id: AnnotationSystemId,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "desc")]
    desc: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyEncodingDesc {
    #[serde(
        rename = "annotation_systems",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    annotation_systems: Vec<PyAnnotationSystem>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyRevisionChange {
    #[serde(rename = "desc")]
    desc: String,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "resp")]
    resp: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyRevisionDesc {
    #[serde(rename = "change", skip_serializing_if = "Vec::is_empty", default)]
    changes: Vec<PyRevisionChange>,
}

/// Python projection of the full TEI document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PyTeiDocument {
    pub(crate) header: PyTeiHeader,
    pub(crate) text: PyTeiText,
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

impl From<PyTeiHeader> for TeiHeader {
    fn from(value: PyTeiHeader) -> Self {
        let mut header = Self::new(value.file_desc);
        if let Some(profile) = value.profile_desc {
            header = header.with_profile_desc(ProfileDesc::from(profile));
        }
        if let Some(encoding) = value.encoding_desc {
            header = header.with_encoding_desc(EncodingDesc::from(encoding));
        }
        if let Some(revision) = value.revision_desc {
            header = header.with_revision_desc(RevisionDesc::from(revision));
        }
        header
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

impl From<PyProfileDesc> for ProfileDesc {
    fn from(value: PyProfileDesc) -> Self {
        let mut profile = Self::new();
        if let Some(synopsis) = value.synopsis {
            profile = profile.with_synopsis(synopsis);
        }
        for speaker in value.speakers {
            if let Err(error) = profile.add_speaker(speaker.as_str()) {
                panic!("stored speaker should be valid: {error}");
            }
        }
        for language in value.languages {
            if let Err(error) = profile.add_language(language.as_str()) {
                panic!("stored language should be valid: {error}");
            }
        }
        profile
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

impl From<PyAnnotationSystem> for AnnotationSystem {
    fn from(system: PyAnnotationSystem) -> Self {
        let description = system.desc.unwrap_or_default();
        Self::new(system.xml_id.as_str(), description)
            .unwrap_or_else(|error| panic!("stored annotation system should be valid: {error}"))
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
        }
    }
}

impl From<PyEncodingDesc> for EncodingDesc {
    fn from(value: PyEncodingDesc) -> Self {
        let mut encoding = Self::new();
        for system in value.annotation_systems {
            encoding.add_annotation_system(AnnotationSystem::from(system));
        }
        encoding
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

impl From<PyRevisionChange> for RevisionChange {
    fn from(change: PyRevisionChange) -> Self {
        Self::new(change.desc, change.resp.unwrap_or_default())
            .unwrap_or_else(|error| panic!("stored revision change should be valid: {error}"))
    }
}

impl From<&RevisionDesc> for PyRevisionDesc {
    fn from(desc: &RevisionDesc) -> Self {
        Self {
            changes: desc.iter().map(PyRevisionChange::from).collect(),
        }
    }
}

impl From<PyRevisionDesc> for RevisionDesc {
    fn from(desc: PyRevisionDesc) -> Self {
        let mut revision = Self::new();
        for change in desc.changes {
            revision.add_change(RevisionChange::from(change));
        }
        revision
    }
}

impl From<&TeiBody> for PyTeiBody {
    fn from(body: &TeiBody) -> Self {
        let blocks = body.blocks().iter().map(py_body_block_from_core).collect();
        Self { blocks }
    }
}

impl TryFrom<PyTeiBody> for TeiBody {
    type Error = BodyContentError;

    fn try_from(value: PyTeiBody) -> Result<Self, Self::Error> {
        let mut body = Self::default();
        for block in value.blocks {
            body.extend([core_block_from_py(block)?]);
        }
        Ok(body)
    }
}

impl From<&TeiDocument> for PyTeiDocument {
    fn from(document: &TeiDocument) -> Self {
        Self {
            header: PyTeiHeader::from(document.header()),
            text: PyTeiText {
                body: PyTeiBody::from(document.text().body()),
            },
        }
    }
}

impl TryFrom<PyTeiDocument> for TeiDocument {
    type Error = BodyContentError;

    fn try_from(value: PyTeiDocument) -> Result<Self, Self::Error> {
        let header: TeiHeader = value.header.into();
        let body = TeiBody::try_from(value.text.body)?;
        Ok(Self::new(header, TeiText::new(body)))
    }
}

impl From<Inline> for PyInline {
    fn from(value: Inline) -> Self {
        match value {
            Inline::Text(text) => Self::Text { value: text },
            Inline::Hi(hi) => Self::Hi {
                rend: hi.rend().map(str::to_owned),
                content: hi.content().iter().cloned().map(Self::from).collect(),
            },
            Inline::Pause(pause) => Self::Pause {
                dur: pause.duration().map(str::to_owned),
                kind: pause.kind().map(str::to_owned),
            },
        }
    }
}

fn inline_from_py(inline_value: PyInline) -> Result<Inline, BodyContentError> {
    match inline_value {
        PyInline::Text { value } => Ok(Inline::Text(value)),
        PyInline::Hi { rend, content } => {
            let converted_values: Result<Vec<Inline>, BodyContentError> =
                content.into_iter().map(inline_from_py).collect();
            let converted_inlines = converted_values?;
            let hi = match rend {
                Some(r) => tei_core::Hi::try_with_rend(r, converted_inlines)?,
                None => tei_core::Hi::try_new(converted_inlines)?,
            };
            Ok(Inline::Hi(hi))
        }
        PyInline::Pause { dur, kind } => {
            let mut pause = Pause::new();
            if let Some(duration) = dur {
                pause.set_duration(duration);
            }
            if let Some(classification) = kind {
                pause.set_kind(classification);
            }
            Ok(Inline::Pause(pause))
        }
    }
}

fn py_body_block_from_core(block: &BodyBlock) -> PyBodyBlock {
    match block {
        BodyBlock::Paragraph(p) => PyBodyBlock::Paragraph {
            xml_id: p.id().map(|id| id.as_str().to_owned()),
            content: p.content().iter().cloned().map(PyInline::from).collect(),
        },
        BodyBlock::Utterance(u) => PyBodyBlock::Utterance {
            xml_id: u.id().map(|id| id.as_str().to_owned()),
            speaker: u.speaker().map(|s| s.as_str().to_owned()),
            content: u.content().iter().cloned().map(PyInline::from).collect(),
        },
    }
}

fn core_block_from_py(block: PyBodyBlock) -> Result<BodyBlock, BodyContentError> {
    match block {
        PyBodyBlock::Paragraph { xml_id, content } => {
            let mut paragraph = P::from_inline(
                content
                    .into_iter()
                    .map(inline_from_py)
                    .collect::<Result<Vec<_>, _>>()?,
            )?;
            if let Some(id) = xml_id {
                paragraph.set_id(id)?;
            }
            Ok(BodyBlock::Paragraph(paragraph))
        }
        PyBodyBlock::Utterance {
            xml_id,
            speaker,
            content,
        } => {
            let mut utterance = Utterance::from_inline(
                speaker.as_deref(),
                content
                    .into_iter()
                    .map(inline_from_py)
                    .collect::<Result<Vec<_>, _>>()?,
            )?;
            if let Some(id) = xml_id {
                utterance.set_id(id)?;
            }
            Ok(BodyBlock::Utterance(utterance))
        }
    }
}

/// Converts a core TEI document into a projection `Value` for tests.
///
/// # Errors
///
/// Returns a JSON serialisation error when the projection cannot be rendered.
pub fn document_to_value(document: &TeiDocument) -> Result<Value, tei_serde::serde_json::Error> {
    let projection = PyTeiDocument::from(document);
    tei_serde::json::to_value(&projection)
}

/// Converts a projection `Value` into a core document for tests.
///
/// # Errors
///
/// Returns a JSON deserialisation error when the payload is not a valid
/// projection or when conversion back to the core TEI model fails.
pub fn value_to_document(value: &Value) -> Result<TeiDocument, tei_serde::serde_json::Error> {
    let projection: PyTeiDocument =
        tei_serde::json::from_value(value.clone()).map_err(|error| {
            tei_serde::serde_json::Error::custom(format!("invalid TEI projection: {error}"))
        })?;
    TeiDocument::try_from(projection)
        .map_err(|error| tei_serde::serde_json::Error::custom(format!("invalid TEI body: {error}")))
}

/// Maps a core streaming event to its Python projection.
pub(crate) fn py_event_from_core(event: tei_xml::streaming::TeiEvent) -> PyEvent {
    match event {
        tei_xml::streaming::TeiEvent::DocumentStart => PyEvent::DocumentStart,
        tei_xml::streaming::TeiEvent::Header(header) => PyEvent::Header {
            header: PyTeiHeader::from(&header),
        },
        tei_xml::streaming::TeiEvent::BodyBlock(block) => match block {
            BodyBlock::Paragraph(p) => PyEvent::Paragraph {
                xml_id: p.id().map(|id| id.as_str().to_owned()),
                content: p.content().iter().cloned().map(PyInline::from).collect(),
            },
            BodyBlock::Utterance(u) => PyEvent::Utterance {
                xml_id: u.id().map(|id| id.as_str().to_owned()),
                speaker: u.speaker().map(|s| s.as_str().to_owned()),
                content: u.content().iter().cloned().map(PyInline::from).collect(),
            },
        },
        tei_xml::streaming::TeiEvent::DocumentEnd => PyEvent::DocumentEnd,
    }
}
