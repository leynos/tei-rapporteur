# User's guide

The `tei-rapporteur` workspace currently focuses on establishing the crate
layout that underpins the rest of the roadmap. This guide summarizes what is
available today and how to exercise it.

## Workspace overview

- `tei-core` now models the top-level `TeiDocument` together with its
  `TeiHeader` and body-aware `TeiText`. The text model records ordered
  paragraphs (`P`) and utterances with optional speaker references. Each block
  stores a sequence of `Inline` nodes, allowing clients to mix plain text with
  emphasised `<hi>` spans and `<pause/>` cues without hand-rolling XML. Plain
  strings flow through the new `P::from_text_segments` and
  `Utterance::from_text_segments` helpers; the older `new` constructors remain
  as deprecated shims for existing callers.
- `tei-xml` depends on the core crate and now covers both directions of XML
  flow. `serialize_document_title(raw_title)` still emits a `<title>` snippet,
  `parse_xml(xml)` wraps `quick-xml` to materialize full `TeiDocument` values,
  and the new `emit_xml(&document)` helper uses `quick_xml::se::to_string` to
  produce canonical TEI strings. All helpers return `TeiError`, so callers see
  consistent diagnostics whether parsing malformed input or attempting to emit
  control characters that XML forbids.
- `tei-py` now ships the `tei_rapporteur` PyO3 module. The exported `Document`
  class wraps `TeiDocument`, validates titles via the Rust constructors, and
  exposes a `title` getter plus an `emit_title_markup` convenience method. The
  module also surfaces a top-level `emit_title_markup` function so Python
  callers mirror the Rust helper without reimplementing validation rules.
- `tei-test-helpers` captures assertion helpers that multiple crates reuse in
  their unit and behaviour-driven tests.
- `pyproject.toml` configures `maturin` to build `tei-py`, allowing
  `maturin develop` or `maturin build` to work from the workspace root without
  additional arguments.

## Building and testing

Use the Makefile targets to work with the entire workspace:

- `make build` compiles every crate in debug mode.
- `make test` runs all unit tests and the behaviour tests powered by
  `rstest-bdd`.
- `make check-fmt`, `make lint`, and `make fmt` mirror the repository quality
  gates described in `AGENTS.md`.

## Behavioural guarantees

`tei-core` and `tei-xml` ship behaviour-driven tests that exercise happy and
unhappy paths. Core scenarios validate that header metadata can be assembled,
that blank revision notes are rejected, and that the body model preserves
paragraph/utterance order while rejecting empty utterances. Additional cases
demonstrate inline emphasis, rend-aware mixed content, pause cues with duration
metadata, and ensure empty `<hi>` segments are rejected. The XML crate now
tests title serialization, full-document parsing, and XML emission: feature
files cover successful parsing, missing header errors, syntax failures
triggered by truncated documents, as well as emission of canonical minimal TEI
output and the error surfaced when a document sneaks in forbidden control
characters. These tests run alongside the unit suite, so developers receive
fast feedback when modifying the scaffolding. The new `tei-py` suite adds
`rstest-bdd` scenarios for the Python module, covering successful construction
of `Document` from a valid title, rejection of blank titles via `ValueError`,
and round-tripping markup through the module-level helper.

## Python bindings

The workspace now provides a ready-to-build Python wheel. `pyproject.toml`
declares `maturin` as the build backend and targets `tei-py/Cargo.toml`, so the
workflow looks like:

```bash
python -m pip install --upgrade pip maturin
maturin develop  # builds and installs tei_rapporteur into the active venv
python -c "import tei_rapporteur as tr; print(tr.Document('Wolf 359').title)"
```

Within Python, `tei_rapporteur.Document` constructs a validated TEI document by
wrapping the Rust `TeiDocument`. The class exposes a `.title` property and an
`emit_title_markup()` method that mirrors the Rust helper. The module also
offers a top-level `emit_title_markup(title: str)` so scripting callers can
work without instantiating a document. CI now builds the wheel on Ubuntu,
installs it via `pip`, and imports the module to ensure the PyO3 glue remains
healthy.

Binary interchange is now supported through
`tei_rapporteur.from_msgpack(payload: bytes)`. The helper accepts the bytes
produced by `msgspec.msgpack.encode` (or any compatible encoder), decodes them
via `rmp_serde`, and returns a `Document`. Invalid payloads raise `ValueError`,
so Python callers receive a familiar exception instead of a Rust-specific error
type. This allows workflows such as:

```python
import msgspec
import tei_rapporteur as tei

episode = Episode(title="Bridgewater")  # msgspec.Struct
payload = msgspec.msgpack.encode(episode)
document = tei.from_msgpack(payload)
print(document.title)
```

The BDD tests now cover both successful decoding and error handling, ensuring
the MessagePack entry point remains reliable as the API expands.
