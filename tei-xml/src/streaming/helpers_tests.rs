//! Unit tests for streaming attribute extraction helpers.

use quick_xml::events::BytesStart;
use rstest::{fixture, rstest};
use tei_core::TeiError;

use super::helpers::{
    extract_div_attrs, extract_item_attrs, extract_pause_attrs, extract_utterance_attrs,
};

#[derive(Clone, Copy)]
enum HelperKind {
    Utterance,
    Div,
    Item,
    Pause,
}

#[derive(Clone, Copy)]
struct ElementAttrs {
    tag: &'static str,
    attrs: &'static [(&'static str, &'static str)],
}

#[fixture]
fn element_with_attrs(
    #[default(ElementAttrs { tag: "u", attrs: &[] })] input: ElementAttrs,
) -> BytesStart<'static> {
    let ElementAttrs { tag, attrs } = input;
    let mut el = BytesStart::new(tag);
    for attr in attrs {
        el.push_attribute(*attr);
    }
    el
}

fn assert_utterance_attrs(element: &BytesStart<'_>, expected: &[(&str, Option<&str>)]) {
    let attrs = extract_utterance_attrs(element).expect("failed to extract utterance attrs");
    for (field, value) in expected {
        let actual = match *field {
            "xml:id" => attrs.id.as_deref(),
            "n" => attrs.n.as_deref(),
            "who" => attrs.who.as_deref(),
            "source" => attrs.source.as_deref(),
            "resp" => attrs.resp.as_deref(),
            "cert" => attrs.cert.as_deref(),
            "corresp" => attrs.corresp.as_deref(),
            "ana" => attrs.ana.as_deref(),
            other => panic!("unknown utterance field: {other}"),
        };
        assert_eq!(actual, *value, "unexpected utterance @{field}");
    }
}

fn assert_div_attrs(element: &BytesStart<'_>, expected: &[(&str, Option<&str>)]) {
    let attrs = extract_div_attrs(element, None).expect("failed to extract div attrs");
    for (field, value) in expected {
        let actual = match *field {
            "type" => Some(attrs.div_type.as_str()),
            "subtype" => attrs.subtype.as_deref(),
            "xml:id" => attrs.id.as_deref(),
            "head" => attrs.head.as_ref().map(|_| "<head>"),
            other => panic!("unknown div field: {other}"),
        };
        assert_eq!(actual, *value, "unexpected div @{field}");
    }
}

fn assert_item_attrs(element: &BytesStart<'_>, expected: &[(&str, Option<&str>)]) {
    let attrs = extract_item_attrs(element, None).expect("failed to extract item attrs");
    for (field, value) in expected {
        let actual = match *field {
            "xml:id" => attrs.id.as_deref(),
            "n" => attrs.n.as_deref(),
            "corresp" => attrs.corresp.as_deref(),
            "label" => attrs.label.as_ref().map(|_| "<label>"),
            other => panic!("unknown item field: {other}"),
        };
        assert_eq!(actual, *value, "unexpected item @{field}");
    }
}

fn assert_pause_attrs(element: &BytesStart<'_>, expected: &[(&str, Option<&str>)]) {
    let (dur, pause_type) = extract_pause_attrs(element).expect("failed to extract pause attrs");
    for (field, value) in expected {
        let actual = match *field {
            "dur" => dur.as_deref(),
            "type" => pause_type.as_deref(),
            other => panic!("unknown pause field: {other}"),
        };
        assert_eq!(actual, *value, "unexpected pause @{field}");
    }
}

fn assert_attrs_for(
    helper: HelperKind,
    element: &BytesStart<'_>,
    expected: &[(&str, Option<&str>)],
) {
    match helper {
        HelperKind::Utterance => assert_utterance_attrs(element, expected),
        HelperKind::Div => assert_div_attrs(element, expected),
        HelperKind::Item => assert_item_attrs(element, expected),
        HelperKind::Pause => assert_pause_attrs(element, expected),
    }
}

