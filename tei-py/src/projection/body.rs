//! Conversion functions between core TEI body elements and their Python
//! projection counterparts.
//!
//! Shared helpers eliminate duplication where the same core type (`P`,
//! `Utterance`) appears in both [`BodyBlock`] and [`DivContent`] contexts.

use tei_core::{
    BodyBlock, Div, DivContent, Inline, Item, Label, List, P, PointerList, TeiError, Utterance,
};

use super::{
    PyBodyBlock, PyDivContent, PyInline, PyItem, PyLabel, apply_optional_pointer_list,
    certainty_from_option, certainty_to_string, pointer_list_to_vec,
};

// ---------------------------------------------------------------------------
// Core → Py helpers
// ---------------------------------------------------------------------------

/// Projects a core paragraph into shared `(xml_id, content)` fields.
fn project_paragraph(p: &P) -> (Option<String>, Vec<PyInline>) {
    (
        p.id().map(|id| id.as_str().to_owned()),
        p.content().iter().cloned().map(PyInline::from).collect(),
    )
}

/// Intermediate representation of an utterance's Python-facing fields,
/// used to construct both [`PyBodyBlock::Utterance`] and
/// [`PyDivContent::Utterance`] without repeating the projection logic.
struct PyUtteranceFields {
    xml_id: Option<String>,
    n: Option<String>,
    speaker: Option<String>,
    source: Vec<String>,
    resp: Vec<String>,
    cert: Option<String>,
    corresp: Vec<String>,
    ana: Vec<String>,
    content: Vec<PyInline>,
}

/// Projects a core utterance into shared fields.
fn project_utterance(u: &Utterance) -> PyUtteranceFields {
    PyUtteranceFields {
        xml_id: u.id().map(|id| id.as_str().to_owned()),
        n: u.number().map(str::to_owned),
        speaker: u.speaker().map(|s| s.as_str().to_owned()),
        source: pointer_list_to_vec(u.source()),
        resp: pointer_list_to_vec(u.resp()),
        cert: certainty_to_string(u.cert()),
        corresp: pointer_list_to_vec(u.corresp()),
        ana: pointer_list_to_vec(u.ana()),
        content: u.content().iter().cloned().map(PyInline::from).collect(),
    }
}

/// Projects a core body block into its Python counterpart.
pub(crate) fn py_body_block_from_core(block: &BodyBlock) -> PyBodyBlock {
    match block {
        BodyBlock::Paragraph(p) => {
            let (xml_id, content) = project_paragraph(p);
            PyBodyBlock::Paragraph { xml_id, content }
        }
        BodyBlock::Utterance(u) => py_utterance_body_block(u),
        BodyBlock::Div(div) => PyBodyBlock::Div {
            xml_id: div.id().map(|id| id.as_str().to_owned()),
            div_type: div.div_type().to_owned(),
            content: div.content().iter().map(py_div_content_from_core).collect(),
        },
    }
}

/// Projects a core utterance into a [`PyBodyBlock::Utterance`].
pub(crate) fn py_utterance_body_block(u: &Utterance) -> PyBodyBlock {
    let f = project_utterance(u);
    PyBodyBlock::Utterance {
        xml_id: f.xml_id,
        n: f.n,
        speaker: f.speaker,
        source: f.source,
        resp: f.resp,
        cert: f.cert,
        corresp: f.corresp,
        ana: f.ana,
        content: f.content,
    }
}

/// Projects a core division content element into its Python counterpart.
pub(crate) fn py_div_content_from_core(div_content: &DivContent) -> PyDivContent {
    match div_content {
        DivContent::Paragraph(p) => {
            let (xml_id, content) = project_paragraph(p);
            PyDivContent::Paragraph { xml_id, content }
        }
        DivContent::Utterance(u) => {
            let f = project_utterance(u);
            PyDivContent::Utterance {
                xml_id: f.xml_id,
                n: f.n,
                speaker: f.speaker,
                source: f.source,
                resp: f.resp,
                cert: f.cert,
                corresp: f.corresp,
                ana: f.ana,
                content: f.content,
            }
        }
        DivContent::List(list) => PyDivContent::List {
            xml_id: list.id().map(|id| id.as_str().to_owned()),
            items: list.items().iter().map(py_item_from_core).collect(),
        },
    }
}

fn py_item_from_core(item: &Item) -> PyItem {
    PyItem {
        xml_id: item.id().map(|id| id.as_str().to_owned()),
        n: item.n().map(str::to_owned),
        corresp: pointer_list_to_vec(item.corresp()),
        label: item.label().map(py_label_from_core),
        content: item.content().iter().cloned().map(PyInline::from).collect(),
    }
}

