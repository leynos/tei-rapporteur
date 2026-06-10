# Add `<div>`, `<list>`, `<item>`, and `<label>` to the TEI body model

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`tolerances`, `risks`, `progress`, `surprises & discoveries`, `decision log`,
and `outcomes & retrospective` must be kept up to date as work proceeds.

Status: COMPLETE

## Purpose / big picture

The Text Encoding Initiative (TEI) Episodic Profile currently models the
`<body>` element as a flat sequence of `<p>` (paragraph) and `<u>` (utterance)
elements. Podcast show notes, however, require hierarchical structure: sections
(intro, interview, sponsors), bulleted lists of links, guest credits, and
timestamped chapter markers. Without `<div>`, `<list>`, `<item>`, and
`<label>`, producers must flatten this structure into paragraphs, losing
semantics that downstream tools need.

After this change, a user can:

1. Parse TEI documents containing `<div>` elements that group paragraphs,
   utterances, and lists into logical sections, each identified by `@type` and
   optionally `@xml:id`.
2. Parse `<list>` elements containing `<item>` children, where each item
   carries optional `<label>` inline content, optional `@n`, `@corresp`, and
   `@xml:id` attributes.
3. Round-trip these structures through all seven layers: Rust core types,
   XML streaming parser, XML emitter, ODD schema, Relax NG schema, Python
   `msgspec` structs, and JSON schema.
4. Validate `xml:id` uniqueness and internal pointer resolution across the
   new elements.
5. Use the Python `tei_rapporteur.structs` module to construct, inspect,
   and exchange `Div`, `List`, and `Item` objects via `MessagePack` and
   dictionary payloads.

Observable success: running `make check-fmt && make lint && make test` passes,
the new behaviour-driven development (BDD) scenarios exercise parsing,
emission, round-trip, streaming, and validation for the new elements, and
`make validate-xml` validates fixtures containing `<div>`, `<list>`, `<item>`,
and `<label>` against the updated Relax NG schema.

## Constraints

These are hard invariants. Violation requires escalation, not workarounds.

- The `BodyBlock` enum's existing `Paragraph` and `Utterance` variants must
  not change shape. All existing tests must continue to pass unmodified.
- Serde serialization of existing documents must not change. The JSON schema
  `$id` version must not be bumped (the schema is additive).
- The `json-schema` feature gate pattern must be followed: all new types in
  `tei-core` must use
  `#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]`.
- The `serde(deny_unknown_fields)` pattern used on `P` and `Utterance` must
  be applied to `Item` to reject misspelt attributes during XML deserialization.
- No file may exceed 400 lines (per `AGENTS.md`).
- Module-level `//!` doc comments are mandatory on every new module.
- Clippy `doc_markdown` is denied: wrap technical terms in backticks.
- Python `msgspec.Struct` required fields must precede optional fields (or
  use `kw_only=True`).
- en-GB-oxendict spelling in all documentation and comments.
- `rstest-bdd` v0.5.0 must be used for all BDD tests.
- JSON schema snapshots are generated, never hand-edited. Changes go in
  `tei-serde/src/schema.rs` `apply_profile_constraints()` and are regenerated
  with `make json-schema`.
- Integration test crate files that own a child module directory must use
  the `tests/<name>/mod.rs` layout to satisfy clippy `self_named_module_files`.

## Tolerances (exception triggers)

- **Scope:** if implementation requires changes to more than 25 files (net),
  stop and escalate.
- **Interface:** if any existing public API signature on `TeiBody`,
  `BodyBlock`, `P`, `Utterance`, or `TeiDocument` must change (beyond adding
  new variants/methods), stop and escalate.
- **Dependencies:** if a new external crate dependency is required, stop
  and escalate.
- **Test iterations:** if `make test` still fails after 5 focused attempts
  on a single failing test, stop and escalate.
- **Ambiguity:** `<div>` nesting is explicitly out of scope for this plan
  (deferred to 2.3.2). If a requirement implies nesting, stop and present
  options.
- **`<label>` complexity:** if `<label>` needs to support full inline
  content (nested `<hi>`, `<pause>`) beyond plain text, that is in scope. If it
  needs to support block-level content, stop and escalate.

## Risks

- **Risk:** Serde's `$value` mixed-content handling may conflict with
  `<item>` containing both a `<label>` child element and inline text. Severity:
  medium. Likelihood: medium. Mitigation: prototype `Item` serde round-trip in
  Stage B before committing to the shape. If `$value` cannot distinguish
  `<label>` from inline text, model `<label>` as a dedicated `Label` wrapper
  rather than relying on serde's untagged dispatch.

- **Risk:** The streaming parser's emphasis-nesting mechanism (`InEmphasis`
  with boxed parent) may need to be generalized for `InDiv > InList > InItem`
  nesting. Severity: medium. Likelihood: high. Mitigation: introduce separate
  `InDiv`, `InList`, `InItem` parser states that return to their parent state
  on close, following the same ownership transfer pattern used by `InEmphasis`.

- **Risk:** Adding `Div` to `BodyBlock` changes the `match` exhaustiveness
  for all existing code that matches on `BodyBlock` variants. This will cause
  compile errors in the validation module, Python projection, and streaming
  event conversion. Severity: low. Likelihood: certain. Mitigation: this is
  expected and desirable. The compiler will identify every site that needs
  updating.

- **Risk:** `proptest` arbitrary `TeiDocument` generation in `tei-serde`
  may need to be extended to cover the new types. If not extended, property
  tests may miss edge cases. Severity: low. Likelihood: high. Mitigation:
  extend the `Arbitrary` implementations for `BodyBlock` to include `Div` and
  update the `proptest` strategies.

## Progress

- [x] Stage A: understand and propose (no code changes) — this document.
- [x] Stage B: prototype serde round-trip for `Div`/`List`/`Item`/`Label`.
- [x] Stage C: Rust core types in `tei-core`.
- [x] Stage D: XML streaming parser states and handlers.
- [x] Stage E: XML emitter (custom hybrid emitter bypassing serde for body).
- [x] Stage F: ODD and Relax NG schema updates.
- [x] Stage G: Python `msgspec` structs, projection layer, and type stubs.
- [x] Stage H: JSON schema regeneration and profile constraints.
- [x] Stage I: validation updates (`xml:id` uniqueness and pointer
  resolution across `Div`, `List`, and `Item`).
