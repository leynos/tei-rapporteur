//! Behaviour-driven scenarios covering document emission.

use anyhow::{Context, Result, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{FileDesc, P, TeiDocument, TeiError, TeiHeader, TeiText};
use tei_xml::{emit_xml, parse_xml};

const _: &str = include_str!("features/emit_xml.feature");

const PRETTY_FIXTURE: &str = r#"<TEI>
  <teiHeader>
    <fileDesc>
      <title>Sample Episode</title>
      <series>Broadcasts</series>
    </fileDesc>
  </teiHeader>
  <text>
    <body>
      <p xml:id="intro">Hello <hi rend="stress">world</hi></p>
      <u xml:id="u1" who="host">Mind the gap</u>
    </body>
  </text>
</TEI>
"#;

const CANONICAL_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Sample Episode</title>",
    "<series>Broadcasts</series>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<p xml:id=\"intro\">Hello <hi rend=\"stress\">world</hi></p>",
    "<u xml:id=\"u1\" who=\"host\">Mind the gap</u>",
    "</body>",
    "</text>",
    "</TEI>",
);

#[derive(Default)]
struct EmitState {
    xml: RefCell<Option<String>>,
    document: RefCell<Option<Result<TeiDocument, TeiError>>>,
    emission: RefCell<Option<Result<String, TeiError>>>,
}

impl EmitState {
    fn set_xml(&self, value: &str) {
        *self.xml.borrow_mut() = Some(value.to_owned());
    }

    fn xml(&self) -> Result<String> {
        self.xml
            .borrow()
            .as_ref()
            .cloned()
            .context("scenario must define XML input")
    }

    fn set_document(&self, result: Result<TeiDocument, TeiError>) {
        *self.document.borrow_mut() = Some(result);
    }

    fn document(&self) -> Result<Result<TeiDocument, TeiError>> {
        self.document
            .borrow()
            .as_ref()
            .cloned()
            .context("document must exist before emission")
    }

    fn set_emission(&self, result: Result<String, TeiError>) {
        *self.emission.borrow_mut() = Some(result);
    }

    fn emission(&self) -> Result<Result<String, TeiError>> {
        self.emission
            .borrow()
            .as_ref()
            .cloned()
            .context("emission must run before assertions")
    }
}

fn tei_fixture(name: &str) -> Result<&'static str> {
    match name {
        "pretty-with-ids" => Ok(PRETTY_FIXTURE),
        "canonical-with-ids" => Ok(CANONICAL_FIXTURE),
        other => bail!("unknown TEI fixture: {other}"),
    }
}

fn canonical_fixture(name: &str) -> Result<&'static str> {
    match name {
        "canonical-with-ids" => Ok(CANONICAL_FIXTURE),
        other => bail!("unknown canonical fixture: {other}"),
    }
}

fn document_with_control_characters() -> TeiDocument {
    let paragraph = P::from_text_segments(["\u{1}"]).unwrap_or_else(|error| {
        panic!("paragraph should accept control text for testing: {error}")
    });

    let mut text = TeiText::empty();
    text.push_paragraph(paragraph);

    let file_desc = FileDesc::from_title_str("Invalid Control Characters")
        .unwrap_or_else(|error| panic!("title should validate: {error}"));
    let header = TeiHeader::new(file_desc);

    TeiDocument::new(header, text)
}

#[fixture]
fn validated_state_result() -> Result<EmitState> {
    let state = EmitState::default();
    ensure!(state.xml.borrow().is_none(), "xml slot must start empty");
    ensure!(
        state.document.borrow().is_none(),
        "document slot must start empty"
    );
    ensure!(
        state.emission.borrow().is_none(),
        "emission slot must start empty"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> EmitState {
    match validated_state_result() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialise emission state: {error}"),
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[given("the TEI fixture \"{fixture}\"")]
fn the_tei_fixture(#[from(validated_state)] state: &EmitState, fixture: String) -> Result<()> {
    let xml = tei_fixture(&fixture)?;
    state.set_xml(xml);
    let _ = state.xml()?;
    Ok(())
}

#[given("a parsed document containing invalid control characters")]
fn a_document_with_invalid_control_characters(
    #[from(validated_state)] state: &EmitState,
) -> Result<()> {
    state.set_document(Ok(document_with_control_characters()));
    state
        .document()?
        .context("expected document to be initialised")?;
    Ok(())
}

#[when("I parse the TEI input")]
fn i_parse_the_input(#[from(validated_state)] state: &EmitState) -> Result<()> {
    let xml = state.xml()?;
    let result = parse_xml(&xml);
    state.set_document(result);
    Ok(())
}

#[when("I emit the parsed document")]
fn i_emit_the_parsed_document(#[from(validated_state)] state: &EmitState) -> Result<()> {
    let document = state
        .document()?
        .context("expected parsed document before emission")?;
    let result = emit_xml(&document);
    state.set_emission(result);
    Ok(())
}

#[then("emission succeeds")]
fn emission_succeeds(#[from(validated_state)] state: &EmitState) -> Result<()> {
    let result = state.emission()?;
    result.context("expected emission to succeed")?;
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[then("the XML matches the canonical fixture \"{fixture}\"")]
fn xml_matches_canonical_fixture(
    #[from(validated_state)] state: &EmitState,
    fixture: String,
) -> Result<()> {
    let expected = canonical_fixture(&fixture)?;
    let emitted = state
        .emission()?
        .context("expected successful emission before comparison")?;
    ensure!(
        emitted == expected,
        "canonical XML mismatch: expected {expected:?}, found {emitted:?}"
    );
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[then("the emitted XML contains \"{snippet}\"")]
fn emitted_xml_contains(#[from(validated_state)] state: &EmitState, snippet: String) -> Result<()> {
    let emission = state
        .emission()?
        .context("expected emission before substring assertion")?;
    ensure!(
        emission.contains(&snippet),
        "emitted XML should contain {snippet:?}, found {emission:?}"
    );
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[then("emission fails mentioning \"{snippet}\"")]
fn emission_fails_mentioning(
    #[from(validated_state)] state: &EmitState,
    snippet: String,
) -> Result<()> {
    let outcome = state.emission()?;
    let Err(error) = outcome else {
        bail!("expected emission to fail");
    };
    let message = error.to_string();
    ensure!(
        message.contains(&snippet),
        "error should mention {snippet:?}, found {message:?}"
    );
    Ok(())
}

#[scenario(path = "tests/features/emit_xml.feature", index = 0)]
fn canonicalises_pretty_printed_input(
    #[from(validated_state)] _: EmitState,
    #[from(validated_state_result)] result: Result<EmitState>,
) -> Result<()> {
    let _ = result?;
    Ok(())
}

#[scenario(path = "tests/features/emit_xml.feature", index = 1)]
fn preserves_xml_id_attributes(
    #[from(validated_state)] _: EmitState,
    #[from(validated_state_result)] result: Result<EmitState>,
) -> Result<()> {
    let _ = result?;
    Ok(())
}

#[scenario(path = "tests/features/emit_xml.feature", index = 2)]
fn rejects_control_characters(
    #[from(validated_state)] _: EmitState,
    #[from(validated_state_result)] result: Result<EmitState>,
) -> Result<()> {
    let _ = result?;
    Ok(())
}
