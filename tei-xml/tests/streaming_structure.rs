//! Structural streaming coverage: divisions, nested divisions, and entity
//! references emitted by the streaming pull parser. These exercise the public
//! parser API directly, so they stay independent of the BDD harness in
//! `streaming_behaviour.rs`.

#![cfg(feature = "streaming")]

use anyhow::{Context, bail, ensure};
use tei_core::{BodyBlock, Inline};
use tei_xml::streaming::{TeiEvent, TeiPullParser};

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