fn py_label_from_core(label: &Label) -> PyLabel {
    PyLabel {
        content: label
            .content()
            .iter()
            .cloned()
            .map(PyInline::from)
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Py → Core helpers
// ---------------------------------------------------------------------------

/// Converts a vector of Python inline nodes to core inline nodes.
fn inlines_from_py(content: Vec<PyInline>) -> Result<Vec<Inline>, TeiError> {
    content
        .into_iter()
        .map(super::inline_from_py)
        .collect::<Result<Vec<_>, _>>()
}

/// Builds a core paragraph from Python projection fields.
fn build_paragraph(xml_id: Option<String>, content: Vec<PyInline>) -> Result<P, TeiError> {
    let mut paragraph = P::from_inline(inlines_from_py(content)?)?;
    if let Some(id) = xml_id {
        paragraph.set_id(id)?;
    }
    Ok(paragraph)
}

/// All attributes and content of a Python-side utterance, bundled to avoid an
/// excess-arguments signature.
pub(crate) struct UtteranceArgs {
    pub(crate) xml_id: Option<String>,
    pub(crate) n: Option<String>,
    pub(crate) speaker: Option<String>,
    pub(crate) source: Vec<String>,
    pub(crate) resp: Vec<String>,
    pub(crate) cert: Option<String>,
    pub(crate) corresp: Vec<String>,
    pub(crate) ana: Vec<String>,
    pub(crate) content: Vec<PyInline>,
}

/// Builds a core utterance from Python projection fields.
fn build_utterance(args: UtteranceArgs) -> Result<Utterance, TeiError> {
    let mut utterance =
        Utterance::from_inline(args.speaker.as_deref(), inlines_from_py(args.content)?)?;
    if let Some(id) = args.xml_id {
        utterance.set_id(id)?;
    }
    if let Some(number) = args.n {
        utterance.set_number(number);
    }
    apply_optional_pointer_list(&mut utterance, args.source, Utterance::set_source)?;
    apply_optional_pointer_list(&mut utterance, args.resp, Utterance::set_resp)?;
    if let Some(certainty) = certainty_from_option(args.cert)? {
        utterance.set_cert(certainty);
    }
    apply_optional_pointer_list(&mut utterance, args.corresp, Utterance::set_corresp)?;
    apply_optional_pointer_list(&mut utterance, args.ana, Utterance::set_ana)?;
    Ok(utterance)
}

/// Reconstructs a core body block from its Python projection.
pub(crate) fn core_block_from_py(block: PyBodyBlock) -> Result<BodyBlock, TeiError> {
    match block {
        PyBodyBlock::Paragraph { xml_id, content } => {
            Ok(BodyBlock::Paragraph(build_paragraph(xml_id, content)?))
        }
        PyBodyBlock::Utterance {
            xml_id,
            n,
            speaker,
            source,
            resp,
            cert,
            corresp,
            ana,
            content,
        } => Ok(BodyBlock::Utterance(build_utterance(UtteranceArgs {
            xml_id,
            n,
            speaker,
            source,
            resp,
            cert,
            corresp,
            ana,
            content,
        })?)),
        PyBodyBlock::Div {
            xml_id,
            div_type,
            content,
        } => div_block_from_py(xml_id, div_type, content),
    }
}

fn div_block_from_py(
    xml_id: Option<String>,
    div_type: String,
    content: Vec<PyDivContent>,
) -> Result<BodyBlock, TeiError> {
    let mut div = Div::new(div_type)?;
    if let Some(id) = xml_id {
        div.set_id(id)?;
    }
    for py_content in content {
        match core_div_content_from_py(py_content)? {
            DivContent::Paragraph(p) => div.push_paragraph(p),
            DivContent::Utterance(u) => div.push_utterance(u),
            DivContent::List(l) => div.push_list(l),
        }
    }
    Ok(BodyBlock::Div(div))
}

fn core_div_content_from_py(py_content: PyDivContent) -> Result<DivContent, TeiError> {
    match py_content {
        PyDivContent::Paragraph { xml_id, content } => {
            Ok(DivContent::Paragraph(build_paragraph(xml_id, content)?))
        }
        PyDivContent::Utterance {
            xml_id,
            n,
            speaker,
            source,
            resp,
            cert,
            corresp,
            ana,
            content,
        } => Ok(DivContent::Utterance(build_utterance(UtteranceArgs {
            xml_id,
            n,
            speaker,
            source,
            resp,
            cert,
            corresp,
            ana,
            content,
        })?)),
        PyDivContent::List { xml_id, items } => {
            let core_items: Vec<Item> = items
                .into_iter()
                .map(core_item_from_py)
                .collect::<Result<Vec<_>, _>>()?;
            let mut list = List::new(core_items)?;
            if let Some(id) = xml_id {
                list.set_id(id)?;
            }
            Ok(DivContent::List(list))
        }
    }
}

fn core_item_from_py(item: PyItem) -> Result<Item, TeiError> {
    let mut core_item = Item::new(inlines_from_py(item.content)?)?;
    if let Some(id) = item.xml_id {
        core_item.set_id(id)?;
    }
    if let Some(n_value) = item.n {
        core_item.set_n(n_value)?;
    }
    if !item.corresp.is_empty() {
        core_item.set_corresp(PointerList::new(item.corresp)?);
    }
    if let Some(py_label) = item.label {
        core_item.set_label(Label::new(inlines_from_py(py_label.content)?)?);
    }
    Ok(core_item)
}
