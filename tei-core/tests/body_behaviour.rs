//! Behaviour-driven tests for TEI body composition.

use anyhow::{Context, Result, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{BodyBlock, BodyContentError, P, Speaker, TeiBody, Utterance};

#[derive(Default)]
struct BodyState {
    body: RefCell<TeiBody>,
    last_error: RefCell<Option<BodyContentError>>,
}

impl BodyState {
    fn reset_body(&self) {
        *self.body.borrow_mut() = TeiBody::default();
        *self.last_error.borrow_mut() = None;
    }

    fn push_paragraph(&self, paragraph: P) {
        self.body.borrow_mut().push_paragraph(paragraph);
    }

    fn push_utterance(&self, utterance: Utterance) {
        self.body.borrow_mut().push_utterance(utterance);
    }

    fn set_error(&self, error: BodyContentError) {
        *self.last_error.borrow_mut() = Some(error);
    }

    fn body(&self) -> std::cell::Ref<'_, TeiBody> {
        self.body.borrow()
    }
}

/// Helper to retrieve a block at a 1-based index and execute an assertion on it.
fn with_block_at_index<F>(state: &BodyState, index: usize, f: F) -> Result<()>
where
    F: FnOnce(&BodyBlock) -> Result<()>,
{
    let zero_based = index.checked_sub(1).context("block indices start at 1")?;
    let body = state.body();
    let block = body
        .blocks()
        .get(zero_based)
        .context("scenario should configure the block")?;
    f(block)
}

fn build_state() -> Result<BodyState> {
    let state = BodyState::default();
    ensure!(
        state.body.borrow().blocks().is_empty(),
        "fresh body must start without blocks"
    );
    ensure!(
        state.last_error.borrow().is_none(),
        "fresh body must start without errors"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> BodyState {
    match build_state() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialise body state: {error}"),
    }
}

#[fixture]
fn validated_state_result() -> Result<BodyState> {
    build_state()
}

#[given("an empty TEI body")]
fn an_empty_body(#[from(validated_state)] state: &BodyState) -> Result<()> {
    state.reset_body();
    let body = state.body();
    ensure!(
        body.blocks().is_empty(),
        "body reset should remove all blocks"
    );
    ensure!(
        state.last_error.borrow().is_none(),
        "reset body should clear recorded errors"
    );
    Ok(())
}

#[when("I add a paragraph containing \"{content}\"")]
fn i_add_a_paragraph(#[from(validated_state)] state: &BodyState, content: String) -> Result<()> {
    let paragraph = P::new([content]).context("paragraph should be valid")?;
    state.push_paragraph(paragraph);
    Ok(())
}

#[when("I attempt to add a paragraph containing \"{content}\"")]
fn i_attempt_to_add_paragraph(
    #[from(validated_state)] state: &BodyState,
    content: String,
) -> Result<()> {
    match P::new([content]) {
        Ok(paragraph) => state.push_paragraph(paragraph),
        Err(error) => state.set_error(error),
    }
    ensure_attempt_recorded_or_appended(state, "paragraph")?;
    Ok(())
}

#[when("I add an utterance for \"{speaker}\" saying \"{content}\"")]
fn i_add_an_utterance(
    #[from(validated_state)] state: &BodyState,
    speaker: String,
    content: String,
) -> Result<()> {
    let utterance =
        Utterance::new(Some(speaker), [content]).context("utterance should be valid")?;
    state.push_utterance(utterance);
    Ok(())
}

#[when("I attempt to set paragraph identifier to \"{identifier}\"")]
fn i_attempt_to_set_paragraph_identifier(
    #[from(validated_state)] state: &BodyState,
    identifier: String,
) -> Result<()> {
    let mut paragraph = P::new(["Valid paragraph content"])
        .context("scenario baseline paragraph should be valid")?;

    match paragraph.set_id(identifier) {
        Ok(()) => state.push_paragraph(paragraph),
        Err(error) => state.set_error(error),
    }
    ensure_attempt_recorded_or_appended(state, "paragraph")?;
    Ok(())
}

#[when("I attempt to record an utterance for \"{speaker}\" saying \"{content}\"")]
fn i_attempt_to_record_an_utterance(
    #[from(validated_state)] state: &BodyState,
    speaker: String,
    content: String,
) -> Result<()> {
    match Utterance::new(Some(speaker), [content]) {
        Ok(utterance) => state.push_utterance(utterance),
        Err(error) => state.set_error(error),
    }
    ensure_attempt_recorded_or_appended(state, "utterance")?;
    Ok(())
}

#[when("I attempt to set utterance identifier to \"{identifier}\"")]
fn i_attempt_to_set_utterance_identifier(
    #[from(validated_state)] state: &BodyState,
    identifier: String,
) -> Result<()> {
    let mut utterance = Utterance::new(Some("Host"), ["Valid utterance content"])
        .context("scenario baseline utterance should be valid")?;

    match utterance.set_id(identifier) {
        Ok(()) => state.push_utterance(utterance),
        Err(error) => state.set_error(error),
    }

    ensure_attempt_recorded_or_appended(state, "utterance")?;
    Ok(())
}

fn ensure_attempt_recorded_or_appended(state: &BodyState, what: &str) -> Result<()> {
    let recorded_error = state.last_error.borrow().is_some();
    let block_count = state.body().blocks().len();
    ensure!(
        recorded_error || block_count > 0,
        "attempting to add a {what} should record an error or append a block",
    );
    Ok(())
}

#[then("the body should report {count} blocks")]
fn the_body_should_report_blocks(
    #[from(validated_state)] state: &BodyState,
    count: usize,
) -> Result<()> {
    let body = state.body();
    let actual = body.blocks().len();
    ensure!(
        actual == count,
        "unexpected block count: expected {count}, found {actual}"
    );
    Ok(())
}

#[then("block {index} should be a paragraph with \"{content}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn block_should_be_paragraph(
    #[from(validated_state)] state: &BodyState,
    index: usize,
    content: String,
) -> Result<()> {
    with_block_at_index(state, index, |block| {
        let BodyBlock::Paragraph(paragraph) = block else {
            bail!("expected block {index} to be a paragraph");
        };
        let expected = std::slice::from_ref(&content);
        let actual_segments = paragraph.segments();
        ensure!(
            actual_segments == expected,
            "paragraph content mismatch: expected {expected:?}, found {actual_segments:?}"
        );
        Ok(())
    })
}

