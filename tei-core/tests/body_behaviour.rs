//! Behaviour-driven tests for TEI body composition.

use anyhow::{Context, Result, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{BodyBlock, BodyContentError, Hi, Inline, P, Pause, Speaker, TeiBody, Utterance};

#[derive(Clone, Debug, Default)]
struct MixedInlineExpectation {
    prefix: String,
    emphasis: String,
    rend: String,
    suffix: String,
}

#[derive(Clone, Debug, Default)]
struct PauseExpectation {
    kind: String,
    duration: String,
}

#[derive(Default)]
struct BodyState {
    body: RefCell<TeiBody>,
    last_error: RefCell<Option<BodyContentError>>,
    last_mixed: RefCell<Option<MixedInlineExpectation>>,
    last_pause: RefCell<Option<PauseExpectation>>,
}

impl BodyState {
    fn reset_body(&self) {
        *self.body.borrow_mut() = TeiBody::default();
        *self.last_error.borrow_mut() = None;
        *self.last_mixed.borrow_mut() = None;
        *self.last_pause.borrow_mut() = None;
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

    fn set_mixed_expectation(&self, expectation: MixedInlineExpectation) {
        *self.last_mixed.borrow_mut() = Some(expectation);
    }

    fn mixed_expectation(&self) -> Option<MixedInlineExpectation> {
        self.last_mixed.borrow().clone()
    }

    fn set_pause_expectation(&self, expectation: PauseExpectation) {
        *self.last_pause.borrow_mut() = Some(expectation);
    }

    fn pause_expectation(&self) -> Option<PauseExpectation> {
        self.last_pause.borrow().clone()
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
    ensure!(
        state.last_mixed.borrow().is_none(),
        "fresh body must not record mixed inline expectations",
    );
    ensure!(
        state.last_pause.borrow().is_none(),
        "fresh body must not record pause expectations",
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
    let paragraph = P::from_text_segments([content]).context("paragraph should be valid")?;
    state.push_paragraph(paragraph);
    Ok(())
}

#[when("I add a paragraph emphasising \"{content}\"")]
fn i_add_a_paragraph_emphasising(
    #[from(validated_state)] state: &BodyState,
    content: String,
) -> Result<()> {
    let emphasis = Inline::hi([Inline::text(content)]);
    let paragraph = P::from_inline([emphasis]).context("paragraph should be valid")?;
    state.push_paragraph(paragraph);
    Ok(())
}

#[when("I add a paragraph mixing \"{prefix}\" with emphasis \"{emphasis}\" rendered as \"{rend}\"")]
fn i_add_a_paragraph_mixing(
    #[from(validated_state)] state: &BodyState,
    prefix: String,
    emphasis: String,
    rend: String,
) -> Result<()> {
    let expectation = MixedInlineExpectation {
        prefix,
        emphasis,
        rend,
        suffix: String::from("!"),
    };
    let emphasis_inline = Inline::Hi(Hi::with_rend(
        expectation.rend.clone(),
        [Inline::text(expectation.emphasis.clone())],
    ));
    let paragraph = P::from_inline([
        Inline::text(expectation.prefix.clone()),
        emphasis_inline,
        Inline::text(expectation.suffix.clone()),
    ])
    .context("paragraph should accept mixed inline content")?;
    state.push_paragraph(paragraph);
    state.set_mixed_expectation(expectation);
    Ok(())
}

#[when("I attempt to add a paragraph containing \"{content}\"")]
fn i_attempt_to_add_paragraph(
    #[from(validated_state)] state: &BodyState,
    content: String,
) -> Result<()> {
    match P::from_text_segments([content]) {
        Ok(paragraph) => state.push_paragraph(paragraph),
        Err(error) => state.set_error(error),
    }
    ensure_attempt_recorded_or_appended(state, "paragraph")?;
    Ok(())
}

#[when("I attempt to add a paragraph emphasising \"{content}\"")]
fn i_attempt_to_add_paragraph_emphasising(
    #[from(validated_state)] state: &BodyState,
    content: String,
) -> Result<()> {
    match P::from_inline([Inline::hi([Inline::text(content)])]) {
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
    let utterance = Utterance::from_text_segments(Some(speaker), [content])
        .context("utterance should be valid")?;
    state.push_utterance(utterance);
    Ok(())
}

#[when("I add an utterance for \"{speaker}\" with a pause cue")]
fn i_add_an_utterance_with_pause(
    #[from(validated_state)] state: &BodyState,
    speaker: String,
) -> Result<()> {
    let utterance = Utterance::from_inline(
        Some(speaker),
        [Inline::text("Wait"), Inline::pause(), Inline::text(" go")],
    )
    .context("utterance with pause should be valid")?;
    state.push_utterance(utterance);
    Ok(())
}

#[when("I add an utterance for \"{speaker}\" with a \"{kind}\" pause lasting \"{duration}\"")]
fn i_add_an_utterance_with_measured_pause(
    #[from(validated_state)] state: &BodyState,
    speaker: String,
    kind: String,
    duration: String,
) -> Result<()> {
    let expectation = PauseExpectation { kind, duration };
    let mut pause = Pause::new();
    pause.set_kind(expectation.kind.as_str());
    pause.set_duration(expectation.duration.as_str());

    let utterance = Utterance::from_inline(
        Some(speaker),
        [
            Inline::text("We are"),
            Inline::Pause(pause),
            Inline::text(" live"),
        ],
    )
    .context("utterance with measured pause should be valid")?;
    state.push_utterance(utterance);
    state.set_pause_expectation(expectation);
    Ok(())
}

#[when("I attempt to set paragraph identifier to \"{identifier}\"")]
fn i_attempt_to_set_paragraph_identifier(
    #[from(validated_state)] state: &BodyState,
    identifier: String,
) -> Result<()> {
    let mut paragraph = P::from_text_segments(["Valid paragraph content"])
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
    match Utterance::from_text_segments(Some(speaker), [content]) {
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
    let mut utterance = Utterance::from_text_segments(Some("Host"), ["Valid utterance content"])
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
        let expected = [Inline::text(content.clone())];
        let actual_segments = paragraph.content();
        ensure!(
            actual_segments == expected.as_slice(),
            "paragraph content mismatch: expected {expected:?}, found {actual_segments:?}"
        );
        Ok(())
    })
}

#[then("block {index} should emphasise \"{content}\"")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "rstest_bdd supplies owned Strings for captured step parameters."
)]
fn block_should_emphasise(
    #[from(validated_state)] state: &BodyState,
    index: usize,
    content: String,
) -> Result<()> {
    with_block_at_index(state, index, |block| {
        let BodyBlock::Paragraph(paragraph) = block else {
            bail!("expected block {index} to be a paragraph");
        };
        let [Inline::Hi(hi)] = paragraph.content() else {
            bail!("paragraph should contain a single emphasised inline");
        };
        let mut emphasised_iter = hi.content().iter().filter_map(Inline::as_text);
        let actual = emphasised_iter
            .next()
            .context("expected emphasised segment")?;
        ensure!(
            emphasised_iter.next().is_none(),
            "expected a single emphasised segment, found {:?}",
            hi.content()
        );
        ensure!(
            actual == content,
            "emphasised content mismatch: expected {content}, found {actual}"
        );
        Ok(())
    })
}

#[then("block {index} should reflect the mixed inline paragraph")]
fn block_should_mix_inline(#[from(validated_state)] state: &BodyState, index: usize) -> Result<()> {
    let expectation = state
        .mixed_expectation()
        .context("expected mixed inline expectation")?;
    with_block_at_index(state, index, |block| {
        let BodyBlock::Paragraph(paragraph) = block else {
            bail!("expected block {index} to be a paragraph");
        };
        let expected_prefix = expectation.prefix.as_str();
        let expected_emphasis = expectation.emphasis.as_str();
        let expected_rend = expectation.rend.as_str();
        let expected_suffix = expectation.suffix.as_str();

        let [
            Inline::Text(leading_text),
            Inline::Hi(hi),
            Inline::Text(trailing_text),
        ] = paragraph.content()
        else {
            bail!("paragraph should contain text, emphasis, and trailing text segments");
        };
        ensure!(
            leading_text.as_str() == expected_prefix,
            "paragraph prefix mismatch: expected {expected_prefix}, found {leading_text}"
        );

        ensure!(
            hi.rend() == Some(expected_rend),
            "expected emphasis rend {}, found {:?}",
            expected_rend,
            hi.rend(),
        );
        let mut emphasised_iter = hi.content().iter().filter_map(Inline::as_text);
        let actual_emphasis = emphasised_iter
            .next()
            .context("expected emphasised segment")?;
        ensure!(
            emphasised_iter.next().is_none(),
            "expected a single emphasised segment, found {:?}",
            hi.content(),
        );
        ensure!(
            actual_emphasis == expected_emphasis,
            "emphasised content mismatch: expected {expected_emphasis}, found {actual_emphasis}"
        );

        ensure!(
            trailing_text.as_str() == expected_suffix,
            "paragraph suffix mismatch: expected {expected_suffix}, found {trailing_text}"
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
        let expected = [Inline::text(content.clone())];
        let actual_segments = utterance.content();
        ensure!(
            actual_segments == expected.as_slice(),
            "utterance content mismatch: expected {expected:?}, found {actual_segments:?}"
        );
        Ok(())
    })
}

#[then("block {index} should include a pause inline")]
fn block_should_include_pause(
    #[from(validated_state)] state: &BodyState,
    index: usize,
) -> Result<()> {
    with_block_at_index(state, index, |block| {
        let BodyBlock::Utterance(utterance) = block else {
            bail!("expected block {index} to be an utterance");
        };
        let has_pause = utterance
            .content()
            .iter()
            .any(|inline| matches!(inline, Inline::Pause(_)));
        ensure!(has_pause, "expected utterance to contain a pause inline");
        Ok(())
    })
}

#[then("block {index} should include the measured pause inline")]
fn block_should_include_measured_pause(
    #[from(validated_state)] state: &BodyState,
    index: usize,
) -> Result<()> {
    let expectation = state
        .pause_expectation()
        .context("expected measured pause expectation")?;
    with_block_at_index(state, index, |block| {
        let BodyBlock::Utterance(utterance) = block else {
            bail!("expected block {index} to be an utterance");
        };
        let expected_duration = expectation.duration.as_str();
        let expected_kind = expectation.kind.as_str();
        let pause = utterance
            .content()
            .iter()
            .find_map(|inline| match inline {
                Inline::Pause(pause) => Some(pause),
                _ => None,
            })
            .context("expected utterance to contain a pause inline")?;
        ensure!(
            pause.duration() == Some(expected_duration),
            "pause duration mismatch: expected {}, found {:?}",
            expected_duration,
            pause.duration(),
        );
        ensure!(
            pause.kind() == Some(expected_kind),
            "pause kind mismatch: expected {}, found {:?}",
            expected_kind,
            pause.kind(),
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
fn records_inline_emphasis(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 2)]
fn records_pause_inline(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 3)]
fn rejects_empty_utterance_content(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 4)]
fn rejects_empty_paragraph_content(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 5)]
fn rejects_whitespace_paragraph_identifier(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 6)]
fn rejects_blank_speaker_reference(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 7)]
fn rejects_whitespace_utterance_identifier(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 8)]
fn rejects_empty_inline_emphasis(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 9)]
fn records_mixed_inline_content(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}

#[scenario(path = "tests/features/body.feature", index = 10)]
fn records_measured_pause_inline(
    #[from(validated_state)] _: BodyState,
    #[from(validated_state_result)] validated_state: Result<BodyState>,
) -> Result<()> {
    let _ = validated_state?;
    Ok(())
}
