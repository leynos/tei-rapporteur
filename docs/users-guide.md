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
  while the new `parse_xml(xml)` helper wraps `quick-xml` to materialise full
  `TeiDocument` values. Both functions surface parser/validator issues via the
  shared `TeiError` enum.
- `tei-py` depends on both crates and re-exports the serialization helper as
  `emit_title_markup`, propagating the same `TeiError` enum. This crate is the
  future home of the PyO3 bindings.
- `tei-test-helpers` captures assertion helpers that multiple crates reuse in
  their unit and behaviour-driven tests.

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
tests both title serialisation and the new parser: feature files cover
successful parsing, missing header errors, and syntax failures triggered by
truncated documents. These tests run alongside the unit suite, so developers
receive fast feedback when modifying the scaffolding.
