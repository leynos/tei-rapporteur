# TEI-Rapporteur Design Document

## Introduction

**TEI-Rapporteur** is a Rust-based library providing a canonical data model,
parser, and emitter for a constrained subset of **TEI P5** (Text Encoding
Initiative) XML, tailored to podcast-related use cases. It serves as the
structured "content spine" for a podcast scripting ecosystem, supporting tasks
such as episode authoring (code-named *Episodic*), cliché detection
(*Bromide*), pronunciation QA (*Chiltern*), and accent drift detection
(*Ascorbic*). The library is implemented in Rust for performance and
correctness, with a Python interface built via PyO3. Python users interact with
TEI data through high-level `msgspec.Struct` classes (from the msgspec library)
rather than low-level PyO3 objects. This design keeps Rust as the single source
of truth for parsing/validation while allowing Python developers to work with
familiar dataclass-like objects.

The guiding principle is that TEI XML is the **canonical data model**, and all
other representations (JSON, MessagePack, Python classes, etc.) are projections
or views. Rust “owns” the strong types and enforces validity, whereas Python
serves as an ergonomic scripting layer. At the FFI boundary, Python
`msgspec.Struct` instances are serialized to bytes (JSON or MessagePack) and
passed into Rust; Rust returns validated data back as bytes, which Python code
decodes into `msgspec.Struct` objects. This ensures that no parsing or schema
validation logic is duplicated in Python – the Python side is essentially
stateless with respect to TEI structure. By centralizing all XML handling in
Rust, we achieve a **deterministic, single-source-of-truth** conversion: for
any given TEI input, Rust produces a canonical normalized output, and the
semantic content remains consistent across round-trips.

This document outlines the design of the `tei-rapporteur` library, including
the TEI P5 subset definition, Rust data model and crate architecture,
parsing/emitting strategy, Python integration layer, a proposed streaming
parser extension, and considerations for validation and performance. Example
Rust and Python code are provided to illustrate usage, along with an
architectural overview and a comparison of serialization pathways between Rust
and Python. Throughout, we focus on writing clear, maintainable code – **Rust
remains idiomatic and free of Python-specific cruft, and Python APIs feel
natural to Python users**.

## Workspace scaffolding decisions

The workspace now follows the layout described in `docs/workspace-layout.md`.
Three crates ship with the repository:

- `tei-core` defines early placeholder types that exercise crate boundaries.
  The new `DocumentTitle` newtype rejects empty titles at construction time and
  surfaces an idiomatic `thiserror` error enum so that future data-model code
  can build on consistent validation primitives.
- `tei-xml` depends on `tei-core` and offers a `serialise_document_title`
  helper. The function demonstrates how XML-facing crates will transform the
  core types while keeping validation in the domain layer.
- `tei-py` depends on both crates and forwards the serialisation helper so that
  PyO3 integration work inherits the existing validation logic.

The workspace manifest centralises metadata, lint rules, and shared dependency
versions. Each crate opts into the shared configuration via `workspace = true`
stanzas, ensuring that future crates inherit the same guard rails. Behavioural
coverage for the scaffolding relies on `rstest-bdd`, providing both a happy
path scenario (serialising a valid title) and a failure scenario (rejecting an
empty title). These scenarios will evolve into richer contract tests as the
feature set grows.

## TEI P5 Subset for Podcast Use Cases

**TEI P5** is a comprehensive and extensible XML schema for encoding texts, but
using all of TEI would be overkill. We define a **profiled subset of TEI P5**
(informally, *TEI Episodic Profile*) that includes only the elements and
attributes needed for our podcasting scenarios. This focused subset makes the
data model simpler to implement and guarantees round-trip fidelity for the
features we actually use.

Key inclusions in the TEI subset (to be formally specified via an ODD
customization) are:

- **TEI Header**: Each document begins with a `<teiHeader>`, containing:

- `<fileDesc>` with bibliographic metadata (title of episode, series info,
  etc.).

- `<profileDesc>` with information like cast/speakers, languages or dialects
  used (for accent analysis), and perhaps a description of content (could
  include an index of segments or word count).

- `<encodingDesc>` to document the usage of certain annotations (e.g. a
  controlled vocabulary for cliché types or accent labels).

- `<revisionDesc>` with provenance: e.g. timestamps or versions for when the
  script was generated or edited, tool versions (Bromide, Chiltern, etc.), and
  content hashes for reproducibility.

- **Body Structure (Dialogue and Narrative)**: The main content of the episode
  script is in `<text><body>` with a structure for spoken dialogue and
  narration:

- For a **single-host or conversational script**, we use TEI’s spoken text or
  drama notation. Two possible approaches were considered:

