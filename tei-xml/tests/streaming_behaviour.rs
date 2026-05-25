//! Behaviour-driven scenarios that cover the streaming TEI pull parser.

#![cfg(feature = "streaming")]

use anyhow::{Context, bail, ensure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::cell::RefCell;
use tei_core::{BodyBlock, Inline, TeiError, TeiHeader};
use tei_test_helpers::expect_validated_state;
use tei_xml::streaming::{TeiEvent, TeiPullParser};

// Force Cargo to recompile the test binary when the feature file changes so the
// embedded scenarios stay in sync with expectations.
const _: &str = include_str!("features/streaming.feature");

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

const PARAGRAPHS_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<p>First paragraph</p>",
    "<p>Second paragraph</p>",
    "</body>",
    "</text>",
    "</TEI>",
);

const UTTERANCES_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<u xml:id=\"u1\" who=\"host\">Welcome to the show</u>",
    "<u xml:id=\"u2\" who=\"guest\">Thanks for having me</u>",
    "</body>",
    "</text>",
    "</TEI>",
);

const EMPHASIS_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<p>This is <hi>important</hi> text</p>",
    "</body>",
    "</text>",
    "</TEI>",
);

const PAUSE_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<u who=\"host\">Wait<pause dur=\"PT1S\"/> for it</u>",
    "</body>",
    "</text>",
    "</TEI>",
);

const UNTERMINATED_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Broken</title>",
    "</fileDesc>",
);

const MISSING_HEADER_FIXTURE: &str = concat!("<TEI>", "<text>", "<body/>", "</text>", "</TEI>",);

const CDATA_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<p>Before <![CDATA[raw <content>]]> after</p>",
    "</body>",
    "</text>",
    "</TEI>",
);

const EOF_AFTER_BODY_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<p>Content</p>",
    "</body>",
);

const NAMESPACE_FIXTURE: &str = concat!(
    "<TEI xmlns=\"http://www.tei-c.org/ns/1.0\">",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Namespaced Document</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<p>Content with namespace</p>",
    "</body>",
    "</text>",
    "</TEI>",
);

type EventResult = Result<TeiEvent, TeiError>;

#[derive(Default)]
struct StreamingState {
    xml: RefCell<Option<String>>,
    events: RefCell<Vec<EventResult>>,
    header_event: RefCell<Option<TeiHeader>>,
    parser_header: RefCell<Option<TeiHeader>>,
    last_error: RefCell<Option<TeiError>>,
    header_was_none_before: RefCell<bool>,
}

impl StreamingState {
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

    fn push_event(&self, event: EventResult) {
        self.events.borrow_mut().push(event);
    }

    fn events(&self) -> Vec<EventResult> {
        self.events.borrow().clone()
    }

    fn set_header_event(&self, header: TeiHeader) {
        *self.header_event.borrow_mut() = Some(header);
    }

    fn header_event(&self) -> anyhow::Result<TeiHeader> {
        self.header_event
            .borrow()
            .clone()
            .context("header event not captured")
    }

    fn set_parser_header(&self, header: TeiHeader) {
        *self.parser_header.borrow_mut() = Some(header);
    }

    fn parser_header(&self) -> anyhow::Result<TeiHeader> {
        self.parser_header
            .borrow()
            .clone()
            .context("parser header not captured")
    }

    fn set_error(&self, error: TeiError) {
        *self.last_error.borrow_mut() = Some(error);
    }

    fn set_header_was_none_before(&self, was_none: bool) {
        *self.header_was_none_before.borrow_mut() = was_none;
    }

    fn header_was_none_before(&self) -> bool {
        *self.header_was_none_before.borrow()
    }
}

fn fixture_by_name(name: &str) -> anyhow::Result<&'static str> {
    match name {
        "minimal" => Ok(MINIMAL_FIXTURE),
        "paragraphs" => Ok(PARAGRAPHS_FIXTURE),
        "utterances" => Ok(UTTERANCES_FIXTURE),
        "emphasis" => Ok(EMPHASIS_FIXTURE),
        "pause" => Ok(PAUSE_FIXTURE),
        "unterminated" => Ok(UNTERMINATED_FIXTURE),
        "missing-header" => Ok(MISSING_HEADER_FIXTURE),
        "cdata" => Ok(CDATA_FIXTURE),
        "eof-after-body" => Ok(EOF_AFTER_BODY_FIXTURE),
        "namespace" => Ok(NAMESPACE_FIXTURE),
        other => bail!("unknown TEI fixture: {other}"),
    }
}

