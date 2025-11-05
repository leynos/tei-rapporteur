//! Behaviour-driven scenarios that cover parsing TEI XML strings into
//! structured documents.

use anyhow::{Context, Result, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{TeiDocument, TeiError};
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

#[derive(Default)]
struct ParseState {
    xml: RefCell<Option<String>>,
    result: RefCell<Option<Result<TeiDocument, TeiError>>>,
}

impl ParseState {
    fn set_xml(&self, xml: &str) {
        *self.xml.borrow_mut() = Some(xml.to_owned());
    }

    fn xml(&self) -> Result<String> {
        self.xml
            .borrow()
            .as_ref()
            .cloned()
            .context("scenario must supply XML input")
    }

    fn set_result(&self, result: Result<TeiDocument, TeiError>) {
        *self.result.borrow_mut() = Some(result);
    }

    fn result(&self) -> Result<Result<TeiDocument, TeiError>> {
        self.result
            .borrow()
            .as_ref()
            .cloned()
            .context("parse_xml must run before assertions")
    }
}

fn fixture_by_name(name: &str) -> Result<&'static str> {
    match name {
        "minimal" => Ok(MINIMAL_FIXTURE),
        "missing-header" => Ok(MISSING_HEADER_FIXTURE),
        "unterminated" => Ok(UNTERMINATED_FIXTURE),
        other => bail!("unknown TEI fixture: {other}"),
    }
}

#[fixture]
fn validated_state_result() -> Result<ParseState> {
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
        Err(error) => panic!("failed to initialise parse state: {error}"),
    }
}

// rstest-bdd supplies owned `String` values for placeholders, so keep the
// signature by value and expect the lint locally.
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[given("the TEI fixture \"{fixture}\"")]
fn the_tei_fixture(#[from(validated_state)] state: &ParseState, fixture: String) -> Result<()> {
    let xml = fixture_by_name(&fixture)?;
    state.set_xml(xml);
    let _ = state.xml()?;
    Ok(())
}

#[when("I parse the TEI input")]
fn i_parse_the_input(#[from(validated_state)] state: &ParseState) -> Result<()> {
    let xml = state.xml()?;
    let result = parse_xml(&xml);
    state.set_result(result);
    Ok(())
}

#[then("parsing succeeds")]
fn parsing_succeeds(#[from(validated_state)] state: &ParseState) -> Result<()> {
    let result = state.result()?;
    result.context("expected parsing to succeed")?;
    Ok(())
}

// rstest-bdd supplies owned `String` values for placeholders, so keep the
// signature by value and expect the lint locally.
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[then("the parsed title is \"{title}\"")]
fn parsed_title_is(#[from(validated_state)] state: &ParseState, title: String) -> Result<()> {
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
// signature by value and expect the lint locally.
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[then("parsing fails mentioning \"{snippet}\"")]
fn parsing_fails_with_snippet(
    #[from(validated_state)] state: &ParseState,
    snippet: String,
) -> Result<()> {
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

#[scenario(path = "tests/features/parse_xml.feature", index = 0)]
fn parses_valid_documents(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: Result<ParseState>,
) -> Result<()> {
    let _ = result?;
    Ok(())
}

#[scenario(path = "tests/features/parse_xml.feature", index = 1)]
fn reports_missing_headers(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: Result<ParseState>,
) -> Result<()> {
    let _ = result?;
    Ok(())
}

#[scenario(path = "tests/features/parse_xml.feature", index = 2)]
fn reports_malformed_xml(
    #[from(validated_state)] _: ParseState,
    #[from(validated_state_result)] result: Result<ParseState>,
) -> Result<()> {
    let _ = result?;
    Ok(())
}