- [x] Stage J: BDD behavioural tests and unit tests.
- [x] Stage K: documentation updates (user's guide, design document,
  roadmap).
- [x] Stage L: final validation and commit gating.

## Surprises & discoveries

- Stage B confirmed that `Item` can hold both an optional `Label` field and a
  `$value` `Vec<Inline>` content field without serde conflicts.
- Stage C: `PointerList::new` requires an iterator; use `["#id"]` not `"#id"`.
- Stage C: `Option::as_deref()` for `n()` can be made const by using a manual
  match expression.
- Stage E: `quick_xml` v0.39+ rejects `se::to_string` for `Vec<Inline>`
  because `Inline::Text(String)` is an untagged primitive — "consequent
  primitives would be serialized without delimiter". The plan's assumption that
  serde-based emission "just works" was wrong. A custom hybrid emitter
  (`tei-xml/src/emitter.rs`) was written: header and stand-off are emitted via
  serde (no `Vec<Inline>` fields), while the body is handwritten using direct
  string construction with XML-escaped text.
- Stage F: the externally published Relax NG snapshot lives in two places:
  `schemas/tei-episodic-profile.rng` and the embedded copy at
  `tei-xml/resources/tei-episodic-profile.rng`. Both must be updated together,
  or `write_relax_ng_schema()` drifts from the checked-in schema.
- Stage L: XML property tests need text-only `Label` generation for the same
  reason text-only paragraph and utterance strategies exist — adjacent XML text
  nodes merge on round-trip.

## Decision log

- **Decision:** Model `<label>` as a dedicated `Label` struct wrapping
  `Vec<Inline>`, not as a plain `String` field on `Item`. Rationale: TEI P5
  defines `<label>` as containing inline content (text, `<hi>`, etc.), not just
  plain text. A `String` would lose emphasis markup. A `Vec<Inline>` wrapper is
  consistent with `P` and `Utterance`. Date: 2026-04-03 (plan authoring).

- **Decision:** `DivContent` enum has variants
  `{Paragraph, Utterance, List}` — not `Div` (nesting deferred to 2.3.2).
  Rationale: the task specifies `<div>` nesting as forward-looking for 2.3.2.
  Keeping `DivContent` flat simplifies the parser state machine and avoids
  recursive type complications in serde. Date: 2026-04-03 (plan authoring).

- **Decision:** `BodyBlock` gains a single `Div(Div)` variant. Bare
  `<list>` at body level is not supported; lists appear only inside `<div>`.
  Rationale: the task specifies `DivContent` as `{List, Paragraph, Utterance}`.
  Lists at the body top level are not part of the minimum viable scope. If bare
  body-level lists are needed later, a `BodyBlock::List` variant can be added
  without breaking the `Div` design. Date: 2026-04-03 (plan authoring).

- **Decision:** Streaming parser emits `Div` as a single assembled
  `BodyBlock::Div(Div)` event, not as separate enter/exit events. Rationale:
  the existing streaming API emits `TeiEvent::BodyBlock` for each complete
  block. Splitting `Div` into enter/exit events would change the event model
  and break the Python projection. Buffering the `Div` contents is acceptable
  because `Div` elements in show notes are small (typically a heading and a
  short list). Date: 2026-04-03 (plan authoring).

- **Decision:** Replace serde-based XML emission with a custom hybrid
  emitter (`tei-xml/src/emitter.rs`). Rationale: `quick_xml` v0.39+ correctly
  rejects serialization of `Vec<Inline>` because `Inline` is
  `#[serde(untagged)]` with a `Text(String)` variant — consecutive text nodes
  would merge without delimiters. The hybrid approach delegates header and
  stand-off to serde (which handles those subtrees without issue) and
  hand-writes body emission using `escape_xml_text`. This preserves correctness
  for all element types including those containing inline content. Date:
  2026-04-05 (implementation).

## Outcomes & retrospective

- Structural body elements now round-trip end-to-end across the Rust core
  model, XML parser/emitter, Relax NG schema, JSON Schema, Python projection,
  and Python `msgspec.Struct` surface.
- Validation now traverses `Div`, `List`, and `Item` content for duplicate
  `xml:id` detection and internal pointer resolution, including `Item`
  `@corresp`.
- XML fixtures now include a `div-list` example validated with `jing` against
  the updated Relax NG schema.
- Python callers can now decode and encode `DivBlock`, `ListBlock`, `Item`,
  `Label`, and `DivEvent` values through `tei_rapporteur.structs`.
- Property-based tests now generate `Div` variants, which caught two important
  XML-specific constraints during implementation: XML-forbidden characters in
  generated `@type` values and adjacent text-node merging inside `Label`.
- Final validation completed successfully. The following commands passed:
  `make fmt`, `make json-schema`, `make check-fmt`, `make lint`, `make test`,
  `make validate-xml`, `make markdownlint`, and `make nixie`.

## Context and orientation

### Repository layout

The workspace lives at the repository root and contains these crates:

| Crate              | Path                | Purpose                                              |
| ------------------ | ------------------- | ---------------------------------------------------- |
| `tei-core`         | `tei-core/`         | Pure Rust data model, validation, no PyO3            |
| `tei-xml`          | `tei-xml/`          | XML parsing (`quick-xml`), streaming parser, emitter |
| `tei-serde`        | `tei-serde/`        | JSON/`MessagePack` wrappers, JSON schema generation  |
| `tei-py`           | `tei-py/`           | PyO3 bindings and Python projection layer            |
| `tei-test-helpers` | `tei-test-helpers/` | Shared test assertion utilities                      |

### Key terms

- **`BodyBlock`**: an enum (`tei-core/src/text/body/mod.rs:108`) with
  variants `Paragraph(P)` and `Utterance(Utterance)`. This is the unit of
  block-level content inside `<body>`.
- **`TeiBody`**: a struct (`tei-core/src/text/body/mod.rs:26`) wrapping
  `Vec<BodyBlock>` with push helpers and filter iterators.
- **Streaming parser**: `TeiPullParser` (`tei-xml/src/streaming/parser.rs`)
  — an `Iterator<Item = Result<TeiEvent, TeiError>>` with a state machine
  (`ParserState` enum in `tei-xml/src/streaming/state.rs`).
