//! Step definitions shared by the streaming BDD scenarios. Steps register with
//! the global rstest-bdd registry, so they are resolved by their Gherkin text
//! regardless of which scenario module drives them.

use anyhow::{bail, ensure};
use rstest_bdd_macros::{given, then, when};
use tei_core::{BodyBlock, Inline};
use tei_xml::streaming::{TeiEvent, TeiPullParser};

use super::support::{StreamingState, fixture_by_name};

#[given("a streaming parser for the \"{fixture}\" TEI fixture")]
fn a_streaming_parser_for_fixture(
    #[from(validated_state)] state: &StreamingState,
    fixture: String,
) -> anyhow::Result<()> {
    let xml = fixture_by_name(&fixture)?;
    state.set_xml(xml);
    Ok(())
}

#[when("I collect all events")]
fn i_collect_all_events(#[from(validated_state)] state: &StreamingState) -> anyhow::Result<()> {
    let xml = state.xml()?;
    let parser = TeiPullParser::from_str(&xml);
    for event in parser {
        state.push_event(event);
    }
    Ok(())
}

#[when("I consume up to the Header event")]
fn i_consume_up_to_header(#[from(validated_state)] state: &StreamingState) -> anyhow::Result<()> {
    let xml = state.xml()?;
    let mut parser = TeiPullParser::from_str(&xml);

    for event in parser.by_ref() {
        match event {
            Ok(TeiEvent::Header(header)) => {
                state.set_header_event(header.clone());
                if let Some(h) = parser.header() {
                    state.set_parser_header(h.clone());
                }
                break;
            }
            Ok(_) => {}
            Err(e) => {
                state.set_error(e);
                break;
            }
        }
    }
    Ok(())
}

#[when("I request the next event after DocumentStart")]
fn i_request_next_event_after_start(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    let xml = state.xml()?;
    let mut parser = TeiPullParser::from_str(&xml);

    // Skip DocumentStart
    if let Some(Ok(TeiEvent::DocumentStart)) = parser.next() {
        // Get the next event
        if let Some(result) = parser.next() {
            state.push_event(result);
        }
    }
    Ok(())
}

#[when("I check header before the Header event")]
fn i_check_header_before_header_event(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    let xml = state.xml()?;
    let mut parser = TeiPullParser::from_str(&xml);

    // After DocumentStart but before Header event, header() should be None
    if let Some(Ok(TeiEvent::DocumentStart)) = parser.next() {
        // Check header before consuming the Header event
        state.set_header_was_none_before(parser.header().is_none());
    }
    Ok(())
}

#[then("the event sequence is \"{sequence}\"")]
fn the_event_sequence_is(
    #[from(validated_state)] state: &StreamingState,
    sequence: String,
) -> anyhow::Result<()> {
    let events = state.events();
    let event_names: Vec<&str> = events
        .iter()
        .filter_map(|e| {
            e.as_ref().ok().map(|ev| match ev {
                TeiEvent::DocumentStart => "DocumentStart",
                TeiEvent::Header(_) => "Header",
                TeiEvent::BodyBlock(_) => "BodyBlock",
                TeiEvent::DocumentEnd => "DocumentEnd",
            })
        })
        .collect();
    let actual = event_names.join(", ");
    ensure!(
        actual == sequence,
        "event sequence mismatch: expected {sequence:?}, found {actual:?}"
    );
    Ok(())
}

#[then("I receive {count:usize} BodyBlock events")]
fn i_receive_n_body_block_events(
    #[from(validated_state)] state: &StreamingState,
    count: usize,
) -> anyhow::Result<()> {
    let events = state.events();
    let body_block_count = events
        .iter()
        .filter(|e| matches!(e, Ok(TeiEvent::BodyBlock(_))))
        .count();
    ensure!(
        body_block_count == count,
        "body block count mismatch: expected {count}, found {body_block_count}"
    );
    Ok(())
}

