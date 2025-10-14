# TEI-Rapporteur Implementation Roadmap

This document outlines the development plan for `tei-rapporteur`, structured into distinct phases, steps, and tasks. The plan follows an incremental approach, beginning with the core Rust library and progressively adding the Python interface and advanced features.

## Phase 1: Core Rust Library Implementation

This phase establishes the foundational Rust crates, providing a robust, standalone library for parsing, manipulating, and emitting the profiled TEI P5 subset. The primary outcome is a functional Rust API, independent of any Python integration.

### Step 1.1: Project Scaffolding and Workspace Setup

This step creates the monorepo structure that will house all crates and configuration, ensuring a clean separation of concerns from the outset.

- [ ] Initialize a new project using the `leynos/agent-template-rust` copier.
- [ ] Convert the generated project into a Cargo workspace by creating a root `Cargo.toml` file.
- [ ] Define the initial workspace members: `tei-core`, `tei-xml`, and `tei-py`.
- [ ] Create the directory structure for each crate (`/tei-core`, `/tei-xml`, `/tei-py`), each containing a basic `Cargo.toml` and `src/lib.rs`.
- [ ] Configure inter-crate dependencies (e.g., `tei-xml` depends on `tei-core`).
- [ ] Set up the root `.gitignore` file to handle workspace and build artefacts.

### Step 1.2: Core Data Model (,`tei-core`,)

This step implements the canonical data structures in pure Rust, representing the TEI Episodic Profile as defined in the design document.

- [ ] Define the top-level `TeiDocument` struct, containing `TeiHeader` and `TeiText`.
- [ ] Implement the structs for the TEI Header: `TeiHeader`, `FileDesc`, `ProfileDesc`, `EncodingDesc`, and `RevisionDesc`.
- [ ] Implement the structs for the TEI Body: `TeiText`, `TeiBody`, `P` (paragraph), and `Utterance`.
- [ ] Model mixed content using an `Inline` enum to represent plain text and elements like `<hi>` and `<pause>`.
- [ ] Add `serde::Serialize` and `serde::Deserialize` derives to all data model structs and enums.
- [ ] Implement a custom `TeiError` enum using `thiserror` for structured error handling.
- [ ] Achieve 95% unit test coverage for all data model invariants and business logic within `tei-core`.

### Step 1.3: XML Serialization and Deserialization (,`tei-xml`,)

This step provides the I/O functionality, enabling the conversion between in-memory Rust structs and TEI XML strings.

- [ ] Add `quick-xml` with the `serde` feature as a dependency to `tei-xml`.
- [ ] Implement the `parse_xml(xml: &str) -> Result<TeiDocument, TeiError>` function using `quick_xml::de::from_str`.
- [ ] Implement the `emit_xml(doc: &TeiDocument) -> Result<String, TeiError>` function using `quick_xml::se::to_string`.
- [ ] Write integration tests to verify semantic round-trip fidelity: `emit_xml(parse_xml(input))` should produce a canonically equivalent XML output.
- [ ] Ensure tests cover namespace handling and normalization of insignificant whitespace.

## Phase 2: Python Integration

This phase focuses on building the Python interface, making the core Rust functionality accessible to Python developers through an ergonomic, high-performance wrapper.

### Step 2.1: Python Wrapper Crate (,`tei-py`,)

This step sets up the PyO3 crate and defines the Python module structure.

- [ ] Add `pyo3` with the `extension-module` feature as a dependency to the `tei-py` crate.
- [ ] Create the top-level `#[pymodule]` to define the `tei_rapporteur` Python module.
- [ ] Add a `pyproject.toml` file to the workspace root, configured to build the `tei-py` crate using `maturin`.
- [ ] Define a `#[pyclass]` named `Document` that wraps the Rust `TeiDocument` struct.
- [ ] Implement basic CI workflow steps to build and install the Python wheel.

### Step 2.2: FFI Data Exchange and API Implementation

This step implements the functions that bridge the Rust and Python worlds, prioritizing efficient data transfer.

- [ ] Implement `from_msgpack(bytes: &[u8]) -> PyResult<Document>` in `tei-py`, using `rmp_serde` to deserialize bytes into `TeiDocument`.
- [ ] Implement `to_msgpack(doc: &Document) -> PyResult<Vec<u8>>` in `tei-py`, using `rmp_serde` to serialize `TeiDocument` to MessagePack bytes.
- [ ] Implement `parse_xml(xml_str: &str) -> PyResult<Document>` and `emit_xml(doc: &Document) -> PyResult<String>` as Python-callable functions.
- [ ] Add `pyo3-serde` to `tei-py` to implement `from_dict` and `to_dict` functions for JSON-like Python object exchange.

### Step 2.3: Python-Side Definitions and Packaging

This step defines the Python user experience, including the data classes and package distribution.

- [ ] Define the Python `msgspec.Struct` classes (`Episode`, `Utterance`, etc.) that mirror the Rust data model's structure.
- [ ] Document the public Python API, including usage examples for parsing XML, converting to/from `msgspec` objects, and emitting XML.
- [ ] Configure `maturin` to build and publish cross-platform wheels to PyPI.
- [ ] Write Python-level integration tests that cover the full workflow: XML -> `Document` -> `Episode` struct -> modify -> `Document` -> XML.

## Phase 3: Validation and Advanced Features

This phase enhances the library with robust validation, formalizes serialization formats, and introduces advanced capabilities.

### Step 3.1: Data Integrity and Validation

This step implements the internal and external validation strategies to guarantee data correctness.

- [ ] Implement the `TeiDocument::validate()` method in `tei-core` to perform internal checks (e.g., unique `xml:id`s, valid cross-references).
- [ ] Expose the `validate()` method as a function in the `tei-py` Python API, which raises a `ValueError` on failure.
- [ ] Formalize the TEI Episodic Profile by creating an ODD (One Document Does it all) specification.
- [ ] Generate a Relax NG schema from the ODD.
- [ ] Add a CI step that validates all test XML outputs against the generated Relax NG schema using an external tool like `jing`.

### Step 3.2: Formalized Serialization (,`tei-serde`,)

This step modularizes the JSON and MessagePack logic into a dedicated crate.

- [ ] Create the `tei-serde` crate and move `serde_json` and `rmp-serde` dependencies into it.
- [ ] Move serialization-specific logic from `tei-core` and `tei-py` into `tei-serde`.
- [ ] Generate and publish a versioned JSON Schema corresponding to the `TeiDocument` structure.
- [ ] Implement property-based tests to verify round-trip integrity between TEI XML, Rust structs, and JSON representations.

### Step 3.3: Streaming Parser (Future Work)

This step outlines the implementation of the experimental pull-parser interface for handling very large documents. (This step is considered optional for the initial release).

- [ ] Design and implement the `TeiPullParser` iterator in Rust, gated behind a `streaming` Cargo feature.
- [ ] Expose the pull-parser to Python as a generator function, `tei_rapporteur.iter_parse()`, that yields `msgspec`-compatible dictionaries.
- [ ] Write performance benchmarks comparing the memory and time usage of the full-document parser versus the streaming parser for large TEI files.

