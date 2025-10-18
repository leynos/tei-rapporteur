# tei-rapporteur

<!-- markdownlint-disable-next-line MD036 -->
*Podcast-grade TEI P5: ergonomic, fast, and pleasantly interoperable*

> **Status: Work in Progress** — This repository is an active build. Interfaces
> and crate layout will stabilize as we converge on the design doc and roadmap.

______________________________________________________________________

## What this is

`tei-rapporteur` is a Rust-first TEI P5 data model with a fast, safe parser and
emitter, wrapped for Python via PyO3. Python consumers work with **`msgspec`
Structs** by default for speed and clean serialization in databases and
services. XML handling remains in the Rust domain; Python gets zero-copy-ish
accessors and efficient conversions.

The initial focus is **Episodic** use‑cases:

- *Episode creation* — structured authoring of episode metadata and text.
- *Bromide* — cliché/snowclone detection over TEI content.
- *Chiltern* — pronunciation guidance for TTS (especially model numbers,
  neologisms, terms of art).
- *Ascorbic* — accent drift detection within episodes.

Lossless round‑tripping is **not** a goal; we prioritise **semantic fidelity**
and ergonomic domain objects over XML trivia. Notes/comments may be embedded in
deserialized objects for traceability.

Further details live in the design and planning docs:

- **Design document:**
  [`docs/tei-rapporteur-design-document.md`](docs/tei-rapporteur-design-document.md)
- **Roadmap:** [`docs/roadmap.md`](docs/roadmap.md)
- **Workspace layout:** [`docs/workspace-layout.md`](docs/workspace-layout.md)

## Why this exists

TEI P5 is powerful, but most tools either bury you in XML‑minutiae or hide
structure behind slow dynamics. `tei-rapporteur` keeps the **authoritative
model in Rust**, guarantees performance and safety, and offers a **lean Python
surface** for analytics, pipelines, and storage. Think: *one source of truth in
Rust; elegant Python projections.*

## Architecture at a glance

```plaintext
          +-------------------------+
          |        Python App       |
          |  (pipelines, notebooks) |
          +------------+------------+
                       | PyO3 bridge
                       v
+----------------------+----------------------+
|               tei-rapporteur (Rust)         |
|  TEI P5 domain model • parser • emitter     |
|  Feature-gated: pull parser (experimental)  |
+----------------------+----------------------+
                       |
                       v
         msgspec Structs (Python view)
```

- **Rust core**: TEI domain types, fast parser/emitter, validation, transforms.
- **Python wrapper**: thin PyO3 layer exposing Rust functionality; default
  representation is `msgspec` Structs for low‑overhead (de)serialization.
- **Experimental pull‑parser**: optional, behind a feature flag; may rely on
  Rust nightly continuations or generators.

## Scope and non‑goals

### In scope

- Subset of TEI P5 required for Episodic features.
- Semantic transforms, annotation hooks, and performant emit to XML.
- Python API parity for public Rust functions (work flows in Rust).

### Out of scope (for now)

- Teasing out every corner of full TEI P5.
- Exact byte‑preserving round‑trips.
- Editing arbitrary XML via Python without going through domain objects.

## Getting started

### Prerequisites

- Rust (stable, latest recommended). Nightly only for the gated pull‑parser.
- Python 3.11+.
- [`maturin`](https://github.com/PyO3/maturin) for building the Python wheel.
- `msgspec` for Python representation.

### Build (Rust)

```bash
# Build the Rust workspace
cargo build --workspace

# Run tests
cargo test --workspace
```

### Build and develop (Python bindings)

```bash
# Create and activate a virtual environment
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate

# Install maturin and msgspec
pip install -U pip maturin msgspec

# Build extension in-place (debug)
maturin develop

# Smoke test
python - <<'PY'
import tei_rapporteur as tr
print("tei_rapporteur:", getattr(tr, "__version__", "dev"))
PY
```

### Feature: experimental pull‑parser (gated)

The pull‑parser may depend on Rust nightly features (continuations/generators).
Enable it explicitly:

```bash
# Build with experimental features (nightly toolchain)
rustup toolchain install nightly
cargo +nightly build --workspace --features pull
```

Rationale and constraints are documented in the design doc.

## Quickstart

### Rust

```rust
use tei_rapporteur::{Episode, from_xml, to_xml};

fn main() -> anyhow::Result<()> {
    let xml = std::fs::read_to_string("examples/episode.xml")?;
    let mut ep: Episode = from_xml(&xml)?;
    ep.title = "Episode 12: The Long Hundred".into();
    let out = to_xml(&ep)?;
    println!("{}", out);
    Ok(())
}
```

### Python (msgspec surface)

```python
from tei_rapporteur import parse_episode, emit_episode
from msgspec.json import encode

with open("examples/episode.xml", "rb") as fh:
    ep = parse_episode(fh.read())

print(ep.title)
print(encode(ep).decode())  # serialize to JSON for storage/logging

xml = emit_episode(ep)
print(xml[:200], "…")
```

## Project status

Active development. Expect breaking changes until `v0.1` lands. See the
**Roadmap** for milestones and the **Design document** for evolving decisions
and trade‑offs.

- **Design document:**
  [`docs/tei-rapporteur-design-document.md`](docs/tei-rapporteur-design-document.md)
- **Roadmap:** [`docs/roadmap.md`](docs/roadmap.md)
- **Workspace layout:** [`docs/workspace-layout.md`](docs/workspace-layout.md)

## Contributing

Issues and PRs are welcome. Please keep changes scoped and evidenced:

- Add tests alongside new domain types or transforms.
- Prefer benchmarks for parser/emitter changes.
- Document public APIs in Rust doc‑comments; surface equivalent behaviour in
  Python.

We use `cargo fmt`, `cargo clippy`, and type‑checked Python stubs where useful.
Discussions about TEI profiling or Episodic semantics belong in issues linked
from the **Roadmap**.

## Licence

This repository is licensed under the **ISC** licence. See [`LICENSE`](LICENSE).

______________________________________________________________________

`tei-rapporteur` is part of a broader tooling stack aimed at making long‑form
audio publishing less brittle and more measurable. Build fast, annotate richly,
and keep the data model honest.
