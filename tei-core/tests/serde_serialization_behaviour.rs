//! Behaviour-driven scenarios for JSON and `MessagePack` serialization.

use anyhow::{Context, Result, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::TeiDocument;
use tei_serde::json::{JsonError, Value as JsonValue};
use tei_serde::msgpack::MsgpackDecodeError;
use tei_test_helpers::expect_validated_state;

fn assert_document_title(document: &TeiDocument, expected_title: &str) -> Result<()> {
    ensure!(
        document.title().as_str() == expected_title,
        "expected title {expected_title:?}, found {:?}",
        document.title().as_str()
    );
    Ok(())
}

fn assert_outcome_title<E>(
    outcome: Result<&TeiDocument, &E>,
    expected_title: &str,
    context: &str,
) -> Result<()>
where
    E: std::fmt::Display,
{
    match outcome {
        Ok(document) => assert_document_title(document, expected_title),
        Err(error) => Err(anyhow::anyhow!("{context}: {error}")),
    }
}

#[derive(Default)]
struct SerdeState {
    document: RefCell<Option<TeiDocument>>,
    msgpack_payload: RefCell<Option<Vec<u8>>>,
    json_payload: RefCell<Option<String>>,
    msgpack_outcome: RefCell<Option<Result<TeiDocument, MsgpackDecodeError>>>,
    json_outcome: RefCell<Option<Result<TeiDocument, JsonError>>>,
}

impl SerdeState {
    fn set_document(&self, document: TeiDocument) {
        *self.document.borrow_mut() = Some(document);
        self.msgpack_outcome.borrow_mut().take();
        self.json_outcome.borrow_mut().take();
    }

    fn document(&self) -> Result<std::cell::Ref<'_, TeiDocument>> {
        std::cell::Ref::filter_map(self.document.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must configure a document"))
    }

    fn store_msgpack_payload(&self, payload: Vec<u8>) {
        *self.msgpack_payload.borrow_mut() = Some(payload);
    }

    fn msgpack_payload(&self) -> Result<std::cell::Ref<'_, Vec<u8>>> {
        std::cell::Ref::filter_map(self.msgpack_payload.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must define a `MessagePack` payload"))
    }

    fn store_json_payload(&self, payload: String) {
        *self.json_payload.borrow_mut() = Some(payload);
    }

    fn json_payload(&self) -> Result<String> {
        self.json_payload
            .borrow()
            .as_ref()
            .cloned()
            .context("scenario must define a JSON payload")
    }

    fn set_msgpack_outcome(&self, outcome: Result<TeiDocument, MsgpackDecodeError>) {
        *self.msgpack_outcome.borrow_mut() = Some(outcome);
    }

    fn msgpack_outcome(
        &self,
    ) -> Result<std::cell::Ref<'_, Result<TeiDocument, MsgpackDecodeError>>> {
        std::cell::Ref::filter_map(self.msgpack_outcome.borrow(), Option::as_ref).map_err(|_| {
            anyhow::anyhow!("scenario must deserialize `MessagePack` before assertions")
        })
    }

    fn set_json_outcome(&self, outcome: Result<TeiDocument, JsonError>) {
        *self.json_outcome.borrow_mut() = Some(outcome);
    }

    fn json_outcome(&self) -> Result<std::cell::Ref<'_, Result<TeiDocument, JsonError>>> {
        std::cell::Ref::filter_map(self.json_outcome.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must deserialize JSON before assertions"))
    }
}

fn build_state() -> Result<SerdeState> {
    let state = SerdeState::default();
    ensure!(
        state.document.borrow().is_none(),
        "fresh state must not contain a document"
    );
    ensure!(
        state.msgpack_payload.borrow().is_none(),
        "fresh state must not contain `MessagePack` payloads"
    );
    ensure!(
        state.json_payload.borrow().is_none(),
        "fresh state must not contain JSON payloads"
    );
    ensure!(
        state.msgpack_outcome.borrow().is_none(),
        "fresh state must not contain `MessagePack` outcomes"
    );
    ensure!(
        state.json_outcome.borrow().is_none(),
        "fresh state must not contain JSON outcomes"
    );
    Ok(state)
}

#[fixture]
fn validated_state_result() -> Result<SerdeState> {
    build_state()
}

#[fixture]
fn validated_state() -> SerdeState {
    expect_validated_state(build_state(), "serialization")
}

#[given("a TEI document titled \"{title}\"")]
fn a_tei_document_titled(#[from(validated_state)] state: &SerdeState, title: String) -> Result<()> {
    let document =
        TeiDocument::from_title_str(title.as_str()).context("fixture document must construct")?;
    state.set_document(document);
    Ok(())
}

#[given("an invalid MessagePack payload")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "rstest-bdd step signatures stay uniform even when storing literals"
)]
fn an_invalid_messagepack_payload(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    state.store_msgpack_payload(Vec::new());
    Ok(())
}