- **Python projection**: internally tagged serde enums (`PyBodyBlock`,
  `PyEvent` in `tei-py/src/projection/mod.rs` and
  `tei-py/src/projection/events.rs`) that map core types to
  `msgspec`-compatible shapes.
- **ODD**: the TEI customization file at
  `schemas/tei-episodic-profile.odd`. Module includes are on lines 74-81;
  `<body>` content model is at line 352.
- **Relax NG**: `schemas/tei-episodic-profile.rng`. The `<body>` define is
  at line 289.
- **JSON schema**: generated from `schemars` via
  `tei-serde/src/schema.rs` and `tei-serde/src/bin/generate-json-schema.rs`.
  Snapshots live under `schemas/tei-document.schema*.json`.

### Current `BodyBlock` enum (the starting point)

```rust
// tei-core/src/text/body/mod.rs:105-115
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub enum BodyBlock {
    #[serde(rename = "p")]
    Paragraph(P),
    #[serde(rename = "u")]
    Utterance(Utterance),
}
```

### Current streaming parser state machine

```plaintext
Initial → AwaitingRoot → AwaitingHeader → InHeader → AwaitingText
→ AwaitingBody → InBody → { InParagraph, InUtterance } → InBody
→ AfterBody → DocumentComplete
```

Inline content nesting: `InParagraph`/`InUtterance` can transition to
`InEmphasis` (which boxes its parent state and restores it on `</hi>`).

## Plan of work

The plan is organized into twelve stages (A through L). Each stage ends with a
verification step. Do not proceed to the next stage if verification fails.

### Stage A: understand and propose

This document. No code changes.

**Verification:** this document is reviewed and approved.

### Stage B: prototype serde round-trip for new types

Before committing to the type shapes, write a throwaway unit test in `tei-core`
that constructs a `Div` containing a `List` with two `Item` children (one with a
`Label`), serializes it to XML via `quick_xml::se`, deserializes it back, and
asserts equality. This confirms that serde's `$value` mixed-content handling
works for the chosen struct layout.

Key questions to answer:

1. Can `Item` use `#[serde(rename = "$value")]` for inline content while
   also holding an optional `Label` child element?
2. Does `Div` with `#[serde(rename = "$value")]` on `Vec<DivContent>`
   correctly round-trip through `quick_xml`?

If the prototype fails, adjust the type shapes (for example, model `Label` as a
separate serde-flattened field rather than part of `$value`) and re-test.

**Verification:** the prototype test passes with `cargo test -p tei-core`.

### Stage C: Rust core types in `tei-core`

Create the new types and integrate them into the existing body model.

#### C.1: New files

Create these new modules under `tei-core/src/text/body/`:

- `tei-core/src/text/body/div.rs` — `Div` struct and `DivContent` enum.
- `tei-core/src/text/body/list.rs` — `List` struct.
- `tei-core/src/text/body/item.rs` — `Item` struct and `Label` struct.

#### C.2: `Div` struct (`div.rs`)

```rust
/// A division grouping related body content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "div")]
pub struct Div {
    #[serde(rename = "@type")]
    div_type: String,

    #[serde(
        rename = "@xml:id", alias = "@id",
        skip_serializing_if = "Option::is_none", default
    )]
    id: Option<XmlId>,

    #[serde(rename = "$value", default)]
    content: Vec<DivContent>,
}
```

Public API:

- `Div::new(div_type: impl Into<String>) -> Result<Self, BodyContentError>`
  — rejects empty `@type` after trimming.
- `set_id`, `clear_id`, `id` — same pattern as `P` and `Utterance`.
- `div_type() -> &str`
- `content() -> &[DivContent]`
- `push_paragraph`, `push_utterance`, `push_list` — typed push helpers.
- `is_empty() -> bool`

#### C.3: `DivContent` enum (`div.rs`)

```rust
/// Content permitted inside a `<div>`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub enum DivContent {
    #[serde(rename = "p")]
    Paragraph(P),
    #[serde(rename = "u")]
    Utterance(Utterance),
    #[serde(rename = "list")]
    List(List),
}
```

#### C.4: `List` struct (`list.rs`)

```rust
/// An ordered or unordered list of items.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "list")]
pub struct List {
    #[serde(
        rename = "@xml:id", alias = "@id",
        skip_serializing_if = "Option::is_none", default
    )]
    id: Option<XmlId>,

    #[serde(rename = "$value", default)]
    items: Vec<Item>,
}
```

Public API:

- `List::new(items: impl IntoIterator<Item = Item>) -> Self`
- `set_id`, `clear_id`, `id`
- `items() -> &[Item]`
- `push_item(&mut self, item: Item)`
- `is_empty() -> bool`

#### C.5: `Item` and `Label` structs (`item.rs`)

```rust
/// A single list item, optionally prefixed by a label.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "item", deny_unknown_fields)]
pub struct Item {
    #[serde(
        rename = "@xml:id", alias = "@id",
        skip_serializing_if = "Option::is_none", default
    )]
    id: Option<XmlId>,

    #[serde(rename = "@n", skip_serializing_if = "Option::is_none", default)]
    n: Option<String>,

    #[serde(rename = "@corresp", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    corresp: Option<PointerList>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    label: Option<Label>,

    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
}

/// A label prefix for a list item, containing inline content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "label")]
pub struct Label {
    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
}
```

Public API for `Item`:

- `Item::new(...) -> Result<Self, BodyContentError>` validates non-empty
  content.
- `Item::from_text_segments(segments: impl IntoIterator<Item = S>) ->
  Result<Self, BodyContentError>`
- `set_id`, `clear_id`, `id`
- `set_n`, `n`
- `set_corresp`, `corresp`
- `set_label`, `clear_label`, `label`
- `content() -> &[Inline]`

Public API for `Label`:

- `Label::new(...) -> Result<Self, BodyContentError>` validates non-empty
  content.
- `Label::from_text(text: impl Into<String>) -> Result<Self, BodyContentError>`
- `content() -> &[Inline]`

Note on serde: the `label` field uses a named child element, not `$value`, so
it is distinguished from inline text content. If serde's `$value` creates
ordering ambiguity between `<label>` and inline text, the prototype in Stage B
will detect this. The fallback is to move `label` into `$value` as a
`DivInline` enum variant, but the named-field approach is preferred for clarity.

