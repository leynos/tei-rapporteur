//! TEI document fixtures for validation testing and benchmarking.
//!
//! This module provides builders for generating canonical TEI documents that
//! exercise the TEI Episodic Profile schema. The fixtures are used by the
//! `generate-fixtures` binary to produce XML files for external validation
//! against the Relax NG schema using tools like `jing`.
//!
//! The module also provides scalable benchmark fixture generation via
//! [`BenchFixtureConfig`] and [`generate_benchmark_document`], enabling
//! performance comparisons between the full-document and streaming parsers.
//!
//! Note: Due to quick-xml serialization limitations with mixed content in
//! `$value` fields, fixtures avoid inline elements (`<hi>`, `<pause>`) which
//! cannot round-trip through serialization. The fixtures focus on structural
//! elements (paragraphs, utterances, header metadata) that can be serialized.

mod bench;

pub use bench::{BenchFixtureConfig, generate_benchmark_document, generate_benchmark_xml};

use tei_core::{
    AnnotationSystem, BodyBlock, CiteData, CiteStructure, Div, EncodingDesc, FileDesc, Head, Item,
    Label, List, P, PointerList, ProfileDesc, RefsDecl, RevisionChange, RevisionDesc, Span,
    SpanGroup, StandOff, TeiBody, TeiDocument, TeiError, TeiHeader, TeiText, Utterance,
};

/// A function that builds a TEI document fixture.
pub type FixtureBuilder = fn() -> Result<TeiDocument, TeiError>;

/// A named fixture builder pairing a name with its builder function.
pub type NamedFixture = (&'static str, FixtureBuilder);

/// Returns a minimal TEI document with only a title and empty body.
///
/// # Errors
///
/// Returns [`TeiError`] when the document cannot be constructed.
pub fn minimal_document() -> Result<TeiDocument, TeiError> {
    TeiDocument::from_title_str("Minimal Fixture")
}

/// Returns a document with paragraphs in the body.
///
/// # Errors
///
/// Returns [`TeiError`] when the document or paragraphs cannot be constructed.
pub fn document_with_paragraphs() -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Paragraph Fixture")?;
    let header = TeiHeader::new(file_desc);

    let mut para1 = P::from_text_segments(["Opening narration sets the scene."])?;
    para1.set_id("p1")?;

    let mut para2 = P::from_text_segments(["A second paragraph continues the story."])?;
    para2.set_id("p2")?;

    let body = TeiBody::new([BodyBlock::Paragraph(para1), BodyBlock::Paragraph(para2)]);
    let text = TeiText::new(body);

    Ok(TeiDocument::new(header, text))
}

/// Returns a document with utterances and speaker references.
///
/// # Errors
///
/// Returns [`TeiError`] when the document, profile, or utterances cannot be
/// constructed.
pub fn document_with_utterances() -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Utterance Fixture")?;

    // Add speakers to the profile
    let mut profile = ProfileDesc::new();
    profile.add_speaker("host")?;
    profile.add_speaker("guest")?;

    let header = TeiHeader::new(file_desc).with_profile_desc(profile);

    // Add utterances with speaker references
    let mut u1 = Utterance::from_text_segments(Some("host"), ["Welcome to the show."])?;
    u1.set_id("u1")?;

    let mut u2 = Utterance::from_text_segments(Some("guest"), ["Thanks for having me."])?;
    u2.set_id("u2")?;

    let body = TeiBody::new([BodyBlock::Utterance(u1), BodyBlock::Utterance(u2)]);
    let text = TeiText::new(body);

    Ok(TeiDocument::new(header, text))
}

/// Returns a comprehensive document exercising all serializable profile
/// features.
///
/// This fixture includes header metadata (profile, encoding, revision) and
/// body content (paragraphs and utterances with speaker references). It
/// omits inline elements (`<hi>`, `<pause>`) due to quick-xml serialization
/// limitations.
///
/// # Errors
///
/// Returns [`TeiError`] when any component of the document cannot be
/// constructed.
pub fn comprehensive_document() -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Comprehensive Fixture")?;

    // Build profile with speakers and language
    let mut profile = ProfileDesc::new()
        .with_synopsis("A comprehensive test episode covering all profile features.");
    profile.add_speaker("host")?;
    profile.add_speaker("guest")?;
    profile.add_language("en-GB")?;

    // Build encoding description with annotation system
    let mut encoding = EncodingDesc::new();
    let annotation = AnnotationSystem::new("cliche", "Cliche detection annotations")?;
    encoding.add_annotation_system(annotation);
    let mut refs_decl = RefsDecl::new();
    let mut cite_structure = CiteStructure::new("//u[@xml:id]");
    cite_structure = cite_structure.with_unit("utterance");
    cite_structure.add_cite_data(CiteData::new("speaker").with_use_expr("@who"));
    refs_decl.add_cite_structure(cite_structure);
    encoding.set_refs_decl(refs_decl);

    // Build revision description
    let mut revision = RevisionDesc::new();
    let change = RevisionChange::new("Initial creation", "editor")?;
    revision.add_change(change);

    // Assemble header
    let header = TeiHeader::new(file_desc)
        .with_profile_desc(profile)
        .with_encoding_desc(encoding)
        .with_revision_desc(revision);

    // Build body content using text-only segments (no inline elements)
    let mut intro = P::from_text_segments(["Episode introduction sets the stage."])?;
    intro.set_id("p1")?;

    let mut u1 = Utterance::from_text_segments(Some("host"), ["Hello and welcome!"])?;
    u1.set_id("u1")?;

    let mut u2 = Utterance::from_text_segments(Some("guest"), ["Great to be here."])?;
    u2.set_id("u2")?;

    let mut u3 = Utterance::from_text_segments(Some("host"), ["Let me think about that."])?;
    u3.set_id("u3")?;

    let body = TeiBody::new([
        BodyBlock::Paragraph(intro),
        BodyBlock::Utterance(u1),
        BodyBlock::Utterance(u2),
        BodyBlock::Utterance(u3),
    ]);
    let text = TeiText::new(body);
    let mut span = Span::new();
    span.set_id("sp1")?;
    span.set_target(PointerList::new(["#u1"])?);
    let mut span_group = SpanGroup::new("citation")?;
    span_group.set_id("sg1")?;
    span_group.set_resp(PointerList::new(["#cliche"])?);
    span_group.add_span(span);
    let mut stand_off = StandOff::new();
    stand_off.add_span_group(span_group);

    Ok(TeiDocument::new(header, text).with_stand_off(stand_off))
}

