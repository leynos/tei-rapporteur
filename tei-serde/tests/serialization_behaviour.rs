//! Behaviour-driven scenarios for JSON and `MessagePack` serialisation.

use anyhow::{Context, Result, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::TeiDocument;
use tei_test_helpers::expect_validated_state;

#[derive(Default)]
struct SerdeState {
    document: RefCell<Option<TeiDocument>>,
    decoded_document: RefCell<Option<TeiDocument>>,
    msgpack_payload: RefCell<Option<Vec<u8>>>,
    json_payload: RefCell<Option<String>>,
    error: RefCell<Option<String>>,
}

impl SerdeState {
    fn set_document(&self, document: TeiDocument) {
        *self.document.borrow_mut() = Some(document);
        self.decoded_document.borrow_mut().take();
        self.error.borrow_mut().take();
    }

    fn document(&self) -> Result<std::cell::Ref<'_, TeiDocument>> {
        std::cell::Ref::filter_map(self.document.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must configure a document"))
    }

    fn set_decoded_document(&self, document: TeiDocument) {
        *self.decoded_document.borrow_mut() = Some(document);
        self.error.borrow_mut().take();
    }

    fn store_error(&self, message: String) {
        self.error.borrow_mut().replace(message);
        self.decoded_document.borrow_mut().take();
    }

    fn error(&self) -> Result<String> {
        self.error
            .borrow()
            .as_ref()
            .cloned()
            .context("scenario must record an error before assertions")
    }

    fn decoded_document(&self) -> Result<std::cell::Ref<'_, TeiDocument>> {
        std::cell::Ref::filter_map(self.decoded_document.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must decode a document before assertions"))
    }

    fn store_msgpack_payload(&self, payload: Vec<u8>) {
        *self.msgpack_payload.borrow_mut() = Some(payload);
    }

    fn msgpack_payload(&self) -> Result<std::cell::Ref<'_, Vec<u8>>> {
        std::cell::Ref::filter_map(self.msgpack_payload.borrow(), Option::as_ref)
            .map_err(|_| anyhow::anyhow!("scenario must define a MessagePack payload"))
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
}

fn build_state() -> Result<SerdeState> {
    let state = SerdeState::default();
    ensure!(
        state.document.borrow().is_none(),
        "fresh state must not contain a document"
    );
    ensure!(
        state.decoded_document.borrow().is_none(),
        "fresh state must not contain decoded documents"
    );
    ensure!(
        state.msgpack_payload.borrow().is_none(),
        "fresh state must not contain MessagePack payloads"
    );
    ensure!(
        state.json_payload.borrow().is_none(),
        "fresh state must not contain JSON payloads"
    );
    ensure!(
        state.error.borrow().is_none(),
        "fresh state must not contain recorded errors"
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
    let _ = state.document()?;
    Ok(())
}

#[given("an invalid MessagePack payload")]
fn an_invalid_messagepack_payload(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    state.store_msgpack_payload(Vec::new());
    let _ = state.msgpack_payload()?;
    Ok(())
}

#[given("a JSON payload with a blank title")]
fn a_json_payload_with_a_blank_title(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let document = TeiDocument::from_title_str("placeholder")
        .context("placeholder document must construct")?;
    let mut payload =
        tei_serde::json::to_value(&document).context("serialising fixtures to JSON should work")?;

    if let Some(title) = payload.pointer_mut("/teiHeader/fileDesc/title") {
        *title = tei_serde::json::Value::String("   ".to_owned());
    }

    let payload_text =
        tei_serde::json::to_string(&payload).context("serialising mutated JSON should succeed")?;
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
        .context("serialising to MessagePack should succeed")?;
    state.store_msgpack_payload(payload);
    Ok(())
}

#[when("I deserialize the MessagePack payload")]
fn i_deserialize_the_messagepack_payload(
    #[from(validated_state)] state: &SerdeState,
) -> Result<()> {
    let payload = state.msgpack_payload()?;
    match tei_serde::msgpack::from_slice::<TeiDocument>(payload.as_slice()) {
        Ok(decoded) => state.set_decoded_document(decoded),
        Err(error) => state.store_error(error.to_string()),
    }
    Ok(())
}

#[when("I serialize the document as JSON")]
fn i_serialize_the_document_as_json(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let document = state.document()?;
    let payload =
        tei_serde::json::to_string(&*document).context("serialising to JSON should succeed")?;
    state.store_json_payload(payload);
    Ok(())
}

#[when("I deserialize the JSON payload")]
fn i_deserialize_the_json_payload(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let payload = state.json_payload()?;
    match tei_serde::json::from_str::<TeiDocument>(&payload) {
        Ok(decoded) => state.set_decoded_document(decoded),
        Err(error) => state.store_error(error.to_string()),
    }
    Ok(())
}

#[then("the deserialized document title is \"{title}\"")]
fn the_deserialized_document_title_is(
    #[from(validated_state)] state: &SerdeState,
    title: String,
) -> Result<()> {
    let document = state.decoded_document()?;
    let expected_title = title;
    ensure!(
        document.title().as_str() == expected_title.as_str(),
        "expected title {expected_title:?}, found {:?}",
        document.title().as_str()
    );
    Ok(())
}

#[then("MessagePack deserialization fails")]
fn messagepack_deserialization_fails(#[from(validated_state)] state: &SerdeState) -> Result<()> {
    let message = state.error()?;
    ensure!(
        message.contains("IO error while reading marker"),
        "expected invalid marker read error, got: {message}"
    );
    Ok(())
}

#[then("JSON deserialization fails mentioning \"{snippet}\"")]
fn json_deserialization_fails_mentioning(
    #[from(validated_state)] state: &SerdeState,
    snippet: String,
) -> Result<()> {
    let message = state.error()?;
    ensure!(
        message.contains(&snippet),
        "expected error to mention {snippet:?}, found {message:?}"
    );
    Ok(())
}

#[then("JSON deserialization fails with a syntax error")]
fn json_deserialization_fails_with_a_syntax_error(
    #[from(validated_state)] state: &SerdeState,
) -> Result<()> {
    let message = state.error()?;
    ensure!(
        message.contains("expected value")
            || message.contains("expected ident")
            || message.contains("EOF while parsing"),
        "expected syntax error, got: {message}"
    );
    Ok(())
}

/// Scenario: Serialize a document to `MessagePack` and back.
#[scenario(path = "tests/features/serialization.feature", index = 0)]
pub fn serializes_messagepack_round_trip(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}

/// Scenario: Reject invalid `MessagePack` payloads.
#[scenario(path = "tests/features/serialization.feature", index = 1)]
pub fn rejects_invalid_messagepack_payloads(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}

/// Scenario: Serialize a document to JSON and back.
#[scenario(path = "tests/features/serialization.feature", index = 2)]
pub fn serializes_json_round_trip(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}

/// Scenario: Reject JSON payloads with blank titles.
#[scenario(path = "tests/features/serialization.feature", index = 3)]
pub fn rejects_blank_titles_in_json(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}

/// Scenario: Reject invalid JSON payloads.
#[scenario(path = "tests/features/serialization.feature", index = 4)]
pub fn rejects_invalid_json_payloads(
    #[from(validated_state)] _: SerdeState,
    #[from(validated_state_result)] result: Result<SerdeState>,
) {
    expect_validated_state(result, "serialization");
}
