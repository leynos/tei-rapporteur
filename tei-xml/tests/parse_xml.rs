//! Behaviour-driven scenarios that cover parsing TEI XML strings into
//! structured documents.

use anyhow::{Context, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{BodyBlock, DivContent, TeiDocument, TeiError};
use tei_test_helpers::expect_validated_state;
use tei_xml::parse_xml;

// Force Cargo to recompile the test binary when the feature file changes so the
// embedded scenarios stay in sync with expectations.
const _: &str = include_str!("features/parse_xml.feature");

const MINIMAL_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Wolf 359</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body/>",
    "</text>",
    "</TEI>",
);

const MISSING_HEADER_FIXTURE: &str = concat!("<TEI>", "<text>", "<body/>", "</text>", "</TEI>",);

const UNTERMINATED_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Broken</title>",
    "</fileDesc>",
);

const BLANK_TITLE_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>   </title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body/>",
    "</text>",
    "</TEI>",
);

const ANNOTATED_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Annotated</title>",
    "</fileDesc>",
    "<encodingDesc>",
    "<refsDecl>",
    "<citeStructure match=\"//u[@xml:id]\" unit=\"utterance\">",
    "<citeData property=\"speaker\" use=\"@who\"/>",
    "</citeStructure>",
    "</refsDecl>",
    "</encodingDesc>",
    "</teiHeader>",
    "<standOff>",
    "<spanGrp xml:id=\"sg1\" type=\"citation\" resp=\"#ann1\">",
    "<span xml:id=\"sp1\" target=\"#u1\" cert=\"high\"/>",
    "</spanGrp>",
    "</standOff>",
    "<text>",
    "<body>",
    "<u xml:id=\"u1\" who=\"host\">Hello</u>",
    "</body>",
    "</text>",
    "</TEI>",
);

const NESTED_DIV_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Nested</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"segment\" subtype=\"chapter-markers\" xml:id=\"seg1\">",
    "<head>Chapter markers</head>",
    "<div type=\"segment\" subtype=\"chapter-marker\" xml:id=\"ch1\">",
    "<head>Cold open</head>",
    "<u who=\"host\">Welcome back.</u>",
    "</div>",
    "</div>",
    "</body>",
    "</text>",
    "</TEI>",
);

type DocumentResult = std::result::Result<TeiDocument, TeiError>;

#[derive(Default)]
struct ParseState {
    xml: RefCell<Option<String>>,
    result: RefCell<Option<DocumentResult>>,
}

impl ParseState {
    fn set_xml(&self, xml: &str) {
        *self.xml.borrow_mut() = Some(xml.to_owned());
    }

    fn xml(&self) -> anyhow::Result<String> {
        self.xml
            .borrow()
            .as_ref()
            .cloned()
            .context("scenario must supply XML input")
    }

    fn set_result(&self, result: DocumentResult) {
        *self.result.borrow_mut() = Some(result);
    }

    fn result(&self) -> anyhow::Result<DocumentResult> {
        self.result
            .borrow()
            .as_ref()
            .cloned()
            .context("parse_xml must run before assertions")
    }
}

fn fixture_by_name(name: &str) -> anyhow::Result<&'static str> {
    match name {
        "minimal" => Ok(MINIMAL_FIXTURE),
        "missing-header" => Ok(MISSING_HEADER_FIXTURE),
        "unterminated" => Ok(UNTERMINATED_FIXTURE),
        "blank-title" => Ok(BLANK_TITLE_FIXTURE),
        "annotated" => Ok(ANNOTATED_FIXTURE),
        "nested-div" => Ok(NESTED_DIV_FIXTURE),
        other => bail!("unknown TEI fixture: {other}"),
    }
}