#[given("a JSON payload with a blank title")]
fn a_json_payload_with_a_blank_title(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let document = TeiDocument::from_title_str("placeholder")
        .context("placeholder document must construct")?;
    let mut payload =
        tei_serde::json::to_value(&document).context("serializing fixture to JSON should work")?;

    let Some(title) = payload.pointer_mut("/teiHeader/fileDesc/title") else {
        return Err(anyhow::anyhow!(
            "expected JSON payload to contain /teiHeader/fileDesc/title"
        ));
    };
    *title = JsonValue::String("   ".to_owned());

    let payload_text =
        tei_serde::json::to_string(&payload).context("serializing mutated JSON should succeed")?;
    state.store_json_payload(payload_text);
    Ok(())
}

#[given("an invalid JSON payload")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "rstest-bdd step signatures stay uniform even when storing literals"
)]
fn an_invalid_json_payload(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    state.store_json_payload("this is not JSON".to_owned());
    Ok(())
}

#[when("I serialize the document as MessagePack")]
fn i_serialize_the_document_as_messagepack(
    #[from(validated_state)] state: &SerdeState,
) -> Result<()> {
    let document = state.document()?;
    let payload = tei_serde::msgpack::to_vec_named(&*document)
        .context("serializing to `MessagePack` should succeed")?;
    state.store_msgpack_payload(payload);
    Ok(())
}

#[when("I deserialize the MessagePack payload")]
fn i_deserialize_the_messagepack_payload(
    #[from(validated_state)] state: &SerdeState,
) -> Result<()> {
    let payload = state.msgpack_payload()?;
    let outcome = tei_serde::msgpack::from_slice::<TeiDocument>(payload.as_slice());
    state.set_msgpack_outcome(outcome);
    Ok(())
}

#[when("I serialize the document as JSON")]
fn i_serialize_the_document_as_json(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let document = state.document()?;
    let payload =
        tei_serde::json::to_string(&*document).context("serializing to JSON should succeed")?;
    state.store_json_payload(payload);
    Ok(())
}

#[when("I deserialize the JSON payload")]
fn i_deserialize_the_json_payload(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let payload = state.json_payload()?;
    let outcome = tei_serde::json::from_str::<TeiDocument>(&payload);
    state.set_json_outcome(outcome);
    Ok(())
}

#[then("the deserialized document title is \"{title}\"")]
fn the_deserialized_document_title_is(
    #[from(validated_state)] state: &SerdeState,
    title: String,
) -> Result<()> {
    if let Ok(outcome) = state.msgpack_outcome() {
        return assert_outcome_title(
            outcome.as_ref(),
            title.as_str(),
            "expected `MessagePack` deserialization to succeed",
        );
    }

    if let Ok(outcome) = state.json_outcome() {
        return assert_outcome_title(
            outcome.as_ref(),
            title.as_str(),
            "expected JSON deserialization to succeed",
        );
    }

    Err(anyhow::anyhow!(
        "scenario must deserialize either `MessagePack` or JSON before asserting on title"
    ))
}

#[then("MessagePack deserialization fails")]
fn messagepack_deserialization_fails(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let outcome = state.msgpack_outcome()?;
    let Err(error) = outcome.as_ref() else {
        return Err(anyhow::anyhow!(
            "expected `MessagePack` deserialization to fail"
        ));
    };
    ensure!(
        matches!(error, MsgpackDecodeError::InvalidMarkerRead(_)),
        "expected invalid marker read error, got: {error}"
    );
    Ok(())
}

#[then("JSON deserialization fails")]
fn json_deserialization_fails(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let outcome = state.json_outcome()?;
    let Err(error) = outcome.as_ref() else {
        return Err(anyhow::anyhow!("expected JSON deserialization to fail"));
    };
    ensure!(error.is_data(), "expected JSON data error, got: {error}");
    Ok(())
}

#[then("JSON deserialization fails with a syntax error")]
fn json_deserialization_fails_with_a_syntax_error(
    #[from(validated_state)] state: &SerdeState,
) -> Result<()> {
    let outcome = state.json_outcome()?;
    let Err(error) = outcome.as_ref() else {
        return Err(anyhow::anyhow!("expected JSON deserialization to fail"));
    };
    ensure!(error.is_syntax(), "expected syntax error, got: {error}");
    Ok(())
}

/// Scenario: Serialize a document to `MessagePack` and back.
#[scenario(path = "tests/features/serde_serialization.feature", index = 0)]
pub fn serializes_messagepack_round_trip(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}

/// Scenario: Reject invalid `MessagePack` payloads.
#[scenario(path = "tests/features/serde_serialization.feature", index = 1)]
pub fn rejects_invalid_messagepack_payloads(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}

/// Scenario: Serialize a document to JSON and back.
#[scenario(path = "tests/features/serde_serialization.feature", index = 2)]
pub fn serializes_json_round_trip(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}

/// Scenario: Reject JSON payloads with blank titles.
#[scenario(path = "tests/features/serde_serialization.feature", index = 3)]
pub fn rejects_blank_titles_in_json(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}

/// Scenario: Reject invalid JSON payloads.
#[scenario(path = "tests/features/serde_serialization.feature", index = 4)]
pub fn rejects_invalid_json_payloads(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}