#[then("each BodyBlock is a Paragraph")]
fn each_body_block_is_a_paragraph(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    let events = state.events();
    for event in events {
        if let Ok(TeiEvent::BodyBlock(block)) = event {
            ensure!(
                matches!(block, BodyBlock::Paragraph(_)),
                "expected Paragraph, found Utterance"
            );
        }
    }
    Ok(())
}

#[then("each BodyBlock is an Utterance with a speaker")]
fn each_body_block_is_an_utterance_with_speaker(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    let events = state.events();
    let mut utterance_count = 0;
    for event in events {
        if let Ok(TeiEvent::BodyBlock(block)) = event {
            match block {
                BodyBlock::Utterance(u) => {
                    ensure!(u.speaker().is_some(), "utterance should have a speaker");
                    // Verify xml:id is set
                    ensure!(u.id().is_some(), "utterance should have an xml:id");
                    utterance_count += 1;
                }
                BodyBlock::Paragraph(_) => bail!("expected Utterance, found Paragraph"),
                BodyBlock::Div(_) => bail!("expected Utterance, found Div"),
            }
        }
    }
    ensure!(
        utterance_count == 2,
        "expected 2 utterances, found {utterance_count}"
    );
    Ok(())
}

#[then("the first paragraph contains emphasis")]
fn the_first_paragraph_contains_emphasis(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    let events = state.events();
    for event in events {
        if let Ok(TeiEvent::BodyBlock(BodyBlock::Paragraph(p))) = event {
            let has_emphasis = p.content().iter().any(|i| matches!(i, Inline::Hi(_)));
            ensure!(has_emphasis, "paragraph should contain emphasis");
            return Ok(());
        }
    }
    bail!("no paragraph found");
}

#[then("the first utterance contains a pause")]
fn the_first_utterance_contains_a_pause(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    let events = state.events();
    for event in events {
        if let Ok(TeiEvent::BodyBlock(BodyBlock::Utterance(u))) = event {
            let has_pause = u.content().iter().any(|i| matches!(i, Inline::Pause(_)));
            ensure!(has_pause, "utterance should contain pause");
            return Ok(());
        }
    }
    bail!("no utterance found");
}

#[then("the parser header method returns the same header")]
fn the_parser_header_returns_same_header(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    let header_event = state.header_event()?;
    let parser_header = state.parser_header()?;
    ensure!(
        header_event == parser_header,
        "header mismatch between event and parser method"
    );
    Ok(())
}

#[then("an error is returned")]
fn an_error_is_returned(#[from(validated_state)] state: &StreamingState) -> anyhow::Result<()> {
    let events = state.events();
    let has_error = events.iter().any(Result::is_err);
    ensure!(has_error, "expected an error event");
    Ok(())
}

#[then("an error is returned mentioning \"{snippet}\"")]
fn an_error_is_returned_mentioning(
    #[from(validated_state)] state: &StreamingState,
    snippet: String,
) -> anyhow::Result<()> {
    let events = state.events();
    for event in events {
        if let Err(error) = event {
            let message = error.to_string();
            ensure!(
                message.contains(&snippet),
                "error should mention {snippet:?}, found {message:?}"
            );
            return Ok(());
        }
    }
    bail!("expected an error event mentioning {snippet:?}");
}

#[then("the parser header method returns None before the Header event")]
fn the_parser_header_returns_none_before(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    ensure!(
        state.header_was_none_before(),
        "parser.header() should return None before the Header event"
    );
    Ok(())
}

#[then("the first paragraph contains CDATA text")]
fn the_first_paragraph_contains_cdata_text(
    #[from(validated_state)] state: &StreamingState,
) -> anyhow::Result<()> {
    let events = state.events();
    for event in events {
        if let Ok(TeiEvent::BodyBlock(BodyBlock::Paragraph(p))) = event {
            // Check that the paragraph contains the CDATA content (raw <content>)
            let text: String = p
                .content()
                .iter()
                .filter_map(|i| match i {
                    Inline::Text(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect();
            ensure!(
                text.contains("raw <content>"),
                "paragraph should contain CDATA text 'raw <content>', found {text:?}"
            );
            return Ok(());
        }
    }
    bail!("no paragraph found");
}
