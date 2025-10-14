# Workspace Layout

This document provides developers with an overview of the `tei-rapporteur` repository structure and the rationale behind its design.

## Guiding Principles

The project is structured as a **Rust workspace monorepo** that also produces a Python package. This approach was chosen for several key reasons, aligning directly with the project's architecture as set out in the design document:

1. **Single Source of Truth**: All Rust crates (`tei-core`, `tei-xml`, `tei-py`, etc.) and the Python packaging configuration (`pyproject.toml`) coexist in a single repository. This ensures that changes to the core Rust data model are immediately reflected in the Python wrapper, preventing version drift and integration issues.
2. **Clear Separation of Concerns**: The workspace model allows for a clean division of responsibilities. The `tei-core` crate contains only the pure Rust data model, with no knowledge of XML or Python. The `tei-xml` crate handles serialization, and the `tei-py` crate manages the FFI boundary. This modularity makes the codebase easier to navigate, test, and maintain.
3. **Unified Tooling**: The workspace can be built, tested, and linted with standard `cargo` commands from the root. Simultaneously, Python packaging is managed by `maturin` from the same root, providing a streamlined developer experience.

This structure was deliberately chosen over a Python-centric template (which would not naturally accommodate a multi-crate Rust design) and over separate repositories (which would introduce synchronization overhead).

## Directory Structure

The repository has the following high-level layout:

```null
.
├── Cargo.toml          # Workspace manifest, defines all member crates.
├── pyproject.toml      # Python packaging definition, configured for maturin.
│
├── tei-core/           # Crate for the core Rust data model (structs, enums).
│   ├── Cargo.toml
│   └── src/
│
├── tei-xml/            # Crate for XML parsing and emitting logic (quick-xml).
│   ├── Cargo.toml
│   └── src/
│
├── tei-py/             # Crate for the PyO3 wrapper and Python FFI.
│   ├── Cargo.toml
│   └── src/
│
├── tei-rapporteur/     # (Optional) Python package source directory.
│   └── __init__.py
│
├── .github/            # CI/CD workflows for both Rust and Python.
└── tests/              # Python integration tests.

```

### Key Components

- `Cargo.toml`** (Root)**: This is the workspace manifest. It does not define a package itself but lists all the member crates in the `[workspace.members]` array. This allows `cargo` to build and test all crates from the root directory.
- `pyproject.toml`** (Root)**: This file configures the Python package build process. It specifies `maturin` as the build backend and points to the `tei-py` crate's manifest (`manifest-path = "tei-py/Cargo.toml"`) to identify which crate should be compiled into the Python extension module.
- **Crate Directories (**`tei-*`**)**: Each directory is a self-contained Rust crate with its own `Cargo.toml`. Dependencies between crates (e.g., `tei-py` depending on `tei-core`) are defined within these individual manifests using relative path dependencies.

This layout ensures that the project is simultaneously a first-class Rust workspace and a well-defined Python package, directly reflecting the dual-language nature of its design.

