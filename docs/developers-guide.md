# Developers guide

This guide records internally facing conventions that are not part of the
public user API.

## Spoken text extraction

Spoken-runtime extraction follows Architectural Decision Record (ADR) 006 and
is owned by Rust. Keep the public segment value types and normalization helpers
in `tei-core`, keep XML traversal and profile checks in `tei-xml`, and keep
Python as a thin PyO3 adapter. Python code must not reimplement TEI spoken-text
semantics or parse XML locally.

The current `tei_xml::spoken_text_segments` implementation is a streaming
adapter over `quick-xml`. It returns shared `tei_core::SpokenTextSegment`
values and enforces a narrow spoken-runtime body profile while the broader
canonical `TeiDocument` model is extended in a later milestone. When adding or
changing included, excluded, or silent-boundary elements, update the predicate
helpers in `tei-xml/src/spoken/predicates.rs`, the XML behaviour tests in
`tei-xml/tests/spoken_text.rs`, the Python binding tests, and the user guide in
the same change.

Excluded inline elements and silent markers must be represented as boundaries
that contribute no words. Do not count a nested `<seg>` twice: inline
segmentation contributes to its enclosing spoken block unless it is standalone
in a spoken context.

## XML attribute extraction

`tei-xml/src/attributes.rs` owns shared `quick-xml` attribute extraction for
the spoken-text extractor and the streaming parser. Use
`extract_normalized_attribute` for isolated lookups and
`NormalizedAttributes::from_element` when a handler needs several attributes
from the same `BytesStart`. The latter collects attributes once and serves
repeated lookups from an element-local cache, which keeps hot parser paths from
re-scanning the same start tag for each field.

The helper normalizes attributes with `XmlVersion::Implicit1_0`. The parser
states currently do not retain the XML declaration beside start-element events,
and XML 1.0 is the specification default when a declaration is absent. If the
parsers later preserve XML version state, thread that version into this module
first so spoken-text and streaming behaviour remain aligned.

## Behaviour scenario binding

Prefer `rstest-bdd` name-based scenario binding for behaviour tests. Use the
exact Gherkin `Scenario:` title in each `#[scenario(..., name = "...")]`
attribute so Rust test functions are independent of feature-file ordering. Do
not add new index-based bindings unless an upstream limitation makes them
unavoidable and the same change documents the limitation.

When splitting one feature across several Rust modules, make the module
relationship explicit in module documentation. The parent module should state
which child modules extend its fixtures or shared state, and child modules
should state which parent module they serve.

## Spoken text observability

`tei_xml::spoken_text_segments` emits `tracing` debug events only. Library
callers own subscriber installation and export. The event schema is stable
enough for benchmark and diagnostic consumers:

- `spoken_text_parse_started`: includes `input_bytes`.
- `spoken_text_element_enter`: includes `element`, `is_empty`, `phase`, and
  `stack_depth`.
- `spoken_text_phase_transition`: includes `from` and `to`.
- `spoken_text_phase_rejected`: includes `phase`, `next`, and `error`.
- `spoken_text_unsupported_body_element`: includes `element`, `phase`, and
  `stack_depth`.
- `spoken_text_segment_started`: includes `element`, `kind`, `locator`, and
  `has_xml_id`.
- `spoken_text_segment_suppressed`: includes `element` and `locator`.
- `spoken_text_segment_emitted`: includes `element`, `locator`, and
  `text_bytes`.
- `spoken_text_parse_finished`: includes `input_bytes`, `segment_count`, and
  `elapsed_microseconds`.
- `spoken_text_parse_error`: includes `error`; parser-state failures also
  include `phase` and `stack_depth`.

Treat `input_bytes`, `segment_count`, `text_bytes`, and `elapsed_microseconds`
as the metrics surface for spoken extraction. Throughput dashboards should
derive rates from those fields rather than adding counters in the library.