- *Spoken Transcripts Module*: Use `<u>` elements (utterances) for each speaker
  turn([1](https://journals.openedition.org/corpus/4553?lang=en#:~:text=32%E2%97%8B%20,describe%20a%20%E2%80%9Cspeech%20event)).
  Each `<u>` may have a `who` attribute pointing to a speaker identifier
  (defined in the header’s cast list). Within `<u>`, the transcribed speech is
  encoded, possibly with inline elements for emphasis, pauses, etc.

- *Drama Module*: Use `<sp>` elements for speech, containing a `<speaker>`
  child for the speaker’s name and the spoken text in one or more `<p>`
  paragraphs. This approach is more verbose but explicitly tags the speaker
  name in the text flow.

- We will initially support one approach (using `<u>` for simplicity), but the
  data model is flexible enough to extend to the other. In either case, the
  content of an utterance can include **mixed content**: plain text
  interspersed with inline tags like `<hi>` (for emphasis or styled text),
  `<seg>` (generic segment, e.g. marking a phrase), `<pause>` elements for
  pauses, or others as needed. Handling mixed content is a crucial aspect and
  is discussed in the data model design below.

- **Stand-off Annotations**: Analytical modules like Bromide will add
  annotations without altering the original text. We leverage TEI’s
  `<standOff>` section to hold annotations as spans that reference portions of
  the primary text. For example, Bromide could output:

- A `<spanGrp type="cliche">` grouping all detected clichés, with an attribute
  `resp="#bromide"` pointing to the responsible tool and maybe `corresp`
  linking to the overall text or episode ID.

- Inside, each `<span>` marks a clichéd phrase with attributes like `from` and
  `to` pointing to the start and end points in the text (using TEI’s canonical
  referencing scheme, e.g. pointers into utterances by character offset) and an
  `ana` (analysis) code indicating the type of cliché. The `xml:id` of the
  `<span>` uniquely identifies the annotation. This stand-off markup means the
  base script text remains intact, and multiple annotation layers (clichés,
  pronunciation issues, accent shifts, etc.) can coexist.

- We anticipate similar usage for *Chiltern* (pronunciation QA) and *Ascorbic*
  (accent drift). For instance, Chiltern might identify words with
  pronunciation questions and mark them with `<spanGrp type="pronunciation">`
  annotations, or insert a TEI `<pron>` element if inline representation is
  needed([2](https://www.tei-c.org/Vault/P5//2.4.0/doc/tei-p5-doc/en/html/ref-pron.html#:~:text=P5%3A%20Guidelines%20for%20Electronic%20Text,s%29%20of%20the%20word)).
   Ascorbic might mark segments of an utterance where a speaker’s accent
  appears to shift, using a `<spanGrp type="accent">` with spans covering the
  relevant time or text range and an `ana` value indicating the detected
  accent. These specifics will be refined as those tools develop, but the TEI
  framework can accommodate them via stand-off markup or inline tags (e.g., a
  custom `<distinctPron>` tag defined in the ODD if needed).

- All annotation types will be documented in the TEI header’s `<encodingDesc>`
  so that the semantics of `@ana` codes and custom element usage are clear and
  versioned.

By constraining ourselves to this profile (let’s call it **TEI P5
Episodic-Core**), we avoid the complexity of the entire TEI schema. This subset
will be rigorously documented and accompanied by a Relax NG schema and
Schematron rules generated from the ODD. That way, we have a contract for what
constitutes a valid Episodic TEI document. Any TEI features outside this
profile (unknown elements or attributes) are either dropped or preserved in a
generic way (e.g., as untyped `extra` fields) so that the parser doesn’t break
on unexpected input. The goal is to **accept and preserve all relevant
information** for our use cases, while being forward-compatible with minor
extensions. If an input uses a TEI construct we haven’t modeled, the design
should either ignore it safely or store it in a placeholder structure for
round-trip output, rather than erroring out.

## Architecture Overview

The `tei-rapporteur` project is organized into multiple Rust crates within a
single workspace, each with a focused responsibility. This modular design aids
clarity and allows reuse or independent testing of components:

- **`tei-core`**: A Rust library crate containing the core TEI data model
  (structs and enums representing TEI elements), business logic (e.g., any
  invariant enforcement or transformation functions), and basic validation
  routines. This crate has no dependency on Python or PyO3 – it is pure Rust
  (potentially even `no_std` compatible for portability). Rust developers can
  use `tei-core` directly to parse, manipulate, and emit TEI in Rust
  applications without any Python involvement.

- **`tei-xml`**: The XML parsing and writing logic built on a high-performance
  XML library (we use `quick-xml`). This crate provides functions to parse TEI
  XML into `tei-core` data structures and to serialize `tei-core` structures
  back to XML. It may also include streaming parse utilities (pull parser) and
  pretty-printing or canonicalization logic. In practice, `tei-xml` might be
  merged with `tei-core` if tightly coupled, but conceptually it’s a separate
  concern (XML I/O). This separation can help if we later support alternative
  input formats (e.g., if someone wanted to import a Markdown transcript and
  produce TEI).

- **`tei-serde`**: (Optional) A crate providing serde serializers/deserializers
  for converting the TEI data structures to/from other formats like JSON or
  MessagePack. For example, it might use `serde_json` and `rmp-serde` to enable
  (de)serializing `TeiDocument` to JSON/MsgPack. This crate ensures that the
  Rust data model can be cleanly represented in JSON (the same structure that
  `msgspec.Struct` expects on the Python side). We can also include versioned
  JSON schema snapshots here for integration testing and for documentation of
  the JSON structure.

- **`tei-ann`**: (Optional/future) A helper crate for working with annotations
  (spans). It could provide utilities to apply stand-off `<span>` annotations
  to a text, or to extract and query them. For example, Bromide’s output might
  use this to insert cliché annotations or to merge overlapping spans. This is
  not critical for the basic parser/emitter functionality but part of the
  broader toolkit.

- **`tei-py`**: A PyO3-based Python extension module (compiled to a native
  `.pyd/.so` library) that exposes a Python API. This is the bridge between
  Python and the Rust core. It defines the Python-callable functions and
  classes, marshalling data to and from Rust. **Importantly, `tei-py` is a thin
  layer**: it should primarily accept Python byte arrays or simple types,
  delegate to Rust for actual work, and then return results back to Python
  (often as bytes). We avoid exposing complex Rust structures directly through
  PyO3, which keeps the Rust API idiomatic and prevents Python-specific
  concepts from leaking into `tei-core`. Rust developers working on `tei-core`
  do not need to even be aware of the Python wrapper if they don’t use it.

All crates are versioned together in a single repository (monorepo), ensuring
that changes to `tei-core` stay in sync with `tei-py` and others. We plan to
distribute `tei-core` on crates.io for Rust use, and `tei-py` as a Python wheel
(via maturin) for easy pip installation. This way, Rust-only projects and
Python projects can each consume the library through their natural channels.

**Crate Dependency and Feature Flags**: To maintain a separation of concerns,
the core crates do not depend on PyO3. If needed, we can create an intermediate
crate or feature for conversion glue:

- We might introduce a `tei-bridge` crate (or a feature in `tei-core`), which
  when enabled pulls in `pyo3` and implements traits like `FromPyObject` and
  `ToPyObject` for our core types. This would allow directly accepting Python
  `dict` or `list` structures in Rust functions via serde, using `pyo3-serde`
  under the hood. This approach is purely optional and kept behind a feature
  flag (e.g., a `python` feature) so that by default `tei-core` remains free of
  any Python code.

- Whether we use a separate `tei-bridge` crate or just put these behind a
  feature, the Python extension (`tei-py`) will enable that and thus gain the
  ability to easily convert Python objects. However, an even simpler strategy
  (described below) is to avoid passing rich Python objects at all and stick to
  bytes or basic types across the FFI. That reduces the need for a `tei-bridge`
  layer.

The overall architecture ensures a **clear API boundary** between Rust and
Python. Rust code is written as if it’s a standalone library (no mention of
GIL, `PyObject`, or Python-specific patterns in its public API). Python code
sees a natural interface (functions and dataclasses) and does not worry about
Rust internals. The contract at the boundary is simply serialized data. This
design avoids the “snakes in the Rust code” problem – Rust developers are not
confronted with a tangle of embedded Python types. They can contribute to
`tei-core` without knowing anything about PyO3 or Python, which is important
for long-term maintainability.

Below is a high-level diagram of the system components and data flow:

```plaintext
  [ TEI XML file ] 
         ↓ parse (Rust, quick-xml)
  ┏━━━━━━━━━━━━━━┓           FFI (bytes)           ┏━━━━━━━━━━━━━━━┓
  ┃   tei-core   ┃ ------------------------------> ┃    tei-py     ┃
  ┃  (Rust lib)  ┃ <-----------------------------> ┃ (Python ext)  ┃
  ┗━━━━━━━━━━━━━━┛       Rust structs (serde)      ┗━━━━━━━━━━━━━━━┛
         ↑                   and bytes                     ↑         
         |                (JSON/MsgPack)                   | Python API (msgspec)
         |                                                 | (e.g., Episode Struct)
         |                                                 |         
         |            ┏━━━━━━━━━━━━━━┓                     |
         |            ┃   tei-serde  ┃  JSON/MsgPack bytes |
         |            ┗━━━━━━━━━━━━━━┛  (via msgspec)      |
         |                                                 |
         |            ┏━━━━━━━━━━━━━━┓                     |
         |            ┃   tei-xml    ┃  XML string         |
         |            ┗━━━━━━━━━━━━━━┛  (quick-xml)        ↓         
[ TEI XML output ]                                      Python
                                                      application
```

*Figure: Architectural overview of **tei-rapporteur**. The `tei-core` library
in Rust manages the TEI data model and logic. `tei-xml` parses and emits TEI
XML using quick-xml. The `tei-serde` layer (if used) handles JSON/MsgPack
serialization of Rust structures. The `tei-py` module uses PyO3 to expose a
Python API, operating by passing serialized data across the FFI boundary.
Python code uses `msgspec.Struct` classes (e.g., an `Episode` class) to
represent TEI content, which are encoded/decoded to binary formats for
communication with Rust.*

## Rust Data Model and Serialization

### Data Model Design

In `tei-core`, we define Rust structs and enums to represent the TEI document
structure. The model closely mirrors the TEI XML hierarchy for our chosen
subset, while using Rust’s type system to enforce validity as much as possible.
For example:

- A top-level `TeiDocument` struct represents the `<TEI>` element (root of a
  TEI file). It contains a `tei_header: TeiHeader` and a `text: TeiText` (which
  in turn contains the `<body>` etc.). We might also include metadata like a
  version number for the model or a provenance field if needed (to track which
  version of the schema or tools produced it).

- `TeiHeader`, `TeiText`, `TeiBody`, etc. are structs for those sections. For
  instance, `TeiHeader` would have fields corresponding to the allowed child
  elements (fileDesc, profileDesc, encodingDesc, revisionDesc), each likely
  optional except fileDesc. A `TeiBody` might contain a list of top-level
  divisions or utterances. In a simple podcast script, the body could be just a
  sequence of paragraphs/utterances without further nesting, or we could allow
  a two-level hierarchy (e.g., `<div>` for sections like "Intro", "Interview",
  "Outro", each containing paragraphs or utterances).

- For narrative text like paragraphs we use a struct `P` with a content field.
  For dialogue, if using `<u>`, we might model it similarly to `P` (since an
  utterance can be treated as a special kind of paragraph with a speaker
  attribute). If using `<sp>`, we’d have an `Sp` struct with a
  `speaker: String` field and either text content or sub-elements.

- **Mixed Content**: TEI often allows text mixed with inline elements. For
  example, an utterance could contain plain text, interspersed with `<hi>`
  elements for emphasis and `<pause/>` empty elements. To model this, we use an
  enum for inline content. E.g.:

```rust
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Inline {
    Text(String),
    Hi(Hi),        // corresponds to <hi> element
    // ... other inline elements like <seg>, <pause> etc. can be added
}

#[derive(Serialize, Deserialize)]
struct P {
    #[serde(rename = "$value")]
    content: Vec<Inline>,
}
#[derive(Serialize, Deserialize)]
struct Hi {
    #[serde(rename = "rend", default)]
    rend: Option<String>,            // e.g. <hi rend="italic"> 
    #[serde(rename = "$value", default)]
    content: Vec<Inline>            // hi can contain further inline or text
}
```

In this example, `P` (paragraph or utterance) has a `content` list that can
hold either raw text segments or specific inline elements (like `Hi`). We use
Serde annotations `rename="$value"` to indicate that the text content is
captured as a value in XML, not a nested tag. We mark content lists with
`default` to allow empty content. The `untagged` enum means Serde will
discriminate based on whether a JSON value is a string or an object to decide
if it's `Text` vs `Hi`, which aligns with how quick-xml will treat XML text vs
child elements.

- **Attributes and Identifiers**: Attributes of TEI elements become struct
  fields, using `serde(rename = "...")` to map to the actual XML attribute
  names. For example, if we have a `<u who="s1">...</u>`, our `Utterance`
  struct might be:

```rust
struct Utterance {
    #[serde(rename="who", default)]
    speaker: Option<String>,
    #[serde(rename="$value", default)]
    content: Vec<Inline>
}
```

We use `Option<String>` for the speaker reference (since it might be omitted if
there's only one speaker or if using a different markup). Similarly, global
attributes like `xml:id` can be included. We will include an
`id: Option<String>` field on any element that can carry an `xml:id`. This
allows references between elements (e.g., a `<span from="#u42:...">`
referencing an utterance with `xml:id="u42"`). We ensure in validation that if
an element says `xml:id="X"` then `X` is unique within the document.

- **Preservation of Unknowns**: Because TEI is extensible, our model should not
  choke on unexpected attributes or child elements that might appear if the
  input slightly exceeds our subset. We use catch-all structures to handle
  these cases:

- For attributes, we can include a field like
  `#[serde(flatten)] extra_attrs: HashMap<String, String>` in each struct to
  collect any attribute that isn’t explicitly modeled. This way, unknown
  attributes round-trip (they’ll be output again on serialization) even if our
  code doesn’t interpret them.

- For unknown child elements, the strategy is trickier; one approach is to
  treat the content of an element as a list of an `Inline`-like enum that has a
  variant for “UnknownElement” carrying maybe a generic name and a list of
  children. This can get complex, so as a first step we might decide to simply
  ignore/drop children we don’t recognize (with a warning) or treat them as
  text if possible. But since we are profiling the input via ODD, ideally users
  won’t supply out-of-profile elements except via forward-compatible extension
  which we can handle via optional updates to the model.

- **Error Handling**: We define a central error type `TeiError` (using
  `thiserror` for convenience) to capture parse errors, validation errors,
  etc.. For example, `TeiError::Xml(String)` for an XML syntax or
  well-formedness error (wrapping the message from quick-xml if any), and
  `TeiError::Validation(String)` for semantic issues (like duplicate IDs). This
  error type is used consistently across parse/emit/validate functions.

### XML Parsing and Emitting (serde + quick-xml)

Parsing TEI XML in Rust is done entirely in the `tei-xml` layer using
**quick-xml** with Serde serialization. Quick-xml is a speedy pull-parser that
also provides Serde integration: we can simply derive `serde::Deserialize` on
our structs and then call `quick_xml::de::from_str::<TeiDocument>(&xml_string)`
to get a `TeiDocument` in memory. Similarly, we derive `Serialize` and use
`quick_xml::se::to_string(&doc)` to generate an XML string. This approach
yields a **semantic round-trip**: as long as the input adheres to our data
model, `emit_tei(parse_tei(input))` will produce an XML document equivalent in
content to the original. The output may not be byte-for-byte identical
(whitespace, attribute ordering, and namespace prefixes are normalized), but it
is equivalent at the XML node level. We consider this acceptable, even
desirable, for a canonicalization perspective – it ensures consistent
formatting for downstream processing and diffing.

Some specifics of the parse/emit implementation:

- **Insignificant Whitespace**: By default, our model will not preserve
  insignificant whitespace. For instance, pretty-printing line breaks and
  indentation inside the XML are not retained (quick-xml’s Serde will treat
  them as either ignored or as empty text nodes that we drop). We normalize
  text where appropriate. If whitespace is meaningful (e.g., within `<u>` text,
  spaces between words are of course preserved exactly as in the text content),
  but leading/trailing whitespace in certain contexts or between block elements
  will be trimmed or made consistent. The design goal is to have a *clean,
  normalized XML output* that tools or version control can reliably compare.

- **Comments and PIs**: We do not parse or round-trip XML comments and
  processing instructions in the initial design. These are outside the scope of
  Serde mapping (which deals with data content). The round-trip is therefore
  *semantic, not fully lossless*. Any comments in the source will be dropped
  when parsing to Rust types. This is acceptable for our use cases (we don’t
  anticipate users adding manual comments in an auto-generated script, but it
  could happen). As a future-proofing measure, we have considered adding an
  **annotation field for comments** – for example, each element struct could
  have an optional `comments: Vec<String>` to collect comments that appeared
  immediately inside it (or perhaps just a special field at the document level
  for all comments). By marking such a field with
  `#[serde(skip_deserializing, skip_serializing)]`, we could populate it via a
  custom parse routine if needed. However, implementing that would require a
  custom parser loop (since quick-xml Serde will ignore comments entirely). A
  more robust future solution is a *dual-layer* approach: maintain a parallel
  lossless representation for exact round-trips. In such a design, we would
  have an `XmlDocument` (or a low-level tree of XML nodes) that can capture
  everything including comments, entity references, etc., and then a conversion
  to `TeiDocument`. If exact re-serialization is needed for some editing tools,
  we could use the original `XmlDocument` for output when no semantic changes
  have been made. For now, we explicitly choose **normalized, not fully
  lossless, output**, and we document that choice to avoid future confusion.

- **Namespaces**: TEI uses the default TEI namespace for most elements. We will
  preserve the namespace URI in the output. Quick-xml can handle namespaces,
  especially if we ensure our structs carry the correct names (possibly using
  `serde(rename = "tei:elementName")` if needed to bind to a prefix). When
  emitting, we can choose a stable prefix, e.g., always use `tei:` prefix on
  all TEI elements for clarity. The library will ensure the XML root has the
  proper namespace declaration. Since we likely only have the TEI namespace
  (and perhaps none others except xml: for IDs and lang), this is
  straightforward. We will not rely on the exact prefixes from input; those
  might be normalized in output.

- **Mixed Content Handling**: As shown earlier, by modeling mixed content with
  enums and `$value` placeholders, Serde with quick-xml handles most cases. One
  corner is that **empty elements** like `<pause/>` need to be represented
  appropriately. We might model `<pause>` as a unit struct or as an enum
  variant with no associated data. Quick-xml’s serialize will emit an empty tag
  for an empty string or unit struct. We will test these to ensure, for
  example, a `<pause/>` in the XML becomes something like `Inline::Pause` in
  Rust and back.

- **Performance**: Quick-xml and Serde are known to be efficient. The parse is
  done in one pass and directly fills our structs without intermediate DOM
  creation. The emit is similarly streaming. This means memory overhead is
  proportional to the size of the data (we allocate Rust structs for the
  content) and does not hold the entire file as a heavy DOM with overhead.
  Benchmarks have shown quick-xml to be significantly faster than DOM-based
  parsers in Rust, making it suitable for even large transcripts. If we
  encounter extremely large documents (say transcripts of many hours, tens of
  thousands of utterances), the memory usage of holding the entire
  `TeiDocument` at once might become a concern, which motivates the streaming
  approach described later. But for reasonably sized episodes (on the order of
  a few thousand words), this approach should be very fast and use only a few
  MBs of memory.

### JSON/MessagePack Serialization

In addition to XML, the Rust data model can be serialized to and from JSON or
MessagePack, facilitating two things:

- **Persistence/Interchange**: We may store episodes in a database as JSON
  (e.g., in Postgres JSONB) or send them over a network as MessagePack.

- **Python Interop**: The Python side `msgspec.Struct` classes have a schema
  that matches this JSON structure, so serializing in Rust to JSON/MsgPack and
  decoding in Python (or vice versa) allows easy data exchange.

We derive `serde::Serialize`/`Deserialize` on all our core types, which
automatically gives us JSON and MessagePack support via `serde_json` and
`rmp-serde` respectively. The JSON structure is a **semantic projection** of
the TEI. For instance, a TEI `<spanGrp type="cliche">` might be represented in
JSON as an object like:

```json
{ "type": "cliche", "spans": [ { "id": "c1", "start": 100, "end": 120, "ana": "cliche:idiom.snowclone" } ] }
```

(This is illustrative; the actual JSON schema will be defined so that it aligns
with our Python `Struct` definitions exactly, including any nesting.)
Attributes become JSON fields, element names become object keys or struct field
names, and lists of elements become JSON arrays. We include a top-level field
like `"model_version": 1` in the JSON to allow evolution of the schema over
time.

One important design consideration is that the JSON output should be
**deterministic**. If we serialize the same `TeiDocument` twice, we should get
the exact same JSON string (apart from spacing if any). Serde generally
guarantees field ordering as defined in the struct, but we will document the
JSON format and possibly provide a canonical serialization (for example, always
sort certain map keys if we have maps, etc.) to ensure consistency. This
deterministic JSON (and XML) is helpful for content-addressing and change
detection – e.g., a given script always produces the same XML, so a hash can
identify it uniquely.

Testing round-trip integrity is crucial: we will verify that

- `TEI XML -> parse -> JSON -> emit -> TEI XML` yields the same canonical TEI
  XML (losing only insignificant differences), and

- `JSON -> serde (Rust) -> XML -> parse -> JSON` yields the same JSON
  (idempotent normalization). These invariants ensure that our projections
  don’t accidentally drop or corrupt information.

## Python Integration via PyO3 and msgspec

The Python interface to `tei-rapporteur` is designed for ease of use in
scripting and data analysis environments, without exposing the complexity of
XML or Rust internals to the user. We achieve this by presenting Python
developers with simple functions and Python data classes (via `msgspec.Struct`)
corresponding to the TEI structures. Under the hood, the Python calls into Rust
where the heavy lifting happens.

### PyO3 Wrapper (`tei_py` module)

We implement a PyO3 module (let’s call it `tei_rapporteur` for Python users)
with a set of functions to create or transform TEI data. The key functions
include:

- `parse_xml(xml_str: str) -> Document` – Parse a TEI XML string into a Python
  object. Under the hood, this uses Rust’s `parse_tei` to get a `TeiDocument`.
  We then wrap it in a PyO3 `Document` class (defined on the Rust side) or
  immediately convert it to a Python structure. The returned `Document` is a
  lightweight wrapper that holds the Rust `TeiDocument` internally.

- `emit_xml(doc: Document) -> str` – Take a `Document` (Rust-backed) and emit
  it as TEI XML string by calling Rust’s emitter. This returns the canonical
  XML.

- `validate(doc: Document) -> None` – Run Rust’s validation checks on the
  document (e.g., ID uniqueness, allowed structures). If validation fails, a
  Python exception (e.g., `ValueError`) is raised with details.

- Additional **constructor** functions for alternate input forms:

- `from_dict(py_obj: Mapping | Sequence) -> Document` – Accept a plain Python
  nested dict/list (for instance, something that could be the result of
  `json.loads` on our JSON serialization) and convert it to a `TeiDocument`.
  This uses serde via `pyo3-serde` to map Python built-ins into Rust
  structures. It allows quick testing or construction from Python without
  writing XML.

- `from_msgpack(bytes_obj: bytes) -> Document` – Accept a MessagePack binary
  (as `bytes`) and deserialize it (using `rmp_serde`) into a `TeiDocument`.
  This is a very efficient path if the Python side already has a
  `msgspec.Struct` and encodes it to bytes.

- `from_json(json_str_or_bytes) -> Document` – Similar to above, but for JSON
  text. Uses `serde_json` in Rust. (This might be slightly less efficient than
  MessagePack due to parsing text, but convenient for debugging.)

- Corresponding **output** functions to retrieve data from a `Document`:

- `to_dict(doc: Document) -> dict` – Convert the `TeiDocument` into a Python
  dictionary/list structure via `pyo3-serde`. This produces a JSON-like object
  that could be used directly or fed into other libraries.

- `to_msgpack(doc: Document) -> bytes` – Serialize the `TeiDocument` to
  MessagePack bytes (using `rmp_serde`).

- `to_json(doc: Document) -> bytes|str` – Serialize to JSON (we can allow
  returning either a Python bytes or str for the JSON text).

We also define a Python-visible class, say `Document`, using `#[pyclass]` in
Rust. This class primarily holds `inner: TeiDocument` (as in the example code)
and perhaps implements `__repr__` or other niceties. We provide `__iter__` or
data accessors only if needed, but the expectation is that users will usually
convert the document into their own `msgspec.Struct` classes for any intensive
work. The `Document` class is mostly a vessel to carry data between functions
in this minimal API approach.

Here’s a sketch of what calling the Python API might look like:

```python
import tei_rapporteur as tei

# 1. Parse an XML file into a Document
xml_str = open("episode1.tei.xml", "r", encoding="utf-8").read()
doc = tei.parse_xml(xml_str)
doc.validate()          # Raises if anything invalid

# 2. Convert Document to a Python dict (JSON form) and to a msgspec struct
episode_dict = tei.to_dict(doc)
# Suppose we have defined a msgspec.Struct corresponding to the Episode:
episode = EpisodeStruct(**episode_dict)   # or msgspec.structs aren't dataclasses but we can decode via msgspec
# (Alternatively, we could do: encoded = tei.to_msgpack(doc); episode = msgspec.msgpack.decode(encoded, type=EpisodeStruct))

# 3. Manipulate the episode via msgspec object (e.g., add an utterance)
episode.utterances.append(Utterance(speaker="guest1", content=["Thank you all for listening!"]))
# 4. Validate and emit back to TEI XML
new_doc = tei.from_struct(episode)    # Under the hood: msgspec -> bytes -> Rust
tei.validate(new_doc)
xml_out = tei.emit_xml(new_doc)
```

In this example, `from_struct` would be a convenience that detects a
`msgspec.Struct` and converts it. We haven't explicitly listed `from_struct`
above, but it can be implemented to call `msgspec.to_builtins` on the object in
Python and then call our `from_dict` internally. The reason to support
`from_struct` is to save the user from manually encoding to bytes or dict – we
can do it for them by leveraging the `msgspec` API at runtime. The PyO3
function for `from_struct` can check if the passed Python object has a
`__struct_fields__` attribute (which `msgspec.Struct` objects do) and if so, do
the conversion as shown in the pseudo-code snippet.

### Why `msgspec.Struct`?

We choose `msgspec.Struct` for the Python representation because it offers
dataclass-like convenience with built-in serialization support. By defining
Python classes that subclass `msgspec.Struct`, we get automatic `msgspec.json`
and `msgspec.msgpack` encoding/decoding for those classes. This means Python
code can easily turn a nested object tree into a bytes buffer (JSON or
MessagePack) with one call, which is exactly what we need to send data to Rust.
Likewise, when Rust returns bytes, `msgspec` can decode them right back into
class instances. This approach saves us from writing a lot of boilerplate in
PyO3 to map Python attributes to Rust fields.

Furthermore, using `msgspec.Struct` means we **do not need to maintain parallel
class hierarchies** in Python and Rust manually. We define the schema in one
place (we can even generate the Python `Struct` definitions from the Rust
definitions or vice versa). The Python classes are lightweight and do not
contain behavior – they are purely data containers. All the real logic
(parsing, validation, etc.) is in Rust. This separation aligns with our
philosophy: *Rust for core logic, Python for orchestration*. The earlier design
discussion concluded that this strategy keeps the Python side “pure” (no C
extensions needed for every class, just the one extension module) and the Rust
side idiomatic.

To illustrate, imagine we have the following `msgspec.Struct` classes in Python
corresponding to our Rust model (simplified):

```python
import msgspec

class Span(msgspec.Struct):
    id: str
    start: int
    end: int
    ana: str            # e.g., "cliche:idiom.snowclone"

class Utterance(msgspec.Struct):
    id: str
    speaker: str | None = None
    content: list[str | dict]    # str for text, dict for an inline element like hi
                                 # (which could be represented as a dict with keys 'rend' and 'content')
    
class Episode(msgspec.Struct):
    id: str
    title: str
    utterances: list[Utterance]
    spans: list[Span] = []       # maybe stand-off annotations
    model_version: int = 1
```

*(In practice, `msgspec` can also support recursive types and union types; the
inline element could be better represented as a proper nested Struct class
instead of dict, e.g., a class `Hi(msgspec.Struct)` and then content:
`list[str | Hi]`. For brevity we use dict in content above.)*

Now, a Python user can create and manipulate an `Episode` instance easily. When
it's time to send it to Rust for validation or XML export, they can do:

```python
payload = msgspec.msgpack.encode(my_episode_obj)
doc = tei_rapporteur.from_msgpack(payload)   # into Rust
tei_rapporteur.validate(doc)
xml = tei_rapporteur.emit_xml(doc)
```

This sequence does a single fast binary serialization of the whole object
(`msgspec` uses a performant native implementation), and Rust deserializes it
in one go (`rmp_serde` directly to `TeiDocument`). This **binary boundary**
approach is extremely efficient for large data because it avoids per-field
conversions across the FFI boundary. By contrast, if we tried to manually set
attributes on a PyO3 class for each field, we’d incur a lot of Python C-API
calls.

For smaller documents or convenience, we also allow passing Python dicts or
even the `Struct` object directly:

```python
doc = tei_rapporteur.from_struct(my_episode_obj)
```

In this case, `from_struct` will internally call
`msgspec.to_builtins(my_episode_obj)` to get a plain dict/list structure, then
use serde to decode that to Rust. This has an extra conversion step but can be
useful for quick scripts and is still reasonably fast (msgspec’s conversion to
builtins is implemented in Rust and is quite efficient).

#### Python API Summary

The table below summarizes the Python-facing API and how each function
exchanges data:

| Python Function           | Input (Python side)                 | Conversion Mechanism                                     | When to Use                                          |
| ------------------------- | ----------------------------------- | -------------------------------------------------------- | ---------------------------------------------------- |
| `parse_xml(xml_str)`      | XML string (TEI P5)                 | Rust parses XML (quick-xml) into `TeiDocument`           | Reading TEI files from disk                          |
| `from_dict(obj)`          | `dict`/`list` tree (JSON structure) | Serde via `pyo3_serde` to Rust `TeiDocument`             | Constructing from Python data (e.g., test cases)     |
| `from_struct(obj)`        | `msgspec.Struct` instance           | Calls `msgspec.to_builtins`, then same as above          | High-level, Pythonic import of msgspec data          |
| `from_msgpack(bytes)`     | MessagePack bytes                   | Rust uses `rmp_serde` to decode to `TeiDocument`         | Fast path for large data, or transferring via binary |
| `from_json(str_or_bytes)` | JSON string or bytes                | Rust uses `serde_json` to decode                         | When JSON text is available (slower than MsgPack)    |
| *Return: `Document`*      | *(PyO3 class wrapping data)*        | Holds Rust `TeiDocument` inside (no copy unless mutated) | Represents TEI document in Python                    |

| Python Function   | Output (Python side)       | Conversion Mechanism                                | Notes                               |
| ----------------- | -------------------------- | --------------------------------------------------- | ----------------------------------- |
| `validate(doc)`   | `None` or throws exception | Rust validation logic, throws PyErr on failure      | Checks IDs, structure, etc.         |
| `emit_xml(doc)`   | XML string                 | Rust serializes `TeiDocument` via quick-xml         | Canonical XML output                |
| `to_dict(doc)`    | `dict`/`list` (JSON-like)  | Rust uses `pyo3_serde` to convert to Python objects | For interoperability or inspection  |
| `to_msgpack(doc)` | bytes (MessagePack)        | Rust uses `rmp_serde` to encode                     | For binary storage or fast transfer |
| `to_json(doc)`    | str (JSON text) or bytes   | Rust uses `serde_json` to encode                    | Human-readable serialization        |

(Table: Python API functions provided by `tei-rapporteur` and their data
exchange mechanisms. The `Document` class is returned by parse/from functions
and consumed by validate/emit and conversion functions. In many cases, using
the MessagePack path is the most efficient for large payloads, whereas
dict/JSON paths are convenient for debugging and small data.)

This multi-path design ensures that for **small to medium documents**, one can
directly use `from_struct` or `from_dict` with negligible overhead – it “just
works” with idiomatic Python objects. For **very large documents** (imagine an
episode transcript with tens of thousands of words or numerous annotations),
the recommended approach is to use the MessagePack boundary, which does one
bulk serialization instead of many Python C-API calls. In all cases, the heavy
parsing of XML and enforcement of rules happens in Rust, keeping Python’s role
to encoding/decoding and high-level orchestration.

Another benefit: by not creating elaborate Python classes (no need for
per-element PyClasses or dataclasses), we keep the Python package lightweight.
If down the road we want to provide a more Pythonic OO interface (with methods
on classes, etc.), we can still do that – perhaps by making richer PyClasses
that wrap `TeiDocument` or parts of it. But that’s optional and can be layered
later. The current approach already gives Python users a comfortable experience
using Pydantic-like or dataclass-like objects that can be easily printed,
accessed, and even validated on the Python side if needed (msgspec can do
schema validation).

## Pull-Parser Interface (Streaming Parsing)

For certain use cases, it may be desirable to parse a TEI document
incrementally and process it element-by-element (for example, to start
analyzing or producing output from the beginning of a large transcript before
the entire file is parsed, or to reduce memory usage by not holding the entire
document tree in memory). To facilitate this, `tei-rapporteur` includes a
design for a **pull-parser interface** that yields TEI domain objects as they
are parsed.

### Design Goals for Streaming

- **Incremental processing**: The interface should allow the caller (Rust or
  Python) to obtain the next logical unit from the input stream without loading
  the whole document.

- **Retain domain structure**: Instead of low-level XML tokens, the parser
  yields high-level objects (e.g., an `Utterance` struct or a `P` paragraph
  object) that the user can work with directly. This is higher-level than
  typical SAX parsing, but lower-level than a full document parse.

- **Shared use in Rust and Python**: The implementation will be in Rust, but we
  will expose it to Python in a natural way (e.g., as an iterator or generator
  of `msgspec`-serializable objects).

- **Opt-in and possibly experimental**: Because Rust’s stable APIs for
  generators are limited, this feature might rely on unstable features or
  external crates. We will likely gate it behind a Cargo feature (e.g.,
  `streaming_parser`) so users can turn it on if needed, without affecting the
  stable core.

### Rust Implementation Approaches

Rust does not yet have stable support for yielding values from a function
(coroutines) in a synchronous context, but there are a few ways to implement a
pull-parser:

- **Manual Iterator**: Implement a struct (say `TeiStreamParser`) that contains
  an instance of `quick_xml::Reader` and some state, and implement the
  `Iterator` trait for it. The `next()` method would read from the XML until it
  completes one unit (e.g., one `<u>` element and its content) and then return
  a `TeiElement` enum or a specific struct. Internally, this requires managing
  the nesting and ensuring that when we yield an element, we don’t consume
  beyond it. Quick-xml’s low-level API gives events like “StartElement",
  “Text", “EndElement". We can leverage that: for example, on seeing `<u>`, we
  could accumulate events until the matching `</u>` and then deserialize that
  chunk (perhaps by using `quick_xml::de::from_str` on the slice representing
  the `<u>` element) into an `Utterance` struct. This might be tricky with
  borrowing, but one could copy out the slice of XML for the element.
  Alternatively, we might construct the struct by hand as events come (like
  building the string content and any inline elements).

- The manual iterator approach can be done on stable Rust and gives fine
  control. The downside is complexity in implementation.

- **Generator/Coroutine (Nightly)**: Rust nightly has a `Generator` trait and
  the `yield` keyword (unstable). We could write a generator that yields
  `TeiElement` as it parses. This would likely make the code simpler (we can
  `yield` an Utterance when done parsing it, then continue parsing for the next
  one). However, this requires nightly Rust or a generator library.

- **Async Stream**: Using an async context with something like the
  `async-stream` crate, we can create an asynchronous generator. For example,
  `async_stream::try_stream!` allows `yield`ing values within an async
  function. We could treat the file input as an async stream (or just wrap a
  synchronous read in a future) and yield parsed elements. The result would be
  a `Stream<Item = TeiElement>` that can be consumed either with async or by
  blocking. Async-stream is stable (it uses proc macros), but it introduces a
  dependency on the futures ecosystem.

We might prototype with approach (1) for full control. The end result could be
an API like:

```rust
// in tei-xml crate:
pub enum TeiEvent {
    StartDocument,
    EndDocument,
    Element(ElementEnum), // e.g., Utterance(Utterance), Paragraph(P), etc.
    // potentially other event types
}

pub struct TeiPullParser<R: std::io::BufRead> {
    reader: quick_xml::Reader<R>,
    buf: Vec<u8>,
    // ... any state needed (e.g., current depth, etc.)
}

impl<R: BufRead> Iterator for TeiPullParser<R> {
    type Item = Result<TeiEvent, TeiError>;
    fn next(&mut self) -> Option<Self::Item> {
        // ... parse next piece and return Some(...) or None at EOF
    }
}
```

This iterator would yield events. Possibly we simplify to yield only high-level
elements (skipping StartDocument, etc., unless needed for signaling). The
`ElementEnum` could be an enum of our domain objects: e.g.,
`ElementEnum::Utterance(Utterance)`, `ElementEnum::Span(Span)`, etc., for each
top-level or annotation element we care to yield. We might not yield low-level
things like an `<hi>` because that will come as part of its parent Utterance’s
inline content. Our focus is likely on yielding each utterance or each
top-level division in the body.

**Python Exposure**: To expose this to Python, we can either:

- Provide a function that returns an iterator (PyO3 can convert a Rust Iterator
  into a Python iterator if we implement the PyIterProtocol on it). We could
  have `tei_rapporteur.iter_parse(xml_str)` that returns a Python iterable. The
  iteration in Python would call back into Rust’s iterator `next()`. Each
  yielded item needs to be converted to a Python object. We have choices: we
  could yield a PyO3 `Document` or similar for each element, but that may be
  heavy. Perhaps yielding a Python dict for each element is fine (since the
  user might immediately turn it into their struct or process it). We can
  leverage `pyo3-serde` here: for each `ElementEnum` we yield, call
  `to_builtins` to get a dict, and yield that.

- Alternatively, use Python generators: PyO3 could allow writing a generator
  function in Rust, but it's not straightforward. It's easier to stick to the
  iterator protocol.

**Experimental Nature**: Because this is complex and possibly requires nightly
features, we mark it as an experimental opt-in. For example, in `Cargo.toml` of
`tei-xml`:

```toml
[features]
streaming = ["quick-xml", "generator"]
```

where `generator` is the nightly generator feature. Or we use `async-stream`
behind `streaming_async` feature.

We will clearly document that the streaming interface is available but might
require a nightly compiler or have certain limitations. The primary crate
remains focused on full-document parse.

**Use Cases**: A tool like Episodic might use the streaming parser to scan
through a TEI without loading everything, maybe to calculate timings or do a
first pass transformation. However, since Episodic likely needs the full script
in memory anyway to do planning and feedback loops, the streaming is more
useful for extremely large inputs or integrating with pipelines that prefer
streaming (e.g., if TEI input comes from a network stream).

Nonetheless, having a pull-parser aligns with Rust’s zero-cost iterators
philosophy and gives advanced users more flexibility. It sets us apart by not
just offering the all-or-nothing approach typical of many XML libraries. And
because it's built on the same data model, each yielded element is a proper
instance of our Rust structs (or easily convertible to the same) – ensuring
consistency between streaming and full parse modes.

## Validation Strategy and Data Integrity

Validation of TEI documents can be incredibly complex given the full TEI
schemas (RELAX NG grammar and Schematron rules). For our focused subset, we
enforce correctness through a combination of **Rust type structure**,
**internal checks**, and optional external schema validation for full coverage.

- **By-construction validity**: Many structural constraints are ensured simply
  by using Serde to parse into our typed structs. If the XML is not well-formed
  or violates the basic expected structure (e.g., a `<teiHeader>` is missing),
  parsing will fail immediately with an error. The Rust type definitions
  themselves act as a schema: for example, if our `TeiDocument` requires a
  `tei_header` and `text`, the absence of those in XML will cause a deserialize
  error. Similarly, if an element that should only contain certain children
  gets some other tag, quick-xml/Serde will error out unless we chose to ignore
  it.

- **Internal Rust validation**: After parsing, we invoke
  `TeiDocument::validate(&self)` which performs deeper checks that are not
  enforced by the type system:

- **Unique ID check**: Gather all `xml:id` values in the document (e.g., in
  utterances, spans, etc.) and ensure there are no duplicates. If duplicates
  exist, return a `TeiError::Validation`.

- **Reference check**: For every cross-reference (like a span’s `from`/`to`
  pointing to an `xml:id` or an XPointer into the text), verify that the target
  exists. This might involve ensuring something like “if `from="#u5:char=10"`
  then an element with id `u5` exists and has enough characters”.

- **Structural rules**: Enforce any rules of our subset that Serde doesn’t
  catch. For instance, maybe our subset says that `<body>` can contain either
  `<div>` or `<u>` directly but not both or not nested arbitrarily. If Serde
  allowed a list of a enum of `<div>|<u>`, it might accept a mix; if that’s not
  desired, validate can check and warn or error. Another example: If using
  `<spanGrp type="...">`, maybe ensure all `<span>` inside have that matching
  type context.

- **Annotation integrity**: Check that offsets in spans don’t overlap
  improperly or that spans referenced by `corresp` match an actual element id
  of an episode, etc. Some of these might be beyond simple (like we might
  ensure spans are within bounds of the referenced text length).

- **Value ranges and formats**: If we have any attributes that must be certain
  values (say @type of spanGrp must be one of known values like "cliche" or
  "accent"), we can check that against an enum or list.

These checks ensure that even if an XML passed parsing, it adheres to the
semantic rules of our TEI profile.

- **External schema validation (optional)**: We intend to provide a RELAX NG
  schema (and Schematron if needed) for the TEI Episodic profile. Rather than
  implementing a full RELAX NG validator in Rust (which would be a major
  project on its own), we can integrate external tools for users who need that
  extra guarantee:

- For example, we could include a function or a command-line tool
  `tei_validate_schema(xml_str)` that, when invoked, will run the XML through
  an external validator (like `jing` for RNG or an XSLT for Schematron). This
  would not be on by default (and certainly not in the core parse path), but as
  a utility. We might use this in our test suite or CI: after serialization,
  call out to the schema validator to double-check that our output conforms. If
  any discrepancy is found, that indicates a bug either in our model or
  understanding of the schema.

- This approach keeps the core library lean (no massive schema parsing code)
  but still provides a path to high assurance when needed. We’ll document how
  to run schema validation as part of a QA process (e.g., “if you have jing
  installed, run `tei-rapporteur --validate file.tei.xml`” which internally
  calls out).

- **Round-trip validation**: As mentioned, one key validation is that
  converting from XML to JSON and back (or vice versa) yields the same content.
  We will include property-based tests (using e.g. Rust’s `proptest` crate and
  Python’s `hypothesis`) to generate random but valid structures and ensure
  that `emit_xml(parse_xml(original_xml))` is equivalent to a canonicalized
  `original_xml`. We’ll also test the JSON idempotence. This gives us
  confidence in the correctness of our (de)serializers.

- **Evolution and Versioning**: We include a `model_version` in the data (as
  noted in the JSON). Our Rust library can handle version migrations by
  detecting an older version and upgrading it (if we introduce breaking changes
  to the JSON format in future). For example, if `model_version: 2` adds a new
  field, we might provide a function to convert v1 -> v2 (filling defaults) so
  that older JSON can still be parsed. All such migrations will be documented,
  and the version field in the JSON (and perhaps in the TEI header via
  `<revisionDesc>` or `<encodingDesc>`) will trace this. This strategy is more
  about maintaining data integrity across versions, ensuring that user data in
  a database doesn’t become unreadable after an update.

- **Database considerations**: If storing episodes in a Postgres JSONB, we’ll
  encourage use of **database constraints** to catch major issues early (for
  example, ensuring `model_version` field exists and is within a known range).
  Also, since the TEI ID might be used as a key in related tables, we enforce
  proper generation of IDs (using UUIDs or nanoid) to avoid collisions.

Overall, the validation strategy is **pragmatic**: do as much as is reasonable
in Rust (fast, on-the-fly checks), rely on external proven tools for full
schema compliance when necessary, and continuously test round-trip correctness.
By controlling the subset and documenting it, we mitigate much of TEI’s
notorious complexity.

## Performance Characteristics

Several aspects of performance have been touched on, but to summarize:

- **XML Parse/Emit Performance**: Using `quick-xml` with Serde is very fast and
  memory-efficient for XML. It avoids building a DOM; instead it streams
  through the input once to deserialize and once to serialize. Benchmarks of
  quick-xml indicate it can parse tens of megabytes of XML per second on a
  single core (exact numbers depend on hardware and data). Our subset’s
  structure (with potentially many small text nodes and frequent tags for
  utterances and spans) is a good fit for quick-xml.

- **Memory usage**: A fully parsed `TeiDocument` will use memory proportional
  to the XML size (plus overhead for pointers, enum tags, etc.). Rust’s memory
  usage should still be quite modest (likely on the order of 2-3x the XML text
  size, since we store each string content and also allocate for each struct).
  If an episode transcript is, say, 100KB of text, the in-memory Rust struct
  might be a few hundred KB. This is fine for desktop/server environments. If
  we needed to process gigabyte-scale inputs, the streaming interface would be
  the solution.

- **Python FFI overhead**: By minimizing the number of crossing points (i.e.,
  calling into Rust once with a large payload, rather than for every small
  piece of data), we drastically reduce overhead. Approaches that use one big
  serialization (like MessagePack) scale well with data size. The alternative
  approach (passing Python data structures directly) would involve many Python
  C-API calls (each field access or creation crossing the FFI boundary), which
  can be 10x or more slower for large nested data. Therefore, `tei-rapporteur`
  defaults to the efficient path whenever possible.

- **Concurrency**: The Rust code doesn’t use global state and can be made
  thread-safe. PyO3 by default ensures that our extension functions acquire the
  GIL when needed and release it during heavy Rust computations (if we mark
  them accordingly). We should ensure to release the GIL while parsing or
  emitting, so that Python threads are not blocked if, for example, parsing a
  huge file. This can be done with the `Python::allow_threads` context in PyO3
  around the call into quick-xml.

- **Parallelism**: Users could potentially parse multiple TEI files in parallel
  threads (from Python or Rust) since our code is thread-safe (no shared
  mutable state except perhaps some global config). Each parse will use one
  core; we have not implemented any internal parallelism for a single parse
  (it’s usually not necessary given the speed, but large files could
  theoretically be split among threads by divisions).

- **Annotation overhead**: In scenarios like Bromide, where we might add
  thousands of `<span>` annotations for clichés, note that stand-off markup in
  our model is just a list of `Span` objects. Handling thousands of those is
  trivial in Rust. The serialization overhead of adding them is also linear. We
  should ensure our design of storing spans (like using `Vec<Span>`) is
  efficient for lookups if needed. If one needed to frequently find which
  utterance a span refers to, an index map could be built, but that’s likely
  outside core parsing (more in analysis tools).

- **Comparison and Diff**: Because our output XML is canonicalized, simple text
  diffing might still produce noise (due to differences in position of line
  breaks, etc.). We plan to provide a small utility `tei-diff` that compares
  two TEI XML files in a semantic way (ignoring irrelevant differences). This
  isn't a direct performance issue, but it helps in testing and perhaps in user
  workflows to quickly identify meaningful differences between versions of a
  script.

In summary, `tei-rapporteur` is engineered to handle real-world podcast script
sizes comfortably and to integrate into a pipeline with minimal overhead. For
typical usage (scripts of a few thousand words, dozens of annotations),
performance will be near-instant for parse/emit (on the order of milliseconds).
Even for larger inputs (transcripts of hours of audio, say 100k+ words), it
should be within a couple of seconds to parse on modern hardware. The design
choices, such as using `msgspec` and binary serialization for the Python
bridge, are geared towards keeping the system scalable: small overhead for
small data, and linear scaling for large data with a low constant factor. There
is no magical zero-copy – data does get copied at boundaries – but those copies
are predictable and efficient in Rust (memcpy of a buffer, etc.), and we avoid
repeated conversions.

## Example Usage

This section provides a consolidated look at how one might use the
`tei-rapporteur` library from both Rust and Python, demonstrating the
end-to-end workflow.

### Rust Usage Example

Suppose we want to build a simple Rust program that reads a TEI file, ensures
it’s valid, and prints out all utterances with their speakers.

```rust
use tei_core::{TeiDocument, TeiError};
use tei_xml; // assuming tei_xml provides parse_tei function

fn main() -> Result<(), TeiError> {
    let xml_input = std::fs::read_to_string("episode.tei.xml")
        .expect("Failed to read file");
    // Parse XML into Rust struct
    let mut doc = tei_xml::parse_tei(&xml_input)?;        // TeiDocument
    doc.validate()?;                                      // ensure internal consistency
    // Iterate through utterances (assuming body contains a list of utterances for simplicity)
    for u in doc.text.body.u.iter() {                     // Let's say text.body.u: Vec<Utterance>
        let speaker = u.speaker.as_deref().unwrap_or("Narrator");
        let content_text = u.content.iter().map(|inline| {
            match inline {
                Inline::Text(t) => t.clone(),
                Inline::Hi(hi) => hi.content.join(""), // simplistic: join content of <hi>
                _ => "".to_string()
            }
        }).collect::<String>();
        println!("{}: {}", speaker, content_text);
    }
    // Modify the document (e.g., add a revision note in header)
    if let Some(revDesc) = doc.tei_header.revision_desc.as_mut() {
        revDesc.add_change("Validated and printed by tei-rapporteur example");
    }
    // Emit back to XML
    let xml_output = tei_xml::emit_tei(&doc)?;
    std::fs::write("episode_out.xml", xml_output).expect("Failed to write output");
    Ok(())
}
```

In this snippet:

- We parse the XML via `tei_xml::parse_tei`. This uses quick-xml under the hood
  and returns a `TeiDocument`.

- We call `validate()` to perform extra checks (which could return an error if
  something is wrong).

- We then use the structured data (here iterating utterances) to do something
  useful – in this case, print each utterance. This shows how natural it is to
  work with the Rust structs; no manual XML traversal, just normal Rust struct
  access.

- We modify the document (e.g., add a revision entry).

- Finally, we emit the modified document back to XML and save it.

The Rust API is designed to be intuitive for Rust developers, with typical
patterns (an error type for fallible functions, iterators for lists of things,
optional fields for optional XML components, builder methods where appropriate,
etc.). A Rust user does not need to know anything about Python or msgspec –
those concerns are completely absent in `tei-core` and `tei-xml`.

### Python Usage Example

For Python, assume we have the following `msgspec.Struct` classes defined
matching our TEI model (this could be in our library’s documentation or a
separate package):

```python
import msgspec

class Utterance(msgspec.Struct):
    id: str
    speaker: str | None = None
    content: list[str | "Hi"]  # mixed content: strings and Hi elements

class Hi(msgspec.Struct):
    rend: str | None = None
    content: list[str] = []    # content inside <hi>

class Episode(msgspec.Struct):
    id: str
    title: str
    utterances: list[Utterance]
    spans: list[dict] = []     # spans as dicts with keys id, start, end, ana
    model_version: int = 1
```

Now using `tei_rapporteur` in Python:

```python
import tei_rapporteur as tei

# Load and parse an existing TEI XML file
with open("episode1.tei.xml", "r", encoding="utf-8") as f:
    xml_data = f.read()
doc = tei.parse_xml(xml_data)            # parse into Document
tei.validate(doc)                        # ensure it's valid (raises exception if not)

# Convert to our Episode struct for easier manipulation
episode_json = tei.to_json(doc)          # get JSON text (or bytes) for the doc
episode = msgspec.json.decode(episode_json, type=Episode)

# Suppose we want to run Bromide analysis on the episode content:
# (Pseudo-code for Bromide, which produces a list of spans indicating cliches)
cliche_spans = run_bromide_analysis(episode)   # returns list of {"id": ..., "start":..., "end":..., "ana":...}
episode.spans.extend(cliche_spans)

# Convert the updated episode back to TEI
# We will send it through Rust to regenerate XML with the new spans in <standOff>
new_doc = tei.from_struct(episode)       # accept msgspec Struct, converts internally
tei.validate(new_doc)                    # validate again after modifications
new_xml = tei.emit_xml(new_doc)
with open("episode1_annotated.xml", "w", encoding="utf-8") as f:
    f.write(new_xml)
```

In this scenario:

- We parsed an XML file into a `Document` (which has the content inside, but
  not directly accessible to Python except via the API calls).

- We immediately turned that into JSON via `to_json` and decoded into our
  `Episode` struct. Now we have a fully Python-native object `episode` that we
  can interact with (e.g., pass to Bromide).

- After adding Bromide’s output (spans) to the episode, we want to generate
  updated TEI. We call `from_struct(episode)` to go back into Rust. Under the
  hood, as described, this serializes `episode` to bytes and Rust deserializes
  it.

- We validate and emit new XML from Rust.

- Finally, we save the new XML, which now contains the `<standOff>` with spans
  added by Bromide.

The above Python code shows how seamlessly a user can move between TEI (for
interchange/audit) and a Python JSON-friendly form (for analysis and
manipulation). At no point did the user have to deal with XML libraries, or
ensure their modifications keep the XML consistent – `tei-rapporteur` handles
that.

## Conclusion

`tei-rapporteur` is designed as a robust bridge between the rich structured
world of TEI and the agile world of Python data processing. By carefully
selecting a useful subset of TEI P5 and leveraging Rust’s performance and type
safety, it provides a reliable core for managing podcast scripts and
annotations. The PyO3 wrapper and `msgspec.Struct` integration ensure that
Python tools (be it data science notebooks or production pipelines) can use
this core without friction, enjoying high-level abstractions and low overhead.

Key design takeaways:

- **Single Source of Truth**: All parsing, serialization, and validation logic
  lives in one place (Rust), preventing divergence between systems.

- **Separation of Concerns**: The Rust crate structure isolates concerns (data
  model, XML I/O, Python FFI) so that each can be developed and tested in
  isolation, and developers can work on core logic without Python knowledge and
  vice versa.

- **Semantic Round-tripping**: We prioritize semantic fidelity of data over
  exact XML byte fidelity, enabling normalization that simplifies comparisons
  and ensures consistency. At the same time, the design leaves room for a
  future lossless mode if needed.

- **Performance and Scaling**: Using modern libraries like quick-xml and
  msgspec, the library scales from tiny snippets to large transcripts. The
  choice of binary boundaries and bulk operations keeps overhead low.

- **Extensibility**: Through the TEI ODD mechanism and JSON versioning, the
  model can evolve. Annotations for new analysis types (Chiltern, Ascorbic) can
  be added with minimal disruption, using stand-off markup or additional fields
  as appropriate. The code structure (with optional features for streaming,
  etc.) allows adding advanced functionality without affecting the stable core.

By grounding all podcast-related tooling on TEI-rapporteur, we ensure that
Episodic (the script generator) and analysis modules like Bromide speak the
same language – a structured, interoperable format. This design not only
fosters code reuse (one parser to serve all needs) but also provides
**auditability**: an episode script can be exported as TEI XML with all
annotations (clichés, pronunciation notes, accent metrics) embedded, offering
transparency into what the algorithms did. We foresee this being invaluable for
debugging and for end-users who want to see a “report” of the script’s
stylistic and linguistic properties.

In summary, TEI-Rapporteur is the cornerstone of a modern, multi-language
tooling stack for spoken content creation and analysis. It marries the rich
semantics of the TEI standard with the performance of Rust and the flexibility
of Python, enabling developers and content creators to work with complex
structured documents in a straightforward and efficient way. The design
outlined here will guide our implementation and ensure that we build a library
that is both powerful and pleasant to use, for years of maintainable code and
extensible features to come.

**Sources:**

- ChatGPT Project Stack Strategy (discussion on TEI toolkit design and
  integration)

- ChatGPT Rust/PyO3 Integration Strategy (detailed patterns for bridging Rust
  and Python, and TEI-specific considerations)