#### C.6: Wire into `BodyBlock` and `TeiBody`

In `tei-core/src/text/body/mod.rs`:

- Add `mod div; mod list; mod item;` declarations.
- Add
  `pub use div::{Div, DivContent}; pub use list::List; pub use item::{Item, Label};`
  to the module re-exports.
- Add `BodyBlock::Div(Div)` variant with `#[serde(rename = "div")]`.
- Add `TeiBody::push_div(&mut self, div: Div)` method.
- Add `TeiBody::divs()` filter iterator.

In `tei-core/src/lib.rs`:

- Add `Div, DivContent, Item, Label, List` to the `pub use text::{...}`
  line.

#### C.7: Unit tests

Add unit tests in each new module (`div.rs`, `list.rs`, `item.rs`) covering:

- Construction with valid content.
- Rejection of empty `@type` on `Div`.
- Rejection of empty content on `Item` and `Label`.
- `xml:id` set/clear round-trip.
- `@n`, `@corresp` setters on `Item`.
- `TeiBody` push/filter iterators for `Div`.

**Verification:** `cargo test -p tei-core` passes.
`cargo clippy -p tei-core --all-features -- -D warnings` passes.

### Stage D: XML streaming parser

Extend the streaming parser to handle `<div>`, `<list>`, `<item>`, and
`<label>` elements.

#### D.1: New parser states (`state.rs`)

Add four new variants to `ParserState`:

```rust
/// Inside a `<div>` element, accumulating block-level children.
InDiv {
    div_type: String,
    id: Option<String>,
    content: Vec<DivContent>,
},

/// Inside a `<list>` element within a `<div>`, accumulating items.
InList {
    parent_div_type: String,
    parent_div_id: Option<String>,
    parent_div_content: Vec<DivContent>,
    list_id: Option<String>,
    items: Vec<Item>,
},

/// Inside an `<item>` element, accumulating inline content.
InItem {
    parent_list: Box<ParserState>,
    item_id: Option<String>,
    item_n: Option<String>,
    item_corresp: Option<String>,
    label: Option<Label>,
    content: Vec<Inline>,
},

/// Inside a `<label>` element within an `<item>`.
InLabel {
    parent_item: Box<ParserState>,
    content: Vec<Inline>,
},
```

Update `content_mut()` to include `InItem` and `InLabel` (both hold
`Vec<Inline>` that accept inline content like text, `<hi>`, and `<pause>`).

Add constructor helpers following the existing pattern:
`ParserState::in_div(...)`, `ParserState::in_list(...)`, etc.

#### D.2: Handler methods (`handlers.rs`)

Extend `handle_body_content_start` to recognize `b"div"`:

- Extract `@type` (required) and `@xml:id` (optional).
- Transition to `ParserState::InDiv`.

Add `handle_div_content_start` for elements inside `<div>`:

- Recognize `b"p"`, `b"u"`, `b"list"`.
- For `<p>` and `<u>`: transition to `InParagraph`/`InUtterance` but
  record that the parent is a `Div` (store the div state in a parent mechanism,
  or use explicit `InDiv` restoration on close).

Design note: the simplest approach is to handle `</p>` and `</u>` inside a div
by checking whether the *prior* state context was `InDiv` rather than `InBody`.
This can be achieved by having `finish_paragraph` and `finish_utterance`
inspect the parser's nesting context. An alternative is to introduce
`InDivParagraph` and `InDivUtterance` states, but that duplicates logic. The
preferred approach is to buffer the entire `Div` by nesting: when entering a
`<div>`, push the body-level context; when encountering `<p>` or `<u>` inside
the div, transition to `InParagraph`/`InUtterance` as usual, but on close, push
the result into the `InDiv` content vector and return to `InDiv` (not `InBody`).

Implementation: store the `InDiv` state as the "return-to" state. When
`finish_paragraph` runs:

- If the parser was in `InDiv` before entering the paragraph, push
  `DivContent::Paragraph(p)` into the div's content and restore `InDiv`.
- If the parser was in `InBody`, push `BodyBlock::Paragraph(p)` and
  restore `InBody` as before.

This requires `InParagraph` and `InUtterance` to carry an optional boxed parent
state (like `InEmphasis` does), or a separate "return state" field on the
parser. The cleanest approach is to add an optional `Box<ParserState>` parent
field to `InParagraph` and `InUtterance`, defaulting to `None` for body-level
blocks (which return to `InBody`) and `Some(InDiv{...})` for div-level blocks.

Similarly, `<list>` inside `<div>` transitions to `InList`, and `<item>` inside
`<list>` transitions to `InItem`. `<label>` inside `<item>` transitions to
`InLabel`. `</label>` builds a `Label` and pushes it into the item's `label`
field.

When `</div>` is encountered: build a `Div` from the accumulated content, emit
`TeiEvent::BodyBlock(BodyBlock::Div(div))`, and restore `InBody`.

#### D.3: Helper functions (`helpers.rs`)

Add builder functions:

- `build_div(div_type: String, id: Option<String>,
  content: Vec<DivContent>) -> Result<Div, TeiError>`
- `build_list(id: Option<String>, items: Vec<Item>) -> Result<List, TeiError>`
- `build_item(id, n, corresp, label, content) -> Result<Item, TeiError>`
- `build_label(content: Vec<Inline>) -> Result<Label, TeiError>`

**Verification:** `cargo test -p tei-xml` passes, including new unit tests for
div/list/item parsing.

### Stage E: XML emitter validation

The emitter uses a custom hybrid emitter for body content (implemented in
`tei-xml/src/emitter.rs`) that handles `Div` and `List` elements through
explicit serialization. Because the hybrid emitter drives emission for body
blocks, no changes to the emitter code are needed beyond the existing `Div` and
`List` handling — the new types integrate with the existing serialization
paths. Forbidden-character checks now apply at the hybrid-emitter stage and
naturally cover `Div` content.

**Verification:** write a unit test in `tei-xml` that constructs a
`TeiDocument` with a `Div` containing a `List`, emits it via the crate's
exported emitter function (`emit_xml`), and verifies the output contains the
expected XML structure and forbidden-character handling. Run
`cargo test -p tei-xml`.

