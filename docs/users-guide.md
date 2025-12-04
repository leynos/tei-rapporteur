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
  as deprecated shims for existing callers. `TeiDocument` now exposes
  `validate()` to enforce document-wide rules: it rejects duplicate `xml:id`
  values across annotation systems, paragraphs, and utterances, and ensures
  utterance speakers appear in the profile cast when it exists. An empty cast
  still counts as declared—every `who` fails until the speakers are populated—
  whereas the absence of a cast allows speaker references so drafts can be
  validated incrementally. Identifier checks span the header as well, catching
  clashes between annotation systems and body blocks. Violations surface as
  `TeiError::Validation`.
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
  callers mirror the Rust helper without reimplementing validation rules. The
  MessagePack bridge exposes both `from_msgpack` and `to_msgpack` for binary
  interchange. Dictionary exchange is available via `from_dict`/`to_dict`,
  powered by `pyo3-serde`, so Python built-ins can cross the FFI boundary
  without detouring through JSON text. Phase 2.2 adds `parse_xml`/`emit_xml`
  bindings that forward TEI strings directly to the `tei-xml` helpers. Python
  can now parse canonical TEI without detouring through MessagePack, and
  emission always routes through the same forbidden-character guardrails as the
  Rust callers. Python-facing errors are surfaced as `ValueError` for content
  issues and `TypeError` when callers pass the wrong objects to the bindings.
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
fast feedback when modifying the scaffolding. The `tei-py` suite layers on
`rstest-bdd` scenarios for the Python module, covering successful construction
of `Document` from a valid title, rejection of blank titles via `ValueError`,
round-tripping markup through the module-level helper, both directions of the
MessagePack bridge, and the new XML exchange APIs. Behaviour-driven coverage
now parses canonical TEI fixtures, rejects malformed payloads, emits canonical
strings, and proves forbidden characters bubble up as `ValueError` with an
actionable message. New dictionary scenarios cover happy-path decoding, missing
fields, blank titles, and the `TypeError` raised when `to_dict` is called with
the wrong object. New validation scenarios assert that duplicate `xml:id`
values are rejected and that utterance speakers must be declared when a profile
cast exists, while documents without a cast still pass validation.

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

Python data classes now live in `tei_rapporteur.structs`. The submodule defines
`msgspec.Struct` projections (`Episode`, `TeiHeader`, `FileDesc`, `Paragraph`,
`Utterance`, and `Hi`) that mirror the Rust serde layout. Inline nodes decode
into plain Python objects, so pauses and other inline variants remain flexible.
MessagePack emitted by `to_msgpack` decodes directly into these classes, and
encoding them feeds the payload straight back into `from_msgpack`.

Binary interchange is now supported through
`tei_rapporteur.from_msgpack(payload: bytes)`. The helper accepts the bytes
produced by `msgspec.msgpack.encode` (or any compatible encoder), decodes them
via `rmp_serde`, and returns a `Document`. Invalid payloads raise `ValueError`,
so Python callers receive a familiar exception instead of a Rust-specific error
type. This allows workflows such as:

```python
import msgspec
import tei_rapporteur as tei
from tei_rapporteur.structs import Episode

episode = Episode(title="Bridgewater")  # msgspec.Struct
payload = msgspec.msgpack.encode(episode)
document = tei.from_msgpack(payload)
print(document.title)
```

The inverse helper, `tei_rapporteur.to_msgpack(doc: Document)`, serialises the
validated document into MessagePack bytes via `rmp_serde::to_vec_named`. The
function returns Python `bytes`, making it trivial to persist the payload or
feed it straight into `msgspec.msgpack.decode` to hydrate a structured type.
Non-`Document` inputs raise a `TypeError`, giving users immediate feedback when
they miswire a call. A complete round trip therefore looks like:

```python
doc = tei.Document("Bridgewater")
payload = tei.to_msgpack(doc)
from tei_rapporteur.structs import Episode
episode = msgspec.msgpack.decode(payload, type=Episode)
```

For JSON-style hand-offs, `tei_rapporteur.from_dict(payload)` and
`tei_rapporteur.to_dict(doc)` use `pyo3-serde` to bridge Python built-ins and
the Rust `TeiDocument`. The helpers accept any mapping/sequence tree that would
be valid JSON, raising `ValueError` when required fields are missing or titles
are blank and `TypeError` when a non-`Document` is passed. The output of
`to_dict` matches what `msgspec.to_builtins` produces, so callers can stay with
native Python objects:

```python
doc = tei.Document("Bridgewater")
payload = tei.to_dict(doc)
assert payload["teiHeader"]["fileDesc"]["title"] == "Bridgewater"
round_tripped = tei.from_dict(payload)
```

When scripts already have TEI XML on disk, the new `tei_rapporteur.parse_xml`
and `tei_rapporteur.emit_xml` functions avoid redundant conversions.
`parse_xml` hands the string straight to the Rust parser, returning a
`Document` that holds the validated `TeiDocument`. `emit_xml` performs the
inverse operation and retains the forbidden-character guardrails enforced by
`tei-xml`. A typical round trip combining XML and Python struct manipulation
therefore looks like:

```python
from pathlib import Path
import msgspec
import tei_rapporteur as tei
from tei_rapporteur.structs import Episode

doc = tei.parse_xml(Path("episode.tei.xml").read_text())
payload = tei.to_msgpack(doc)
episode = msgspec.msgpack.decode(payload, type=Episode)
episode.title = "Wolf 359 Reissue"
doc = tei.from_msgpack(msgspec.msgpack.encode(episode))
xml = tei.emit_xml(doc)
```

The BDD tests now cover successful decoding, encoding, XML parsing, emission,
and the corresponding error paths, ensuring the entry points remain reliable as
the API expands.