/// Returns a document whose body contains a structural division with a list.
///
/// # Errors
///
/// Returns [`TeiError`] when any component of the document cannot be
/// constructed.
pub fn document_with_div_and_list() -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Division Fixture")?;
    let header = TeiHeader::new(file_desc);

    let mut intro = P::from_text_segments(["Show notes follow below."])?;
    intro.set_id("p-div-intro")?;

    let mut utterance =
        Utterance::from_text_segments(Some("host"), ["These links are in the notes."])?;
    utterance.set_id("u-div-1")?;

    let label = Label::from_text("1.")?;
    let mut first_item = Item::from_text_segments(["Primary reference"])?;
    first_item.set_id("item-1")?;
    first_item.set_n("1")?;
    first_item.set_corresp(PointerList::new(["#u-div-1"])?);
    first_item.set_label(label);

    let mut second_item = Item::from_text_segments(["Secondary reference"])?;
    second_item.set_id("item-2")?;

    let mut list = List::new([first_item, second_item])?;
    list.set_id("list-1")?;

    let mut div = Div::new("show-notes")?;
    div.set_id("div-1")?;
    div.push_paragraph(intro);
    div.push_utterance(utterance);
    div.push_list(list);

    let body = TeiBody::new([BodyBlock::Div(div)]);
    let text = TeiText::new(body);
    Ok(TeiDocument::new(header, text))
}

/// Returns a document with nested divisions, headings, and subtypes.
///
/// # Errors
///
/// Returns [`TeiError`] when any component of the document cannot be
/// constructed.
pub fn document_with_nested_divs() -> Result<TeiDocument, TeiError> {
    let file_desc = FileDesc::from_title_str("Nested Division Fixture")?;
    let header = TeiHeader::new(file_desc);

    let mut child = Div::new("segment")?;
    child.set_id("ch1")?;
    child.set_subtype("chapter-marker")?;
    child.set_head(Head::from_text("Cold open")?);
    child.push_utterance(Utterance::from_text_segments(
        Some("host"),
        ["Welcome back."],
    )?);

    let mut parent = Div::new("segment")?;
    parent.set_id("seg1")?;
    parent.set_subtype("chapter-markers")?;
    parent.set_head(Head::from_text("Chapter markers")?);
    parent.push_div(child);

    let body = TeiBody::new([BodyBlock::Div(parent)]);
    let text = TeiText::new(body);
    Ok(TeiDocument::new(header, text))
}

/// Returns an iterator over all fixture builders with their names.
///
/// This is used by the `generate-fixtures` binary to produce XML files.
#[must_use]
pub fn fixture_builders() -> Vec<NamedFixture> {
    vec![
        ("minimal", minimal_document),
        ("paragraphs", document_with_paragraphs),
        ("utterances", document_with_utterances),
        ("comprehensive", comprehensive_document),
        ("div-list", document_with_div_and_list),
        ("nested-div", document_with_nested_divs),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_document_builds_successfully() {
        let doc = minimal_document().expect("minimal document should build");
        assert_eq!(doc.title().as_str(), "Minimal Fixture");
    }

    #[test]
    fn document_with_paragraphs_builds_successfully() {
        let doc = document_with_paragraphs().expect("paragraph document should build");
        assert_eq!(doc.text().body().blocks().len(), 2);
    }

    #[test]
    fn document_with_utterances_builds_successfully() {
        let doc = document_with_utterances().expect("utterance document should build");
        assert_eq!(doc.text().body().blocks().len(), 2);
        assert!(doc.header().profile_desc().is_some());
    }

    #[test]
    fn comprehensive_document_builds_successfully() {
        let doc = comprehensive_document().expect("comprehensive document should build");
        assert!(doc.header().profile_desc().is_some());
        assert!(doc.header().encoding_desc().is_some());
        assert!(doc.header().revision_desc().is_some());
        assert!(doc.stand_off().is_some());
        assert_eq!(doc.text().body().blocks().len(), 4);
    }

    #[test]
    fn document_with_div_and_list_builds_successfully() {
        let doc = document_with_div_and_list().expect("division fixture should build");
        assert_eq!(doc.text().body().blocks().len(), 1);
    }

    #[test]
    fn document_with_nested_divs_builds_successfully() {
        let doc = document_with_nested_divs().expect("nested division fixture should build");
        assert_eq!(doc.text().body().blocks().len(), 1);
    }

    #[test]
    fn all_fixtures_pass_validation() {
        for (name, builder) in fixture_builders() {
            let doc = builder().unwrap_or_else(|error| {
                panic!("{name} fixture should build: {error}");
            });
            doc.validate().unwrap_or_else(|error| {
                panic!("{name} fixture should validate: {error}");
            });
        }
    }
}