### Stage F: ODD and Relax NG schema updates

#### F.1: ODD updates (`schemas/tei-episodic-profile.odd`)

1. Update module includes (line ~75):
   - Add `div` to the `textstructure` include:
     `<moduleRef key="textstructure" include="TEI text body div"/>`
   - Add `list`, `item`, `label` to the `core` include:
     `<moduleRef key="core" include="p hi title desc list item label"/>`

2. Update `<body>` content model (line ~352) to include `<div>`:

   ```xml
   <elementSpec ident="body" mode="change">
     <desc>Contains the main content of the episode as a sequence of
       paragraphs, utterances, and divisions.</desc>
     <content>
       <alternate minOccurs="0" maxOccurs="unbounded">
         <elementRef key="p"/>
         <elementRef key="u"/>
         <elementRef key="div"/>
       </alternate>
     </content>
   </elementSpec>
   ```

3. Add `<elementSpec>` for `div`:

   ```xml
   <elementSpec ident="div" mode="change">
     <desc>A thematic or structural division of the body, such as a
       show-notes section or chapter.</desc>
     <content>
       <alternate minOccurs="0" maxOccurs="unbounded">
         <elementRef key="p"/>
         <elementRef key="u"/>
         <elementRef key="list"/>
       </alternate>
     </content>
     <attList>
       <attDef ident="xml:id" mode="change" usage="opt">
         <desc>Unique identifier for the division.</desc>
         <datatype><dataRef key="teidata.xmlName"/></datatype>
       </attDef>
       <attDef ident="type" mode="change" usage="req">
         <desc>Classification of the division (e.g., "show-notes",
           "chapter", "sponsors").</desc>
         <datatype><dataRef key="teidata.text"/></datatype>
       </attDef>
     </attList>
   </elementSpec>
   ```

4. Add `<elementSpec>` entries for `list`, `item`, and `label`:

   ```xml
   <elementSpec ident="list" mode="change">
     <desc>An ordered or unordered list of items.</desc>
     <content>
       <alternate minOccurs="0" maxOccurs="unbounded">
         <elementRef key="item"/>
       </alternate>
     </content>
     <attList>
       <attDef ident="xml:id" mode="change" usage="opt">
         <desc>Unique identifier for the list.</desc>
         <datatype><dataRef key="teidata.xmlName"/></datatype>
       </attDef>
     </attList>
   </elementSpec>

   <elementSpec ident="item" mode="change">
     <desc>A single entry within a list.</desc>
     <content>
       <sequence>
         <elementRef key="label" minOccurs="0" maxOccurs="1"/>
         <alternate minOccurs="1" maxOccurs="unbounded">
           <textNode/>
           <elementRef key="hi"/>
           <elementRef key="pause"/>
         </alternate>
       </sequence>
     </content>
     <attList>
       <attDef ident="xml:id" mode="change" usage="opt">
         <desc>Unique identifier for the item.</desc>
         <datatype><dataRef key="teidata.xmlName"/></datatype>
       </attDef>
       <attDef ident="n" mode="change" usage="opt">
         <desc>A label or number for the item.</desc>
         <datatype><dataRef key="teidata.text"/></datatype>
       </attDef>
       <attDef ident="corresp" mode="change" usage="opt">
         <desc>Pointer(s) to corresponding elements.</desc>
         <datatype><dataRef key="teidata.text"/></datatype>
       </attDef>
     </attList>
     <constraintSpec ident="item-not-empty" scheme="schematron">
       <constraint>
         <sch:ns prefix="tei" uri="http://www.tei-c.org/ns/1.0"/>
         <sch:rule context="tei:item">
           <sch:assert test="normalize-space(.) != '' or .//tei:pause">
             Item must contain non-whitespace text or a pause element.
           </sch:assert>
         </sch:rule>
       </constraint>
     </constraintSpec>
   </elementSpec>

   <elementSpec ident="label" mode="change">
     <desc>A label prefixing a list item.</desc>
     <content>
       <alternate minOccurs="1" maxOccurs="unbounded">
         <textNode/>
         <elementRef key="hi"/>
       </alternate>
     </content>
   </elementSpec>
   ```

#### F.2: Relax NG updates (`schemas/tei-episodic-profile.rng`)

1. Update the `body` define (line ~289) to include `<div>`:

   ```xml
   <define name="body">
     <element name="body" ns="http://www.tei-c.org/ns/1.0">
       <zeroOrMore>
         <choice>
           <ref name="p"/>
           <ref name="u"/>
           <ref name="div"/>
         </choice>
       </zeroOrMore>
     </element>
   </define>
   ```

2. Add new defines for `div`, `list`, `item`, and `label`:

   ```xml
   <define name="div">
     <element name="div" ns="http://www.tei-c.org/ns/1.0">
       <optional>
         <attribute name="xml:id"
                    ns="http://www.w3.org/XML/1998/namespace">
           <text/>
         </attribute>
       </optional>
       <attribute name="type">
         <text/>
       </attribute>
       <zeroOrMore>
         <choice>
           <ref name="p"/>
           <ref name="u"/>
           <ref name="list"/>
         </choice>
       </zeroOrMore>
     </element>
   </define>

   <define name="list">
     <element name="list" ns="http://www.tei-c.org/ns/1.0">
       <optional>
         <attribute name="xml:id"
                    ns="http://www.w3.org/XML/1998/namespace">
           <text/>
         </attribute>
       </optional>
       <zeroOrMore>
         <ref name="item"/>
       </zeroOrMore>
     </element>
   </define>

   <define name="item">
     <element name="item" ns="http://www.tei-c.org/ns/1.0">
       <optional>
         <attribute name="xml:id"
                    ns="http://www.w3.org/XML/1998/namespace">
           <text/>
         </attribute>
       </optional>
       <optional>
         <attribute name="n"><text/></attribute>
       </optional>
       <optional>
         <attribute name="corresp"><text/></attribute>
       </optional>
       <optional>
         <ref name="label"/>
       </optional>
       <ref name="inlineContent"/>
     </element>
   </define>

   <define name="label">
     <element name="label" ns="http://www.tei-c.org/ns/1.0">
       <oneOrMore>
         <choice>
           <text/>
           <ref name="hi"/>
         </choice>
       </oneOrMore>
     </element>
   </define>
   ```

