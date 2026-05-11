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
