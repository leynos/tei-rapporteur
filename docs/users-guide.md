# User's guide

The `tei-rapporteur` workspace currently focuses on establishing the crate
layout that underpins the rest of the roadmap. This guide summarizes what is
available today and how to exercise it.

## Workspace overview

- `tei-core` now models the top-level `TeiDocument` together with its
  `TeiHeader` and placeholder `TeiText`. Header metadata is captured through
  dedicated structs (`FileDesc`, `ProfileDesc`, `EncodingDesc`, and
  `RevisionDesc`) so callers can assemble rich document context without
  touching XML.
- `tei-xml` depends on the core crate and offers
  `serialize_document_title(raw_title)`, which turns validated titles into a
  `<title>` snippet.
- `tei-py` depends on both crates and re-exports the serialization helper as
  `emit_title_markup`. This crate is the future home of the PyO3 bindings.
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
unhappy paths. Core scenarios validate that header metadata can be assembled
and that blank revision notes are rejected, whilst the XML crate confirms title
serialization and error propagation. These tests run alongside the unit suite
so developers receive fast feedback when modifying the scaffolding.