#[fixture]
fn validated_state_result() -> anyhow::Result<ParseState> {
    let state = ParseState::default();
    ensure!(state.xml.borrow().is_none(), "xml slot must start empty");
    ensure!(
        state.result.borrow().is_none(),
        "result slot must start empty"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> ParseState {
    match validated_state_result() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialize parse state: {error}"),
    }
}

// rstest-bdd supplies owned `String` values for placeholders, so keep the
// signature by value.
#[given("the TEI fixture \"{fixture}\"")]
fn the_tei_fixture(
    #[from(validated_state)] state: &ParseState,
    fixture: String,
) -> anyhow::Result<()> {
    let xml = fixture_by_name(&fixture)?;
    state.set_xml(xml);
    let _ = state.xml()?;
    Ok(())
}

#[when("I parse the TEI input")]
fn i_parse_the_input(#[from(validated_state)] state: &ParseState) -> anyhow::Result<()> {
    let xml = state.xml()?;
    let result = parse_xml(&xml);
    state.set_result(result);
    Ok(())
}

#[then("parsing succeeds")]
fn parsing_succeeds(#[from(validated_state)] state: &ParseState) -> anyhow::Result<()> {
    let result = state.result()?;
    result.context("expected parsing to succeed")?;
    Ok(())
}

// rstest-bdd supplies owned `String` values for placeholders, so keep the
// signature by value.
#[then("the parsed title is \"{title}\"")]
fn parsed_title_is(
    #[from(validated_state)] state: &ParseState,
    title: String,
) -> anyhow::Result<()> {
    let document = state
        .result()?
        .context("expected successful parse before asserting title")?;
    ensure!(
        document.title().as_str() == title,
        "title mismatch: expected {title:?}, found {:?}",
        document.title().as_str()
    );
    Ok(())
}

// rstest-bdd supplies owned `String` values for placeholders, so keep the
// signature by value.
#[then("parsing fails mentioning \"{snippet}\"")]
fn parsing_fails_with_snippet(
    #[from(validated_state)] state: &ParseState,
    snippet: String,
) -> anyhow::Result<()> {
    let outcome = state.result()?;
    let Err(error) = outcome else {
        bail!("expected parsing to fail");
    };
    let message = error.to_string();
    ensure!(
        message.contains(&snippet),
        "error should mention {snippet:?}, found {message:?}"
    );
    Ok(())
}

#[then("the parsed document includes stand-off annotations and citation declarations")]
fn parsed_document_includes_richer_tei(
    #[from(validated_state)] state: &ParseState,
) -> anyhow::Result<()> {
    let document = state
        .result()?
        .context("expected successful parse before asserting TEI richness")?;
    let stand_off = document
        .stand_off()
        .context("expected standOff section to be present")?;
    ensure!(
        stand_off.span_groups().len() == 1,
        "expected one span group, found {}",
        stand_off.span_groups().len()
    );

    let refs_decl = document
        .header()
        .encoding_desc()
        .and_then(|encoding| encoding.refs_decl())
        .context("expected refsDecl to be present")?;
    ensure!(
        refs_decl.cite_structures().len() == 1,
        "expected one citeStructure, found {}",
        refs_decl.cite_structures().len()
    );
    Ok(())
}

#[then("the parsed document includes nested divisions with headings and subtypes")]
fn parsed_document_includes_nested_divisions(
    #[from(validated_state)] state: &ParseState,
) -> anyhow::Result<()> {
    let document = state
        .result()?
        .context("expected successful parse before asserting nested divisions")?;
    let Some(BodyBlock::Div(div)) = document.text().body().blocks().first() else {
        bail!("expected a top-level div body block");
    };
    ensure!(
        div.subtype() == Some("chapter-markers"),
        "expected top-level div subtype to be chapter-markers"
    );
    ensure!(
        div.head().is_some_and(|head| head.content().len() == 1),
        "expected top-level div head content"
    );
    let Some(DivContent::Div(child)) = div.content().first() else {
        bail!("expected nested div as first child");
    };
    ensure!(
        child.subtype() == Some("chapter-marker"),
        "expected nested div subtype to be chapter-marker"
    );
    ensure!(
        child.head().is_some_and(|head| head.content().len() == 1),
        "expected nested div head content"
    );
    Ok(())
}

#[scenario(path = "tests/features/parse_xml.feature", index = 0)]
fn parses_valid_documents(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: anyhow::Result<ParseState>,
) {
    expect_validated_state(result, "parse");
}

#[scenario(path = "tests/features/parse_xml.feature", index = 1)]
fn reports_missing_headers(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: anyhow::Result<ParseState>,
) {
    expect_validated_state(result, "parse");
}

#[scenario(path = "tests/features/parse_xml.feature", index = 2)]
fn reports_malformed_xml(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: anyhow::Result<ParseState>,
) {
    expect_validated_state(result, "parse");
}

#[scenario(path = "tests/features/parse_xml.feature", index = 5)]
fn parses_nested_divisions(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: anyhow::Result<ParseState>,
) {
    expect_validated_state(result, "parse");
}

#[scenario(path = "tests/features/parse_xml.feature", index = 3)]
fn rejects_blank_titles(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: anyhow::Result<ParseState>,
) {
    expect_validated_state(result, "parse");
}

#[scenario(path = "tests/features/parse_xml.feature", index = 4)]
fn parses_annotated_documents(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: anyhow::Result<ParseState>,
) {
    expect_validated_state(result, "parse");
}
