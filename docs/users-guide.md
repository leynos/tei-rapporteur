# User's guide

The `tei-rapporteur` workspace currently focuses on establishing the crate
layout that underpins the rest of the roadmap. This guide summarizes what is
available today and how to exercise it.

## Workspace overview

- `tei-core` hosts the first domain type, `DocumentTitle`, alongside a minimal
  `TeiDocument` struct. Titles are trimmed and validated at construction time
  so downstream crates never observe empty `<title>` elements.
- `tei-xml` depends on the core crate and offers
  `serialize_document_title(raw_title)`, which turns validated titles into a
  `<title>` snippet.
- `tei-py` depends on both crates and re-exports the serialization helper as
  `emit_title_markup`. This crate is the future home of the PyO3 bindings.

## Building and testing

Use the Makefile targets to work with the entire workspace:

- `make build` compiles every crate in debug mode.
- `make test` runs all unit tests and the behaviour tests powered by
  `rstest-bdd`.
- `make check-fmt`, `make lint`, and `make fmt` mirror the repository quality
  gates described in `AGENTS.md`.

## Behavioural guarantees

`tei-xml` ships with behaviour-driven tests that exercise happy and unhappy
paths for title serialization. Successful scenarios confirm the generated TEI
markup, whilst failure scenarios assert that empty titles are rejected with a
clear error message. These tests run alongside the unit tests via `cargo test`
so developers receive fast feedback when modifying the scaffolding.
