//! Behaviour-driven scenarios covering TEI XML emission.

use anyhow::{Context, Result, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{TeiDocument, TeiError};
use tei_xml::emit_xml;

// Keep the compiled test binary aligned with the feature file contents.
const _: &str = include_str!("features/emit_xml.feature");

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

fn title_fixture(name: &str) -> Result<&'static str> {
    match name {
        "wolf-359" => Ok("Wolf 359"),
        "null-control" => Ok("\u{0}"),
        other => bail!("unknown document title fixture: {other}"),
    }
}

type EmitResult = std::result::Result<String, TeiError>;

#[derive(Default)]
struct EmitState {
    document: RefCell<Option<TeiDocument>>,
    result: RefCell<Option<EmitResult>>,
}

impl EmitState {
    fn set_document(&self, document: TeiDocument) {
        *self.document.borrow_mut() = Some(document);
    }

    fn document(&self) -> Result<TeiDocument> {
        self.document
            .borrow()
            .as_ref()
            .cloned()
            .context("the scenario must define a document before emitting")
    }

    fn set_result(&self, result: EmitResult) {
        *self.result.borrow_mut() = Some(result);
    }

    fn result(&self) -> Result<EmitResult> {
        self.result
            .borrow()
            .as_ref()
            .cloned()
            .context("emit_xml must run before assertions")
    }
}

#[fixture]
fn validated_state_result() -> Result<EmitState> {
    let state = EmitState::default();
    ensure!(
        state.document.borrow().is_none(),
        "fresh emit state must not contain a document"
    );
    ensure!(
        state.result.borrow().is_none(),
        "fresh emit state must not contain a result"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> EmitState {
    match validated_state_result() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialise emit state: {error}"),
    }
}

// rstest-bdd placeholders own their `String` values.
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[given("the document title fixture \"{fixture}\"")]
fn the_document_title_fixture(
    #[from(validated_state)] state: &EmitState,
    fixture: String,
) -> Result<()> {
    let title = title_fixture(&fixture)?;
    let document = TeiDocument::from_title_str(title)
        .with_context(|| format!("failed to build document from {fixture}"))?;
    state.set_document(document);
    let _ = state.document()?;
    Ok(())
}

#[when("I emit the TEI document")]
fn i_emit_the_tei_document(#[from(validated_state)] state: &EmitState) -> Result<()> {
    let document = state.document()?;
    let result = emit_xml(&document);
    state.set_result(result);
    Ok(())
}

#[then("emitting succeeds")]
fn emitting_succeeds(#[from(validated_state)] state: &EmitState) -> Result<()> {
    let result = state.result()?;
    result.context("expected emission to succeed")?;
    Ok(())
}

#[then("the TEI output equals the minimal fixture")]
fn the_output_equals_the_minimal_fixture(#[from(validated_state)] state: &EmitState) -> Result<()> {
    let xml = state
        .result()?
        .context("expected XML emission before asserting output")?;
    ensure!(
        xml == MINIMAL_FIXTURE,
        "emitted XML mismatch: expected {MINIMAL_FIXTURE:?}, found {xml:?}"
    );
    Ok(())
}

// rstest-bdd placeholders own their `String` values.
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest-bdd placeholders must own their `String` values"
)]
#[then("emitting fails mentioning \"{snippet}\"")]
fn emitting_fails_mentioning(
    #[from(validated_state)] state: &EmitState,
    snippet: String,
) -> Result<()> {
    let outcome = state.result()?;
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
fn emits_a_minimal_document(
    #[from(validated_state)] _: EmitState,
    #[from(validated_state_result)] result: Result<EmitState>,
) -> Result<()> {
    let _ = result?;
    Ok(())
}

#[scenario(path = "tests/features/emit_xml.feature", index = 1)]
fn rejects_control_characters(
    #[from(validated_state)] _: EmitState,
    #[from(validated_state_result)] result: Result<EmitState>,
) -> Result<()> {
    let _ = result?;
    Ok(())
}
