//! Behaviour-driven parsing coverage for guest-biography body enrichment.

use anyhow::{Context, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{BodyBlock, DivContent, Inline, PointerList, TeiDocument, TeiError};
use tei_test_helpers::expect_validated_state;
use tei_xml::{emit_xml, parse_xml};

// Force Cargo to recompile the test binary when the feature file changes so the
// embedded scenario stays in sync with expectations.
const _: &str = include_str!("features/parse_xml.feature");

const GUEST_BIOS_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Guest Bios</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"guest-bios\" xml:id=\"guest-bios\">",
    "<list xml:id=\"guest-bio-list\">",
    "<item xml:id=\"guest-bio-ada\" ",
    "corresp=\"urn:episodic:reference-document-revision:019e1368\">",
    "<label>Ada Lovelace</label>",
    "Mathematician and computing pioneer.",
    "</item>",
    "</list>",
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
        Err(error) => panic!("failed to initialise guest-bios parse state: {error}"),
    }
}

// rstest-bdd supplies owned `String` values for placeholders, so keep the
// signature by value.
#[given("the TEI fixture \"{fixture}\"")]
fn the_tei_fixture(
    #[from(validated_state)] state: &ParseState,
    fixture: String,
) -> anyhow::Result<()> {
    let "guest-bios" = fixture.as_str() else {
        bail!("unknown guest-bios parsing fixture: {fixture}");
    };
    state.set_xml(GUEST_BIOS_FIXTURE);
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
    result.context("expected guest-bios parsing to succeed")?;
    Ok(())
}

#[then("the parsed document includes guest bios linked to an external reference revision")]
fn parsed_document_includes_guest_bios(
    #[from(validated_state)] state: &ParseState,
) -> anyhow::Result<()> {
    let document = state
        .result()?
        .context("expected successful parse before asserting guest bios")?;
    document
        .validate()
        .context("guest-bios TEI should validate")?;

    let Some(BodyBlock::Div(div)) = document.text().body().blocks().first() else {
        bail!("expected a top-level guest-bios div body block");
    };
    ensure!(
        div.div_type() == "guest-bios",
        "expected guest-bios div type, found {:?}",
        div.div_type()
    );

    let Some(DivContent::List(list)) = div.content().first() else {
        bail!("expected guest-bios div to contain a list");
    };
    let item = list
        .items()
        .first()
        .context("expected guest-bios list to contain an item")?;
    let label = item.label().context("expected guest-bio item label")?;
    ensure!(
        label.content() == [Inline::text("Ada Lovelace")],
        "guest-bio label should survive parsing"
    );
    ensure!(
        item.content() == [Inline::text("Mathematician and computing pioneer.")],
        "guest-bio inline body should survive parsing"
    );

    let expected =
        PointerList::parse_attribute("urn:episodic:reference-document-revision:019e1368")
            .context("expected corresp should be valid")?;
    ensure!(
        item.corresp() == Some(&expected),
        "guest-bio @corresp should survive parsing"
    );
    Ok(())
}

#[then("the emitted guest-bios XML round-trips cleanly")]
fn emitted_guest_bios_xml_round_trips(
    #[from(validated_state)] state: &ParseState,
) -> anyhow::Result<()> {
    let document = state
        .result()?
        .context("expected successful parse before round-trip")?;
    let emitted = emit_xml(&document).context("guest-bios TEI should emit")?;
    ensure!(
        emitted.contains("corresp=\"urn:episodic:reference-document-revision:019e1368\""),
        "guest-bio @corresp should survive emission"
    );
    let reparsed_doc = parse_xml(&emitted).context("emitted guest-bios TEI should parse again")?;
    reparsed_doc
        .validate()
        .context("reparsed guest-bios TEI should validate")?;
    Ok(())
}

#[scenario(path = "tests/features/parse_xml.feature", index = 6)]
fn parses_guest_bios(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: anyhow::Result<ParseState>,
) {
    expect_validated_state(result, "guest-bios parse");
}
