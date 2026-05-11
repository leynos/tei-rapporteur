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