**Verification:** `make validate-xml` passes after adding new fixtures that
exercise `<div>`, `<list>`, `<item>`, and `<label>`. This requires updating the
`generate-fixtures` binary to emit a fixture containing these elements.

### Stage G: Python projection layer

#### G.1: Rust projection types (`tei-py/src/projection/mod.rs`)

Add new variants to `PyBodyBlock`:

```rust
#[serde(rename = "div")]
Div {
    #[serde(skip_serializing_if = "Option::is_none")]
    xml_id: Option<String>,
    div_type: String,
    content: Vec<PyDivContent>,
},
```

Add a new enum `PyDivContent`:

```rust
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub(crate) enum PyDivContent {
    #[serde(rename = "paragraph")]
    Paragraph { ... },  // same shape as PyBodyBlock::Paragraph
    #[serde(rename = "utterance")]
    Utterance { ... },  // same shape as PyBodyBlock::Utterance
    #[serde(rename = "list")]
    List {
        #[serde(skip_serializing_if = "Option::is_none")]
        xml_id: Option<String>,
        items: Vec<PyItem>,
    },
}
```

Add `PyItem` and `PyLabel`:

```rust
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    xml_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    corresp: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<PyLabel>,
    content: Vec<PyInline>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PyLabel {
    content: Vec<PyInline>,
}
```

Update conversion functions:

- `py_body_block_from_core`: add `BodyBlock::Div` arm.
- `core_block_from_py`: add `PyBodyBlock::Div` arm.
- Add `py_div_content_from_core` and `core_div_content_from_py` helpers.
- Add `py_item_from_core` and `core_item_from_py` helpers.

Update `TryFrom<PyTeiBody> for TeiBody` to handle `PyBodyBlock::Div`.

#### G.2: Streaming events (`tei-py/src/projection/events.rs`)

Add a `PyEvent::Div` variant:

```rust
#[serde(rename = "div")]
Div {
    #[serde(skip_serializing_if = "Option::is_none")]
    xml_id: Option<String>,
    div_type: String,
    content: Vec<PyDivContent>,
},
```

Update `py_event_from_core` to handle `BodyBlock::Div`.

#### G.3: Python structs (`tei-py/python/structs.py`)

Add new `msgspec.Struct` classes:

```python
class Label(msgspec.Struct, omit_defaults=True):
    content: list[Inline] = []

class Item(msgspec.Struct, tag="item", tag_field="type",
           omit_defaults=True):
    content: list[Inline] = []
    xml_id: str | None = None
    n: str | None = None
    corresp: list[str] = []
    label: Label | None = None

class ListBlock(msgspec.Struct, tag="list", tag_field="type",
                omit_defaults=True):
    items: list[Item] = []
    xml_id: str | None = None

DivContent = Paragraph | Utterance | ListBlock

class DivBlock(msgspec.Struct, tag="div", tag_field="type",
               omit_defaults=True):
    div_type: str
    content: list[DivContent] = []
    xml_id: str | None = None
```

Update the `BodyBlock` TypeAlias:

```python
BodyBlock = Paragraph | Utterance | DivBlock
```

Add corresponding streaming event classes (`DivEvent`), and update the `Event`
TypeAlias.

Note: field ordering must place required fields before optional fields.
`DivBlock.div_type` (required) comes first, then optional fields. `Item` has
`content` as required (list, defaults to empty), then optional fields.

#### G.4: Python type stubs (`python/tei_rapporteur/structs.pyi`)

Mirror all new classes in the `.pyi` stub file, following the existing pattern
of `... # type: ignore[assignment]` for default field values.

**Verification:** `cargo test -p tei-py` passes, including new round-trip tests
for the projection layer.

### Stage H: JSON schema regeneration

#### H.1: Profile constraints (`tei-serde/src/schema.rs`)

Review whether any new constraints are needed in `apply_profile_constraints()`.
The `Div.div_type` field is required and non-empty by construction; if
`schemars` doesn't generate a `minLength` constraint for it, add one in the
profile constraints function.

#### H.2: Regenerate snapshots

Run `make json-schema` to regenerate `schemas/tei-document.schema.json` and the
versioned snapshot.

#### H.3: Update BDD schema tests

If `tei-serde/tests/features/json_schema.feature` has scenarios that assert
specific schema structure (e.g., counting `oneOf` variants for `BodyBlock`),
update them to reflect the new `Div` variant.

**Verification:** `cargo test -p tei-serde` passes. The schema snapshot tests
confirm the generated schema includes the new `$defs` entries.

### Stage I: validation updates

#### I.1: Body block validation (`tei-core/src/validation/mod.rs`)

Update `validate_body_blocks` to handle `BodyBlock::Div`:

- Recursively validate `Div.id` for uniqueness.
- Recursively validate `DivContent` children:
  - `DivContent::Paragraph` — record `xml:id`.
  - `DivContent::Utterance` — record `xml:id`, validate speaker.
  - `DivContent::List` — record `List.id`, then for each `Item`: record
    `Item.id`.

#### I.2: Pointer resolution (`tei-core/src/validation/pointers.rs`)

Update `validate_internal_pointers` to traverse `Div` children, validating
`@corresp` pointer lists on `Item` elements.

#### I.3: Unit tests

Add validation tests covering:

- Duplicate `xml:id` between a `Div` and a body-level `Paragraph`.
- Duplicate `xml:id` between two `Item` elements.
- Unresolved `@corresp` pointer on an `Item`.
- Valid document with `Div` containing `List` passes validation.

**Verification:** `cargo test -p tei-core` passes.

### Stage J: BDD behavioural tests

#### J.1: Core body model scenarios

Create `tei-core/tests/features/div_list.feature`:

```gherkin
Feature: Division and list body blocks

  Scenario: Construct a division with paragraphs and a list
    Given a division of type "show-notes"
    And the division contains a paragraph "Welcome to the show"
    And the division contains a list with 2 items
    When the body is assembled
    Then the body contains 1 division
    And the division contains 3 children

  Scenario: Reject division with empty type
    When a division is created with an empty type
    Then construction fails mentioning "type"

  Scenario: Reject item with empty content
    When an item is created with no content
    Then construction fails mentioning "content"
```

#### J.2: XML streaming parser scenarios