#[then("block {index} should be an utterance for \"{speaker}\" with \"{content}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn block_should_be_utterance(
    #[from(validated_state)] state: &BodyState,
    index: usize,
    speaker: String,
    content: String,
) -> Result<()> {
    with_block_at_index(state, index, |block| {
        let BodyBlock::Utterance(utterance) = block else {
            bail!("expected block {index} to be an utterance");
        };
        let actual_speaker = utterance.speaker().map(Speaker::as_str);
        ensure!(
            actual_speaker == Some(speaker.as_str()),
            "speaker mismatch: expected {speaker}, found {actual_speaker:?}",
        );
        let expected = std::slice::from_ref(&content);
        let actual_segments = utterance.segments();
        ensure!(
            actual_segments == expected,
            "utterance content mismatch: expected {expected:?}, found {actual_segments:?}"
        );
        Ok(())
    })
}

#[then("body validation fails with \"{message}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn body_validation_fails_with(
    #[from(validated_state)] state: &BodyState,
    message: String,
) -> Result<()> {
    let binding = state.last_error.borrow();
    let error = binding.as_ref().context("expected an error")?;
    let actual = error.to_string();
    ensure!(
        actual == message,
        "validation error mismatch: expected {message}, found {actual}"
    );
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 0)]
fn records_paragraphs_and_utterances(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 1)]
fn rejects_empty_utterance_content(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 2)]
fn rejects_empty_paragraph_content(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 3)]
fn rejects_whitespace_paragraph_identifier(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 4)]
fn rejects_blank_speaker_reference(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 5)]
fn rejects_whitespace_utterance_identifier(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}