#[fixture]
fn validated_state_result() -> anyhow::Result<StreamingState> {
    let state = StreamingState::default();
    ensure!(state.xml.borrow().is_none(), "xml slot must start empty");
    ensure!(
        state.events.borrow().is_empty(),
        "events slot must start empty"
    );
    Ok(state)
}

#[fixture]
fn validated_state() -> StreamingState {
    match validated_state_result() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialise streaming state: {error}"),
    }
}

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

#[then("the header title is \"{title}\"")]
fn the_header_title_is(
    #[from(validated_state)] state: &StreamingState,
    title: String,
) -> anyhow::Result<()> {
    let events = state.events();
    for event in events {
        if let Ok(TeiEvent::Header(header)) = event {
            let actual = header.file_desc().title().as_str();
            ensure!(
                actual == title,
                "header title mismatch: expected {title:?}, found {actual:?}"
            );
            return Ok(());
        }
    }
    bail!("no header event found");
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

#[scenario(path = "tests/features/streaming.feature", index = 0)]
fn parse_minimal_incrementally(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 1)]
fn yield_paragraphs_as_body_blocks(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 2)]
fn yield_utterances_with_speaker(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 3)]
fn parse_inline_emphasis(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 4)]
fn parse_pause_markers(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 5)]
fn header_accessible_after_parsing(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 6)]
fn report_malformed_xml(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 7)]
fn report_missing_header(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 8)]
fn handle_cdata_in_body(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 9)]
fn handle_eof_after_body(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 10)]
fn header_is_none_before_header_event(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

#[scenario(path = "tests/features/streaming.feature", index = 11)]
fn parse_namespaced_tei_document_correctly(
    #[from(validated_state)] _: StreamingState,
    #[from(validated_state_result)] result: anyhow::Result<StreamingState>,
) {
    expect_validated_state(result, "streaming");
}

const DIV_WITH_LIST_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"chapter\">",
    "<p>Intro text</p>",
    "<u who=\"host\">Welcome</u>",
    "<list>",
    "<item><label>1.</label>First item</item>",
    "<item n=\"2\">Second item</item>",
    "</list>",
    "</div>",
    "</body>",
    "</text>",
    "</TEI>",
);

const NESTED_DIV_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"segment\" subtype=\"chapter-markers\" xml:id=\"seg1\">",
    "<head>Chapter markers</head>",
    "<div type=\"segment\" subtype=\"chapter-marker\" xml:id=\"ch1\">",
    "<head>Cold open</head>",
    "<u who=\"host\">Welcome back.</u>",
    "</div>",
    "</div>",
    "</body>",
    "</text>",
    "</TEI>",
);

#[test]
fn streams_div_with_paragraphs_and_list_items() -> anyhow::Result<()> {
    use tei_core::{Div, DivContent};

    let parser = TeiPullParser::from_str(DIV_WITH_LIST_FIXTURE);

    let mut divs: Vec<Div> = Vec::new();
    for event in parser {
        if let TeiEvent::BodyBlock(BodyBlock::Div(div)) = event? {
            divs.push(div);
        }
    }

    ensure!(divs.len() == 1, "expected exactly one Div body block");

    let div = divs.first().expect("should have at least one div");
    ensure!(div.div_type() == "chapter", "expected div type 'chapter'");
    ensure!(div.content().len() == 3, "expected 3 div children");

    // First child: paragraph
    let Some(DivContent::Paragraph(p)) = div.content().first() else {
        bail!("expected paragraph as first div child");
    };
    ensure!(
        p.content()
            .iter()
            .any(|i| matches!(i, Inline::Text(t) if t == "Intro text")),
        "paragraph should contain 'Intro text'"
    );

    // Second child: utterance
    let Some(DivContent::Utterance(u)) = div.content().get(1) else {
        bail!("expected utterance as second div child");
    };
    ensure!(u.speaker().is_some(), "utterance should have a speaker");

    // Third child: list with labelled items
    let Some(DivContent::List(list)) = div.content().get(2) else {
        bail!("expected list as third div child");
    };
    ensure!(list.items().len() == 2, "list should have 2 items");

    let first_item = list.items().first().expect("list should have a first item");
    let label = first_item
        .label()
        .context("first item should have a label")?;
    ensure!(
        label
            .content()
            .iter()
            .any(|i| matches!(i, Inline::Text(t) if t == "1.")),
        "label should contain '1.'"
    );
    ensure!(
        first_item
            .content()
            .iter()
            .any(|i| matches!(i, Inline::Text(t) if t == "First item")),
        "first item should contain 'First item'"
    );

    let second_item = list.items().get(1).expect("list should have a second item");
    ensure!(
        second_item.n() == Some("2"),
        "second item should have n='2'"
    );
    ensure!(
        second_item
            .content()
            .iter()
            .any(|i| matches!(i, Inline::Text(t) if t == "Second item")),
        "second item should contain 'Second item'"
    );

    Ok(())
}