Add scenarios to `tei-xml/tests/features/streaming.feature`:

```gherkin
  Scenario: Parse division with list
    Given a streaming parser for the "div-list" TEI fixture
    When I collect all events
    Then I receive 1 BodyBlock event
    And the BodyBlock is a Div of type "show-notes"

  Scenario: Parse nested list items with labels
    Given a streaming parser for the "div-list-labels" TEI fixture
    When I collect all events
    Then the Div contains a List with 2 items
    And the first item has a label
```

#### J.3: XML emission scenarios

Add scenarios to `tei-xml/tests/features/emit_xml.feature`:

```gherkin
  Scenario: Emit division with list
    Given a document with a "show-notes" division containing a list
    When the document is emitted as XML
    Then the output contains "<div type="show-notes">"
    And the output contains "<list>"
    And the output contains "<item>"
```

#### J.4: Validation scenarios

Add to `tei-core/tests/features/validation.feature` (or create a new feature
file):

```gherkin
  Scenario: Reject duplicate xml:id across div and paragraph
    Given a document with a paragraph with id "dup"
    And a division containing an item with id "dup"
    When the document is validated
    Then validation fails mentioning "duplicate xml:id"
```

#### J.5: Round-trip scenarios

Add scenarios to `tei-serde` or `tei-xml` that parse a fixture containing
div/list/item, emit it, reparse, and assert equality.

**Verification:** `make test` passes with all new scenarios green.

### Stage K: documentation updates

#### K.1: User's guide (`docs/users-guide.md`)

Update the "Body structure" section (around line 290) to mention `<div>`,
`<list>`, `<item>`, and `<label>`:

- Add a bullet point describing the new elements.
- Add a short code example showing construction of a `Div` with a `List`.
- Update the streaming parser section to note that `Div` elements are
  emitted as single assembled `BodyBlock` events.
- Update the Python structs section to document the new classes.
- Update the validation section to mention `xml:id` uniqueness across
  `Div`, `List`, and `Item`.

#### K.2: Design document (`docs/tei-rapporteur-design-document.md`)

Update the body model prose to reflect that `BodyBlock` now has three variants.
Add a note about the decision to buffer entire `Div` elements in the streaming
parser.

#### K.3: Roadmap (`docs/roadmap.md`)

Add a new roadmap section for the structural elements feature and mark the
relevant entry as done. The roadmap currently has all Phase 1-3 items complete,
so this should be added as a new section (e.g., Phase 4 or a 2.3.x milestone
entry).

**Verification:** `make markdownlint` passes.

### Stage L: final validation and commit gating

Run the full quality gate suite:

```bash
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt.log
make lint      2>&1 | tee /tmp/lint.log
make test      2>&1 | tee /tmp/test.log
make json-schema
make validate-xml 2>&1 | tee /tmp/validate-xml.log
make markdownlint 2>&1 | tee /tmp/markdownlint.log
make fmt 2>&1 | tee /tmp/fmt.log
make nixie 2>&1 | tee /tmp/nixie.log
```

All commands must exit 0. If any fail, fix the issue and re-run the failing
command before proceeding. Once all gates pass, the change is ready for commit.

Also update the `proptest` strategies in `tei-serde` to include `Div` in
arbitrary `BodyBlock` generation, and re-run `make test` to confirm
property-based tests pass with the expanded domain.

**Verification:** all quality gate commands exit 0.

## Concrete steps

These are the exact commands to run at each stage. Working directory is the
repository root (`/home/user/project`) unless stated otherwise.

### Stage B verification

```bash
cargo test -p tei-core -- div_serde_prototype 2>&1 | tee /tmp/stage-b.log
```

Expected: the prototype test passes.

### Stage C verification

```bash
set -o pipefail
cargo test -p tei-core 2>&1 | tee /tmp/stage-c-test.log
cargo clippy -p tei-core --all-features -- -D warnings 2>&1 \
    | tee /tmp/stage-c-lint.log
```

Expected: all tests pass, no clippy warnings.

### Stage D verification

```bash
set -o pipefail
cargo test -p tei-xml 2>&1 | tee /tmp/stage-d.log
```

Expected: all tests pass, including new streaming parser tests.

### Stage F verification

```bash
make validate-xml 2>&1 | tee /tmp/stage-f.log
```

Expected: all fixtures validated successfully, including new div/list fixtures.

### Stage G verification

```bash
set -o pipefail
cargo test -p tei-py 2>&1 | tee /tmp/stage-g.log
```

Expected: all tests pass, including projection round-trip tests.

### Stage H verification

```bash
set -o pipefail
make json-schema
cargo test -p tei-serde 2>&1 | tee /tmp/stage-h.log
```

Expected: schema regenerated, all schema tests pass.

### Stage L verification (final)

```bash
set -o pipefail
make check-fmt 2>&1 | tee /tmp/final-fmt.log
make lint      2>&1 | tee /tmp/final-lint.log
make test      2>&1 | tee /tmp/final-test.log
make validate-xml 2>&1 | tee /tmp/final-xml.log
make markdownlint 2>&1 | tee /tmp/final-md.log
```

Expected: all commands exit 0.

## Validation and acceptance

Quality criteria (what "done" means):

- **Tests:** `make test` passes. All new BDD scenarios (div/list parsing,
  emission, round-trip, streaming, validation) are green. The new scenarios
  fail before the implementation and pass after (red-green cycle).
- **Lint/typecheck:** `make check-fmt` and `make lint` pass.
- **XML validation:** `make validate-xml` passes with fixtures exercising
  the new elements.
- **JSON schema:** `make json-schema` produces updated snapshots that
  include `Div`, `List`, `Item`, and `Label` definitions.
- **Markdown:** `make markdownlint` passes.
- **Documentation:** `docs/users-guide.md`, `docs/roadmap.md`, and
  `docs/tei-rapporteur-design-document.md` are updated.
- **Backward compatibility:** all pre-existing tests pass without
  modification.

Quality method (how we check):

- Run the concrete steps listed in Stage L.
- Manually review a round-trip: construct a TEI document in Python with a
  `DivBlock` containing a `ListBlock`, encode to `MessagePack`, decode back,
  and assert equality.

## Idempotence and recovery

All stages are additive. The new types and variants do not modify existing code
paths — they extend them. If a stage fails:

- Fix the issue and re-run the stage's verification command.
- Do not proceed to the next stage until the current stage passes.
- If a fundamental design issue is discovered (e.g., serde `$value`
  conflict), document it in `Decision log`, adjust the type shapes, and re-run
  from Stage B.

Running `make test` multiple times is safe and produces the same result.

## Artifacts and notes

### Example TEI fragment (target input)

```xml
<body>
  <div type="show-notes">
    <p xml:id="p1">Welcome to the show notes.</p>
    <list>
      <item n="1" xml:id="item1">
        <label>Link:</label>
        Visit our website
      </item>
      <item n="2" corresp="#guest1">
        Guest bio summary
      </item>
    </list>
  </div>
  <u who="host">And that wraps up today's episode.</u>
</body>
```

### Expected `BodyBlock` variants after parsing

```plaintext
[
  BodyBlock::Div(Div {
    div_type: "show-notes",
    id: None,
    content: [
      DivContent::Paragraph(P { id: Some("p1"), content: [...] }),
      DivContent::List(List {
        id: None,
        items: [
          Item {
            id: Some("item1"), n: Some("1"),
            label: Some(Label { content: [Text("Link:")] }),
            content: [Text("Visit our website")],
          },
          Item {
            id: None, n: Some("2"),
            corresp: Some(PointerList(["#guest1"])),
            content: [Text("Guest bio summary")],
          },
        ],
      }),
    ],
  }),
  BodyBlock::Utterance(Utterance { speaker: Some("host"), content: [...] }),
]
```

## Interfaces and dependencies

### New types (Rust)

In `tei-core/src/text/body/div.rs`:

```rust
pub struct Div { /* see Stage C.2 */ }
pub enum DivContent { Paragraph(P), Utterance(Utterance), List(List) }
```

In `tei-core/src/text/body/list.rs`:

```rust
pub struct List { /* see Stage C.4 */ }
```

In `tei-core/src/text/body/item.rs`:

```rust
pub struct Item { /* see Stage C.5 */ }
pub struct Label { /* see Stage C.5 */ }
```

### Extended types (Rust)

In `tei-core/src/text/body/mod.rs`:

```rust
pub enum BodyBlock {
    Paragraph(P),
    Utterance(Utterance),
    Div(Div),  // NEW
}
```

In `tei-core/src/text/body/mod.rs` (`TeiBody`):

```rust
pub fn push_div(&mut self, div: Div) { ... }
pub fn divs(&self) -> impl Iterator<Item = &Div> { ... }
```

### New types (Python projection)

In `tei-py/src/projection/mod.rs`:

```rust
pub(crate) enum PyDivContent { Paragraph{..}, Utterance{..}, List{..} }
pub(crate) struct PyItem { .. }
pub(crate) struct PyLabel { .. }
```

### New types (Python structs)

In `tei-py/python/structs.py`:

```python
class Label(msgspec.Struct): ...
class Item(msgspec.Struct, tag="item", tag_field="type"): ...
class ListBlock(msgspec.Struct, tag="list", tag_field="type"): ...
class DivBlock(msgspec.Struct, tag="div", tag_field="type"): ...
```

### Dependencies

No new external dependencies are required. All new functionality builds on
existing workspace dependencies: `serde`, `quick-xml`, `schemars`
(feature-gated), `thiserror`, `rstest`, `rstest-bdd` v0.5.0, `msgspec` (Python
side).

### Files to modify

| File                                     | Change                                                                    |
| ---------------------------------------- | ------------------------------------------------------------------------- |
| `tei-core/src/text/body/mod.rs`          | Add `Div` variant to `BodyBlock`, add `push_div`/`divs`, wire new modules |
| `tei-core/src/text/body/div.rs`          | NEW: `Div` and `DivContent` types                                         |
| `tei-core/src/text/body/list.rs`         | NEW: `List` type                                                          |
| `tei-core/src/text/body/item.rs`         | NEW: `Item` and `Label` types                                             |
| `tei-core/src/lib.rs`                    | Export new types                                                          |
| `tei-core/src/validation/mod.rs`         | Traverse `Div` children for validation                                    |
| `tei-core/src/validation/pointers.rs`    | Validate `@corresp` on `Item`                                             |
| `tei-xml/src/streaming/state.rs`         | Add `InDiv`, `InList`, `InItem`, `InLabel` states                         |
| `tei-xml/src/streaming/handlers.rs`      | Handle new element start/end events                                       |
| `tei-xml/src/streaming/helpers.rs`       | Add builder functions                                                     |
| `tei-xml/src/streaming/parser.rs`        | Wire new states in dispatch                                               |
| `tei-xml/src/bin/generate-fixtures.rs`   | Add div/list fixture generation                                           |
| `schemas/tei-episodic-profile.odd`       | Add `div`, `list`, `item`, `label`                                        |
| `schemas/tei-episodic-profile.rng`       | Add corresponding defines                                                 |
| `tei-py/src/projection/mod.rs`           | Add `PyDivContent`, `PyItem`, `PyLabel`, update conversions               |
| `tei-py/src/projection/events.rs`        | Add `Div` event variant                                                   |
| `tei-py/python/structs.py`               | Add `Label`, `Item`, `ListBlock`, `DivBlock` classes                      |
| `python/tei_rapporteur/structs.pyi`      | Add type stubs for new classes                                            |
| `tei-serde/src/schema.rs`                | Add constraints if needed                                                 |
| `schemas/tei-document.schema.json`       | Regenerated (not hand-edited)                                             |
| `schemas/tei-document.schema.v*.json`    | Regenerated (not hand-edited)                                             |
| `docs/users-guide.md`                    | Document new elements                                                     |
| `docs/tei-rapporteur-design-document.md` | Update body model description                                             |
| `docs/roadmap.md`                        | Mark feature as done                                                      |

### Files to create (tests)

| File                                                                                    | Purpose                                 |
| --------------------------------------------------------------------------------------- | --------------------------------------- |
| `tei-core/tests/features/div_list.feature`                                              | BDD scenarios for div/list construction |
| `tei-xml/tests/features/div_list_streaming.feature` or additions to `streaming.feature` | BDD scenarios for div/list streaming    |

### Estimated file count

Approximately 20-25 files modified or created, within the 25-file tolerance.