#[rstest]
#[case::utterance_all_fields(
    HelperKind::Utterance,
    ElementAttrs {
        tag: "u",
        attrs: &[
            ("xml:id", "u1"),
            ("n", "1"),
            ("who", "#speaker1"),
            ("source", "#src1"),
            ("resp", "#resp1"),
            ("cert", "high"),
            ("corresp", "#u2"),
            ("ana", "#ana1"),
        ],
    },
    &[
        ("xml:id", Some("u1")),
        ("n", Some("1")),
        ("who", Some("#speaker1")),
        ("source", Some("#src1")),
        ("resp", Some("#resp1")),
        ("cert", Some("high")),
        ("corresp", Some("#u2")),
        ("ana", Some("#ana1")),
    ]
)]
#[case::utterance_absent_fields(
    HelperKind::Utterance,
    ElementAttrs { tag: "u", attrs: &[] },
    &[
        ("xml:id", None),
        ("n", None),
        ("who", None),
        ("source", None),
        ("resp", None),
        ("cert", None),
        ("corresp", None),
        ("ana", None),
    ]
)]
#[case::div_required_and_optional_fields(
    HelperKind::Div,
    ElementAttrs {
        tag: "div",
        attrs: &[("type", "interview"), ("subtype", "formal"), ("xml:id", "d1")],
    },
    &[
        ("type", Some("interview")),
        ("subtype", Some("formal")),
        ("xml:id", Some("d1")),
        ("head", None),
    ]
)]
#[case::div_optional_fields_absent(
    HelperKind::Div,
    ElementAttrs {
        tag: "div",
        attrs: &[("type", "session")],
    },
    &[("type", Some("session")), ("subtype", None), ("xml:id", None)]
)]
#[case::item_all_optional_fields(
    HelperKind::Item,
    ElementAttrs {
        tag: "item",
        attrs: &[("xml:id", "i1"), ("n", "42"), ("corresp", "#i2")],
    },
    &[
        ("xml:id", Some("i1")),
        ("n", Some("42")),
        ("corresp", Some("#i2")),
        ("label", None),
    ]
)]
#[case::item_absent_fields(
    HelperKind::Item,
    ElementAttrs { tag: "item", attrs: &[] },
    &[("xml:id", None), ("n", None), ("corresp", None), ("label", None)]
)]
#[case::pause_both_fields(
    HelperKind::Pause,
    ElementAttrs {
        tag: "pause",
        attrs: &[("dur", "PT1S"), ("type", "short")],
    },
    &[("dur", Some("PT1S")), ("type", Some("short"))]
)]
#[case::pause_absent_fields(
    HelperKind::Pause,
    ElementAttrs { tag: "pause", attrs: &[] },
    &[("dur", None), ("type", None)]
)]
fn helper_attrs_match_expected_fields(
    #[case] helper: HelperKind,
    #[case] input: ElementAttrs,
    #[case] expected: &[(&str, Option<&str>)],
    #[from(element_with_attrs)]
    #[with(input)]
    element: BytesStart<'static>,
) {
    let _ = input;
    assert_attrs_for(helper, &element, expected);
}

#[rstest]
#[case::utterance(HelperKind::Utterance, r#"u who="&badentity;""#, 1)]
#[case::div(HelperKind::Div, r#"div type="&badentity;""#, 3)]
#[case::item(HelperKind::Item, r#"item n="&badentity;""#, 4)]
#[case::pause(HelperKind::Pause, r#"pause dur="&badentity;""#, 5)]
fn unknown_entity_errors_are_forwarded(
    #[case] helper: HelperKind,
    #[case] content: &str,
    #[case] tag_len: usize,
) {
    let el = BytesStart::from_content(content, tag_len);
    let error = extract_attrs_for(helper, &el).expect_err("attribute extraction should fail");

    assert!(
        error
            .to_string()
            .contains("unrecognized entity `badentity`"),
        "expected unrecognized entity error, got: {error}"
    );
}

fn extract_attrs_for(helper: HelperKind, element: &BytesStart<'_>) -> Result<(), TeiError> {
    match helper {
        HelperKind::Utterance => extract_utterance_attrs(element).map(|_| ()),
        HelperKind::Div => extract_div_attrs(element, None).map(|_| ()),
        HelperKind::Item => extract_item_attrs(element, None).map(|_| ()),
        HelperKind::Pause => extract_pause_attrs(element).map(|_| ()),
    }
}

#[rstest]
#[case::missing_div_type(HelperKind::Div, BytesStart::new("div"), "missing required")]
#[case::duplicate_utterance(
    HelperKind::Utterance,
    BytesStart::from_content(r#"u who="speaker" who="duplicate""#, 1),
    "duplicated attribute"
)]
#[case::duplicate_div(
    HelperKind::Div,
    BytesStart::from_content(r#"div type="section" type="duplicate""#, 3),
    "duplicated attribute"
)]
#[case::duplicate_item(
    HelperKind::Item,
    BytesStart::from_content(r#"item n="1" n="duplicate""#, 4),
    "duplicated attribute"
)]
#[case::duplicate_pause(
    HelperKind::Pause,
    BytesStart::from_content(r#"pause dur="PT1S" dur="duplicate""#, 5),
    "duplicated attribute"
)]
fn attribute_iteration_errors_are_forwarded(
    #[case] helper: HelperKind,
    #[case] el: BytesStart<'_>,
    #[case] expected_error: &str,
) {
    let result = extract_attrs_for(helper, &el);

    let error = result.expect_err("attribute extraction should fail");

    assert!(error.to_string().contains(expected_error));
}
