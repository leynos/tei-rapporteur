//! Behaviour-driven tests for TEI body composition.

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::{cell::RefCell, fmt::Display};
use tei_core::{BodyBlock, BodyContentError, P, TeiBody, Utterance};

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

fn expect_ok<T, E>(result: Result<T, E>, message: &str) -> T
where
    E: Display,
{
    match result {
        Ok(value) => value,
        Err(error) => panic!("{message}: {error}"),
    }
}

/// Helper to retrieve a block at a 1-based index and execute an assertion on it.
fn with_block_at_index<F>(state: &BodyState, index: usize, f: F)
where
    F: FnOnce(&BodyBlock),
{
    #[expect(
        clippy::expect_used,
        reason = "Scenario uses human friendly 1-based indices"
    )]
    let zero_based = index.checked_sub(1).expect("block indices start at 1");
    let body = state.body();
    #[expect(clippy::expect_used, reason = "Scenario declares existing blocks")]
    let block = body
        .blocks()
        .get(zero_based)
        .expect("scenario should configure the block");
    f(block);
}

#[fixture]
fn state() -> BodyState {
    BodyState::default()
}

#[given("an empty TEI body")]
fn an_empty_body(state: &BodyState) {
    state.reset_body();
}

#[when("I add a paragraph containing \"{content}\"")]
fn i_add_a_paragraph(state: &BodyState, content: String) {
    let paragraph = expect_ok(P::new([content]), "paragraph should be valid");
    state.push_paragraph(paragraph);
}

#[when("I add an utterance for \"{speaker}\" saying \"{content}\"")]
fn i_add_an_utterance(state: &BodyState, speaker: String, content: String) {
    let utterance = expect_ok(
        Utterance::new(Some(speaker), [content]),
        "utterance should be valid",
    );
    state.push_utterance(utterance);
}

#[when("I attempt to record an utterance for \"{speaker}\" saying \"{content}\"")]
fn i_attempt_to_record_an_utterance(state: &BodyState, speaker: String, content: String) {
    match Utterance::new(Some(speaker), [content]) {
        Ok(utterance) => state.push_utterance(utterance),
        Err(error) => state.set_error(error),
    }
}

#[then("the body should report {count} blocks")]
fn the_body_should_report_blocks(state: &BodyState, count: usize) {
    let body = state.body();
    assert_eq!(body.blocks().len(), count, "unexpected block count");
}

#[then("block {index} should be a paragraph with \"{content}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn block_should_be_paragraph(state: &BodyState, index: usize, content: String) {
    with_block_at_index(state, index, |block| {
        let BodyBlock::Paragraph(paragraph) = block else {
            panic!("expected block {index} to be a paragraph");
        };
        let expected = std::slice::from_ref(&content);
        assert_eq!(paragraph.segments(), expected, "paragraph content mismatch",);
    });
}

#[then("block {index} should be an utterance for \"{speaker}\" with \"{content}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn block_should_be_utterance(state: &BodyState, index: usize, speaker: String, content: String) {
    with_block_at_index(state, index, |block| {
        let BodyBlock::Utterance(utterance) = block else {
            panic!("expected block {index} to be an utterance");
        };
        let expected = std::slice::from_ref(&content);
        assert_eq!(
            utterance.speaker(),
            Some(speaker.as_str()),
            "speaker mismatch"
        );
        assert_eq!(utterance.segments(), expected, "utterance content mismatch",);
    });
}

#[then("body validation fails with \"{message}\"")]
#[expect(
    clippy::expect_used,
    reason = "Scenario must attempt an utterance before asserting on the error"
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn body_validation_fails_with(state: &BodyState, message: String) {
    let binding = state.last_error.borrow();
    let error = binding.as_ref().expect("expected an error");
    assert_eq!(error.to_string(), message);
}

#[scenario(path = "tests/features/body.feature", index = 0)]
fn records_paragraphs_and_utterances(state: BodyState) {
    let _ = state;
}

#[scenario(path = "tests/features/body.feature", index = 1)]
fn rejects_empty_utterance_content(state: BodyState) {
    let _ = state;
}