#[test]
fn streams_nested_divs_with_headings_and_subtypes() -> anyhow::Result<()> {
    use tei_core::DivContent;

    let parser = TeiPullParser::from_str(NESTED_DIV_FIXTURE);

    let mut divs = Vec::new();
    for event in parser {
        if let TeiEvent::BodyBlock(BodyBlock::Div(div)) = event? {
            divs.push(div);
        }
    }

    ensure!(
        divs.len() == 1,
        "expected exactly one top-level Div body block"
    );
    let div = divs.first().expect("should have a top-level div");
    ensure!(div.div_type() == "segment", "expected top-level div type");
    ensure!(
        div.subtype() == Some("chapter-markers"),
        "expected top-level div subtype"
    );
    ensure!(
        div.head().is_some_and(|head| {
            head.content()
                .iter()
                .any(|inline| matches!(inline, Inline::Text(text) if text == "Chapter markers"))
        }),
        "expected top-level heading text"
    );

    let Some(DivContent::Div(child)) = div.content().first() else {
        bail!("expected nested div as first child");
    };
    ensure!(child.div_type() == "segment", "expected nested div type");
    ensure!(
        child.subtype() == Some("chapter-marker"),
        "expected nested div subtype"
    );
    ensure!(
        child.head().is_some_and(|head| {
            head.content()
                .iter()
                .any(|inline| matches!(inline, Inline::Text(text) if text == "Cold open"))
        }),
        "expected nested heading text"
    );
    ensure!(
        matches!(child.content().first(), Some(DivContent::Utterance(_))),
        "expected nested utterance content"
    );

    Ok(())
}

const ENTITY_REF_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<p>Rock &amp; Roll</p>",
    "<u who=\"host\">1 &lt; 2 &amp; 3 &gt; 0</u>",
    "</body>",
    "</text>",
    "</TEI>",
);

#[test]
fn preserves_entity_references_in_streamed_content() -> anyhow::Result<()> {
    let parser = TeiPullParser::from_str(ENTITY_REF_FIXTURE);

    let mut blocks: Vec<BodyBlock> = Vec::new();
    for event in parser {
        if let TeiEvent::BodyBlock(block) = event? {
            blocks.push(block);
        }
    }

    ensure!(blocks.len() == 2, "expected 2 body blocks");

    let BodyBlock::Paragraph(p) = blocks.first().expect("should have first block") else {
        bail!("expected paragraph as first block");
    };
    let paragraph_text: String = p
        .content()
        .iter()
        .filter_map(|i| match i {
            Inline::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    ensure!(
        paragraph_text == "Rock & Roll",
        "paragraph should contain 'Rock & Roll', found '{paragraph_text}'"
    );

    let BodyBlock::Utterance(u) = blocks.get(1).expect("should have second block") else {
        bail!("expected utterance as second block");
    };
    let utterance_text: String = u
        .content()
        .iter()
        .filter_map(|i| match i {
            Inline::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    ensure!(
        utterance_text == "1 < 2 & 3 > 0",
        "utterance should contain '1 < 2 & 3 > 0', found '{utterance_text}'"
    );

    Ok(())
}
