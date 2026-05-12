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

## tei-py build requirements

`tei-py` uses PyO3 for the Python binding layer. The current supported PyO3
minor series is `0.24.x`, with `pyo3` pinned to `0.24.2` and the direct
`pyo3-build-config` build dependency pinned to `0.24.2`.

Keep `pyo3` and `pyo3-build-config` on the same minor series. PyO3 uses the
build configuration crate to resolve interpreter configuration and emit cfg
flags consumed by the runtime crate. Mixing minor series risks subtle build
configuration mismatches, including cfg flags or generated binding assumptions
that no longer match the runtime dependency.

The `pyo3` dependency enables `auto-initialize` so tests and Rust-side helpers
can attach to an embedded Python interpreter without requiring every caller to
perform explicit interpreter initialization first. The `pyo3-build-config`
dependency enables `resolve-config` so `tei-py/build.rs` can resolve the active
Python configuration and apply the PyO3 cfg values needed by the crate.

## tei-py test-support API

`tei-py/src/test_support.rs` contains the private `run_with_kwargs` helper used
by the `msgspec` bootstrap path:

```rust
fn run_with_kwargs<'py, A>(
    run: &Bound<'py, PyAny>,
    args: A,
    kwargs: &Bound<'py, PyDict>,
)
where
    A: pyo3::call::PyCallArgs<'py>,
```

Any future caller must pass an argument value that implements
`pyo3::call::PyCallArgs<'py>`. PyO3 0.24 tightened `PyAnyMethods::call` to use
`PyCallArgs` directly rather than accepting any value convertible through
`IntoPyObject<'py, Target = PyTuple>`. When the intended Python call receives
one positional argument, wrap that argument in a Rust one-tuple, such as
`(args_tuple,)`.

Only `ensure_msgspec_installed` and `msgspec_available` are public exports from
this module. They are thread-safe: `ensure_msgspec_installed` guards the
bootstrap with `Once`, and `msgspec_available` delegates to it while attached
to the Python interpreter.
