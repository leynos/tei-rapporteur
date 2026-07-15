//! Shared harness for the streaming BDD scenarios: the captured parser state,
//! the named XML fixtures, and the rstest fixtures that seed each scenario.

use anyhow::{Context, ensure};
use rstest::fixture;
use std::cell::RefCell;
use tei_core::{TeiError, TeiHeader};
use tei_xml::streaming::TeiEvent;

pub(crate) const MINIMAL_FIXTURE: &str = concat!(
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

pub(crate) const PARAGRAPHS_FIXTURE: &str = concat!(
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

pub(crate) const UTTERANCES_FIXTURE: &str = concat!(
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

pub(crate) const EMPHASIS_FIXTURE: &str = concat!(
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

pub(crate) const PAUSE_FIXTURE: &str = concat!(
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

pub(crate) const UNTERMINATED_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Broken</title>",
    "</fileDesc>",
);

pub(crate) const MISSING_HEADER_FIXTURE: &str =
    concat!("<TEI>", "<text>", "<body/>", "</text>", "</TEI>",);

pub(crate) const CDATA_FIXTURE: &str = concat!(
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

pub(crate) const EOF_AFTER_BODY_FIXTURE: &str = concat!(
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

pub(crate) const NAMESPACE_FIXTURE: &str = concat!(
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

pub(crate) type EventResult = Result<TeiEvent, TeiError>;

#[derive(Default)]
pub(crate) struct StreamingState {
    xml: RefCell<Option<String>>,
    events: RefCell<Vec<EventResult>>,
    header_event: RefCell<Option<TeiHeader>>,
    parser_header: RefCell<Option<TeiHeader>>,
    last_error: RefCell<Option<TeiError>>,
    header_was_none_before: RefCell<bool>,
}

impl StreamingState {
    pub(crate) fn set_xml(&self, xml: &str) {
        *self.xml.borrow_mut() = Some(xml.to_owned());
    }

    pub(crate) fn xml(&self) -> anyhow::Result<String> {
        self.xml
            .borrow()
            .as_ref()
            .cloned()
            .context("scenario must supply XML input")
    }

    pub(crate) fn push_event(&self, event: EventResult) {
        self.events.borrow_mut().push(event);
    }

    pub(crate) fn events(&self) -> Vec<EventResult> {
        self.events.borrow().clone()
    }

    pub(crate) fn set_header_event(&self, header: TeiHeader) {
        *self.header_event.borrow_mut() = Some(header);
    }

    pub(crate) fn header_event(&self) -> anyhow::Result<TeiHeader> {
        self.header_event
            .borrow()
            .clone()
            .context("header event not captured")
    }

    pub(crate) fn set_parser_header(&self, header: TeiHeader) {
        *self.parser_header.borrow_mut() = Some(header);
    }

    pub(crate) fn parser_header(&self) -> anyhow::Result<TeiHeader> {
        self.parser_header
            .borrow()
            .clone()
            .context("parser header not captured")
    }

    pub(crate) fn set_error(&self, error: TeiError) {
        *self.last_error.borrow_mut() = Some(error);
    }

    pub(crate) fn set_header_was_none_before(&self, was_none: bool) {
        *self.header_was_none_before.borrow_mut() = was_none;
    }

    pub(crate) fn header_was_none_before(&self) -> bool {
        *self.header_was_none_before.borrow()
    }
}

pub(crate) fn fixture_by_name(name: &str) -> anyhow::Result<&'static str> {
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
        other => anyhow::bail!("unknown TEI fixture: {other}"),
    }
}

#[fixture]
pub(crate) fn validated_state_result() -> anyhow::Result<StreamingState> {
    let state = StreamingState::default();
    ensure!(state.xml.borrow().is_none(), "xml slot must start empty");
    ensure!(
        state.events.borrow().is_empty(),
        "events slot must start empty"
    );
    Ok(state)
}

#[fixture]
pub(crate) fn validated_state() -> StreamingState {
    match validated_state_result() {
        Ok(state) => state,
        Err(error) => panic!("failed to initialize streaming state: {error}"),
    }
}
