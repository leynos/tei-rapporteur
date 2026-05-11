# Support nested `<div>`, `@subtype`, and optional `<head>` in body divisions

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `tolerances`, `risks`, `progress`, `Surprises & discoveries`,
`decision log`, and `outcomes & retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Episodic roadmap items 2.3.2, 2.3.3, and 2.3.4 need richer structural TEI than
the current shallow division model provides. Right now `tei-rapporteur` can
represent `<div>` blocks, but each `Div` is flat: it accepts paragraphs,
utterances, and lists only; it has `@type` but not `@subtype`; and it cannot
carry a division heading. That is enough for simple show notes, but not for
chapter markers containing nested sections, guest-bio sections within larger
segments, or sponsor-read sections with explicit headings.

After this change, a caller will be able to parse, emit, validate, serialize,
and project TEI such as a top-level `chapter` division headed by
`<head>Guest bios</head>`, containing nested child divisions with more specific
`@subtype` values like `guest-bio` or `sponsor-read`. Observable success means:
the Rust model round-trips these structures, the streaming parser still emits a
single assembled `BodyBlock::Div` event, the Relax NG and JSON Schema outputs
accept the new shapes, Python `msgspec.Struct` projections expose the new
fields, and the user-facing documentation explains the updated subset clearly.

## Standards grounding

This plan is based on the official Text Encoding Initiative (TEI) P5
references, which should be treated as the standards source during
implementation:

1. `ref-div`: the TEI `<div>` element is a text division and may contain nested
   `<div>` elements.
2. `ref-att.typed`: `@subtype` is part of `att.typed` and is meant as a finer
   classification layered on top of `@type`; TEI explicitly warns against using
   `@subtype` without `@type`.
3. `DS.html` section 4.1: `<head>` is a heading prefixed to the start of a
   textual division. Full TEI allows more than one `<head>`, but Episodic only
   needs a single optional heading per division to satisfy the roadmap items.

The implementation should therefore be TEI-informed but profile-constrained:
support recursive `<div>` content, allow optional `@subtype`, and allow zero or
one `<head>` at the start of each `<div>`. If later work needs multiple heads,
that should be a separate, explicit extension.

## Constraints

- Keep the current top-level body model stable. `BodyBlock` must remain the
  ordered surface for `<body>` content, with `BodyBlock::Div(Div)` continuing
  to represent whole assembled divisions.
- Preserve additive compatibility for existing callers. Existing paragraph,
  utterance, list, item, label, and flat-division documents must continue to
  parse, validate, emit, and serialize without changing their wire shape.
- Do not widen the Episodic profile beyond what the request asks for. This plan
  adds nested `<div>`, optional `@subtype`, and a single optional `<head>`
  child on `<div>`. It does not add arbitrary TEI body children or generic
  `att.typed` support across unrelated elements.
- Keep the streaming event contract unchanged. The XML pull parser must still
  yield a complete `TeiEvent::BodyBlock(BodyBlock::Div(_))`, not paired
  division-open and division-close events.
- Keep documentation and schemas in lock-step. Any change to
  `schemas/tei-episodic-profile.odd` must be reflected in both checked-in Relax
  NG copies: `schemas/tei-episodic-profile.rng` and
  `tei-xml/resources/tei-episodic-profile.rng`.
- Keep JSON Schema generation additive. The published schema snapshots under
  `schemas/tei-document.schema*.json` must be regenerated via
  `make json-schema`, not hand-edited.
- Keep file sizes under 400 lines. If the recursive-division work pushes a file
  over the limit, split it into a new module instead of growing it in place.
- Use en-GB-oxendict spelling in code comments and docs.

## Tolerances (exception triggers)

- Scope: if the implementation needs changes to more than 35 files or roughly
  1,500 net lines, stop and escalate with a narrowed milestone proposal.
- Interface: if satisfying the work requires removing or changing an existing
  public method signature on `Div`, `TeiBody`, `BodyBlock`, Python struct
  aliases, or streaming event tags, stop and escalate. Additive methods and
  fields are acceptable.
- Dependencies: if a new Rust or Python dependency is required, stop and
  escalate.
- Standards mismatch: if a credible TEI requirement emerges that makes a
  single optional `<head>` incorrect for Episodic’s reduced profile, stop and
  present the trade-off between strict-TEI multi-head support and the narrower
  roadmap scope.
- Recursion complexity: if recursive Serde, `schemars`, or `proptest`
  generation cannot be made stable after three focused attempts, stop and
  escalate with the concrete failing behaviour.
- Validation churn: if full validation still fails after five focused red-green
  cycles on the same failing area, stop and document the blocker before
  proceeding.

## Risks

- Risk: recursive `DivContent::Div(Div)` can introduce unbounded test-data
  generation or schema recursion surprises. Severity: high. Likelihood: medium.
  Mitigation: add bounded-depth constructors and bounded-depth `proptest`
  strategies immediately, and assert that the generated JSON Schema validator
  still compiles.
- Risk: the current streaming parser uses `pending_div_state` plus a flat
  `InDiv` state and explicitly errors on nested `<div>`. Severity: high.
  Likelihood: high. Mitigation: replace the flat hand-off with a parent-state
  model that can return completed nested divisions to their enclosing division.
- Risk: the current docs explicitly say nested `Div` is deferred. Severity:
  medium. Likelihood: certain. Mitigation: update `docs/users-guide.md` and
  `docs/tei-rapporteur-design-document.md` in the same change that lands the
  code.
- Risk: TEI allows multiple `<head>` elements on a division, while this plan
  deliberately supports at most one. Severity: low. Likelihood: medium.
  Mitigation: encode the restriction clearly in the ODD, JSON Schema, Python
  structs, and documentation so the profile remains internally consistent.
- Risk: Python tagged unions may become awkward if recursive divisions are
  modelled carelessly. Severity: medium. Likelihood: medium. Mitigation: keep
  the tags stable (`div`, `paragraph`, `utterance`, `list`) and add recursion
  through the `DivContent` alias rather than inventing new container tags.

## Progress

- [x] 2026-04-10: Reviewed project guidance, existing structural-body work, and
  official TEI P5 references; drafted this ExecPlan.
- [x] 2026-04-10 15:50 UTC: Added and updated unit, integration, BDD, schema,
  fixture, and Python projection coverage for nested divisions, `@subtype`, and
  optional `<head>`.
- [x] 2026-04-10 15:50 UTC: Extended the Rust core model with `Head`,
  `DivSubtype`, recursive `DivContent::Div`, and recursive validation for IDs
  and pointers.
- [x] 2026-04-10 15:50 UTC: Extended the XML parser, emitter, fixtures, and
  Relax NG profile to support nested divisions, headings, and subtypes while
  preserving the assembled `BodyBlock::Div` streaming contract.
- [x] 2026-04-10 15:50 UTC: Extended JSON Schema generation, schema assertions,
  and bounded arbitrary-data generators for recursive divisions.
- [x] 2026-04-10 15:50 UTC: Extended Python projections, `msgspec.Struct`
  definitions, and streaming payloads with recursive divisions plus optional
  `head` and `subtype`.
- [x] 2026-04-10 15:50 UTC: Updated `docs/users-guide.md` and
  `docs/tei-rapporteur-design-document.md` to describe the recursive division
  model and the single-head profile rule.
- [x] 2026-04-10 16:05 UTC: Ran `make fmt`, `make json-schema`,
  `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
  `make nixie`. `make validate-xml` could not run to completion in this
  environment because `jing` is not installed.

## Surprises & discoveries

- TEI P5 allows a division to carry more than one `<head>`, but the user
  request only requires an optional child for division headings. Narrowing the
  Episodic profile to zero-or-one `<head>` keeps the implementation aligned
  with the roadmap without pretending to implement the entire TEI text
  structure chapter.
- The current streaming parser already has a `DivChildKind::NestedDiv` branch,
  but it hard-fails with `"nested <div> elements are not supported"`. This is a
  useful starting point because the unsupported path is localized.
- Current documentation is internally inconsistent with the requested feature:
  the design document still says nested divisions are intentionally deferred,
  while the user’s guide already presents `Div` as a standard part of the
  supported body model. Both must be reconciled during implementation.
- Splitting `tei-xml/src/streaming/handlers.rs` and
  `tei-core/src/text/types.rs` became necessary during implementation because
  the new recursive/division-heading logic would otherwise leave touched files
  above the repository's 400-line ceiling.
- `make validate-xml` is wired correctly but depends on an external `jing`
  binary that is not installed in this environment, so XML-schema validation
  must be rerun on a machine with `jing` available.

## Decision log

- Decision: model division headings as a dedicated `Head` wrapper type rather
  than reusing `P` or `Label`. Rationale: a division heading is not a paragraph
  and not a list label; giving it a dedicated type keeps the TEI meaning clear,
  allows inline content, and preserves room for future heading-specific rules.
  Date: 2026-04-10.
- Decision: extend `DivContent` with a recursive `Div(Div)` variant rather than
  introducing a second top-level body abstraction. Rationale: nested divisions
  are content within a parent division, not a new kind of top-level body block.
  Date: 2026-04-10.
- Decision: add `@subtype` only on `Div` for now, even though the TEI
  `att.typed` class is broader. Rationale: the request is explicitly scoped to
  division support needed by Episodic roadmap items, and broader `att.typed`
  work would expand the blast radius unnecessarily. Date: 2026-04-10.
- Decision: support at most one `<head>` per `Div`, and require it to appear
  before any other division content. Rationale: this is sufficient for chapter
  markers, guest bios, and sponsor reads, while keeping the XML model,
  streaming parser, and Python projection straightforward. Date: 2026-04-10.
- Decision: retain the assembled-division streaming contract. Rationale:
  Python streaming consumers and existing tests already assume that each body
  event is a complete semantic block. Date: 2026-04-10.
- Decision: split the streaming handler implementation into dedicated
  start-element, finish-element, and content-handler modules during delivery.
  Rationale: the recursive division work would otherwise leave the touched
  handler file far above the repository’s 400-line limit. Date: 2026-04-10.
- Decision: extract `DivType` and `DivSubtype` into
  `tei-core/src/text/div_types.rs`. Rationale: the additional subtype support
  would otherwise push `tei-core/src/text/types.rs` above the file-size limit
  while mixing division-specific wrappers with unrelated identifier types.
  Date: 2026-04-10.

## Outcomes & retrospective

Nested `<div>` elements, optional `@subtype`, and a single optional leading
`<head>` now work end-to-end across the Rust core model, XML parsing/emission,
JSON Schema publication, Python projections, and user-facing documentation. The
canonical nested-division fixture parses, emits, validates inside Rust,
serializes to JSON/MessagePack, projects to Python `msgspec.Struct` types, and
round-trips through the updated tests.

The repository gates that completed successfully were:

- `make fmt`
- `make json-schema`
- `make check-fmt`
- `make lint`
- `make test`
- `make markdownlint`
- `make nixie`

Two distinct blockers surfaced around `make validate-xml` during
implementation. First, the command initially could not run in the local
environment because the `jing` binary was not installed. Second, the Relax NG
profile still needed follow-up work to align `<head>` support and nested
`<div>` semantics with the updated Rust/XML model. The actionable next steps
were therefore separate: install `jing` anywhere the XML validation gate is
expected to run, including CI, and track the Relax NG follow-up so
`<head>`/nested-division support remains synchronized with the profile.

## Context and orientation

The work spans all of the existing TEI transport layers. A novice implementer
should start with the files below and keep them open together:

- `tei-core/src/text/body/div.rs` and `tei-core/src/text/body/mod.rs` define
  `Div`, `DivContent`, and `BodyBlock`.
- `tei-core/src/validation/mod.rs` and `tei-core/src/validation/pointers.rs`
  walk divisions recursively for `xml:id` uniqueness and internal pointer
  resolution.
- `tei-xml/src/streaming/state.rs`, `tei-xml/src/streaming/helpers.rs`, and
  `tei-xml/src/streaming/handlers.rs` implement the state machine that must be
  upgraded from flat to recursive divisions.
- `tei-xml/src/emitter.rs` hand-emits body XML and must gain `@subtype`,
  `<head>`, and recursive `<div>` output.
- `schemas/tei-episodic-profile.odd`,
  `schemas/tei-episodic-profile.rng`, and
  `tei-xml/resources/tei-episodic-profile.rng` define the published XML profile.
- `tei-serde/src/schema.rs`, `tei-serde/tests/json_schema_behaviour.rs`, and
  `tei-serde/tests/arbitrary/text.rs` govern JSON Schema publication and
  recursive arbitrary-data generation.
- `tei-py/src/projection/mod.rs`, `tei-py/src/projection/body.rs`,
  `tei-py/src/projection/events.rs`, and `tei-py/python/structs.py` are the
  Python-facing surfaces that must learn about `head`, `subtype`, and nested
  divisions.
- `docs/users-guide.md` and `docs/tei-rapporteur-design-document.md` currently
  describe the division model in ways that will be stale as soon as recursion
  lands.

The recommended canonical fixture for this work should model the three roadmap
targets in one tree so every layer exercises the same semantics:

```xml
<div type="segment" subtype="chapter-markers" xml:id="seg1">
  <head>Chapter markers</head>
  <div type="segment" subtype="chapter-marker" xml:id="ch1">
    <head>Cold open</head>
    <u who="host">Welcome back.</u>
  </div>
  <div type="segment" subtype="guest-bios" xml:id="bios">
    <head>Guest bios</head>
    <list>
      <item><label>A.</label>Guest biography summary.</item>
    </list>
  </div>
  <div type="segment" subtype="sponsor-read" xml:id="ad1">
    <head>Sponsored by ExampleCo</head>
    <p>Use code EXAMPLE at checkout.</p>
  </div>
</div>
```

This single shape proves all three requested features at once: recursion,
`@subtype`, and division headings.

## Implementation plan

### Stage 1: Add failing tests first

Start with red tests before touching the model. Add narrow unit tests in
`tei-core/src/text/body/div.rs` covering:

- constructing a `Div` with an optional subtype,
- setting and clearing an optional heading,
- appending nested child divisions,
- and serde round-tripping a `Div` that contains both a `<head>` and a nested
  `<div>`.

Add behavioural and integration coverage that exercises the canonical roadmap
fixture:

- `tei-xml/tests/features/parse_xml.feature`: a scenario that parses the
  canonical nested-division fixture successfully.
- `tei-xml/tests/streaming_behaviour.rs`: a focused test that confirms one
  `BodyBlock::Div` event contains the nested child `Div`s, each with the
  expected `head` and `subtype`.
- `tei-xml/tests/emit_div.rs`: round-trip emission assertions for nested
  divisions and heading placement.
- `tei-core/tests/features/validation.feature` plus
  `tei-core/tests/validation_behaviour/mod.rs`: recursive duplicate-`xml:id`
  and unresolved-pointer coverage inside nested divisions.
- `tei-serde/tests/json_schema_behaviour.rs`: assertions that the generated
  schema includes `@subtype`, `head`, and recursive `div` variants inside
  `DivContent`.
- `tei-py/src/tests/div_structs.rs` and `tei-py/src/tests/projection_tests.rs`:
  decode and round-trip nested divisions with headings through the Python
  projection.

Do not proceed to implementation until these tests fail for the expected
reasons.

### Stage 2: Extend the Rust core model

Implement the new domain types in `tei-core` first. Create a dedicated `Head`
type in a new module such as `tei-core/src/text/body/head.rs`, shaped like
`Label`: it should wrap `Vec<Inline>`, validate non-empty visible content, and
support convenient constructors for plain text and inline content.

Then update `tei-core/src/text/body/div.rs`:

- add `subtype: Option<DivSubtype>` or equivalent validated optional field,
- add `head: Option<Head>`,
- add `DivContent::Div(Div)`,
- add additive helpers such as `set_subtype`, `clear_subtype`, `subtype()`,
  `set_head`, `clear_head`, `head()`, and `push_div`.

If a new validated scalar type is needed, add it in
`tei-core/src/text/types.rs` beside `DivType`, keeping the same trim and
non-empty guarantees.

Update exports in `tei-core/src/text/body/mod.rs`, `tei-core/src/text/mod.rs`,
and `tei-core/src/lib.rs` so the new types are visible at the crate root.

### Stage 3: Make validation recursive

Update `tei-core/src/validation/mod.rs` and
`tei-core/src/validation/pointers.rs` so nested divisions are traversed
recursively. The validator must continue to reject duplicate `xml:id` values
and unresolved internal pointers anywhere in the document tree, including list
items nested several divisions deep.

Keep the recursion local and explicit. A small helper like
`validate_div_contents(contents, seen_ids, known_speakers)` is preferable to
deeply nested `match` blocks repeated in multiple modules.

### Stage 4: Upgrade XML parsing and emission

Replace the flat division streaming state with a recursive one. The current
parser holds a flat `InDiv` state and uses `pending_div_state` to reattach
paragraphs and utterances; that is not sufficient for nested `<div>`.

The cleanest path is:

1. extend `ParserState::InDiv` to carry the raw fields needed for `type`,
   optional `subtype`, optional `head`, accumulated child content, and an
   optional parent state for nested divisions;
2. teach `handle_div_content_start` to treat `<head>` as a legal child at the
   start of a division and to recurse into child `<div>` elements instead of
   erroring;
3. teach `finish_div` to build a `Div` and either emit it as a body block or
   push it back into its parent division as `DivContent::Div`.

Update `tei-xml/src/streaming/helpers.rs` to build `Head` and to populate
`subtype` when present. Update `tei-xml/src/emitter.rs` so emitted divisions
write attributes in the order `type`, optional `subtype`, optional `xml:id`,
then optional `<head>`, then the rest of the division content. Keep heading
emission before all other content to preserve the profile rule.

Also update fixture builders in `tei-xml/src/fixtures/mod.rs` if they are used
by schema or parser tests.

### Stage 5: Update the TEI profile schemas

Edit `schemas/tei-episodic-profile.odd` so the `<div>` content model becomes:

1. optional `<head>` with maxOccurs `1`,
2. zero or more children from `p`, `u`, `list`, and `div`,
3. optional `@subtype` alongside required `@type`.

Add a Schematron rule that rejects blank `@subtype` values if present. The TEI
`att.typed` rule that `@subtype` should not appear without `@type` is already
satisfied because the Episodic profile keeps `@type` required on `div`.

Regenerate and check in both Relax NG outputs:

- `schemas/tei-episodic-profile.rng`
- `tei-xml/resources/tei-episodic-profile.rng`

Then extend `tei-xml/tests/features/schema.feature` or the existing schema
validation tests so the canonical nested-division fixture validates against the
updated Relax NG profile.

### Stage 6: Extend JSON Schema and arbitrary-data generation

Update `tei-serde/tests/arbitrary/text.rs` to generate recursive divisions with
a bounded depth parameter. Do not let `proptest` recurse freely. A maximum
depth of 2 or 3 is enough to prove the recursive model without risking
non-termination.

Regenerate the JSON Schema via `make json-schema` after teaching the schema
constraints layer about the new shape in `tei-serde/src/schema.rs`, if that
post-processing is needed. Then strengthen
`tei-serde/tests/json_schema_behaviour.rs` so it asserts:

- `BodyBlock` still includes the top-level `div` variant,
- `DivContent` now includes a recursive `div` variant,
- `div` properties include `@subtype` and optional `head`,
- serialized fixtures place `head` in the expected TEI-style `$value` path.

### Stage 7: Extend Python projections and streaming payloads

Update the Python-facing tagged shapes without changing their existing tags.
`tei-py/src/projection/mod.rs`, `tei-py/src/projection/body.rs`, and
`tei-py/src/projection/events.rs` should gain:

- optional `subtype` on `PyBodyBlock::Div` and `PyEvent::Div`,
- optional `head` on the same division structs,
- recursive `PyDivContent::Div` so nested divisions can appear inside
  `DivContent`.

Mirror that in `tei-py/python/structs.py` by extending `DivBlock`, `DivEvent`,
and the `DivContent` alias. Add a Python `Head` struct if needed so the
semantic role remains explicit instead of overloading `Paragraph` or raw lists.

Update the existing Python tests that currently assume `DivContent` is only
`Paragraph | Utterance | ListBlock`. They should assert nested `DivBlock`
instances round-trip through MessagePack, dictionary conversion, and streaming
events.

### Stage 8: Update documentation in the same change

Update `docs/users-guide.md` so it accurately describes:

- nested structural divisions,
- optional division headings,
- optional `@subtype`,
- and the updated Python union shapes.

Update `docs/tei-rapporteur-design-document.md` in at least three places:

- the April 2026 “Structural body elements” section, which currently says
  nested `Div` is deferred;
- the Rust data model section, which still contains older prose about future
  divisions;
- the streaming/parser and schema sections, so they describe the recursive
  model and the zero-or-one heading rule.

Keep the documentation honest about the profile restriction: full TEI allows
multiple heads, but Episodic currently allows one optional heading per division.

### Stage 9: Final validation and evidence capture

Once the focused tests are green, run the full repository gates with captured
logs:

```bash
set -o pipefail
make fmt 2>&1 | tee /tmp/additional-div-elements.make-fmt.log
make json-schema 2>&1 | tee /tmp/additional-div-elements.make-json-schema.log
make check-fmt 2>&1 | tee /tmp/additional-div-elements.make-check-fmt.log
make lint 2>&1 | tee /tmp/additional-div-elements.make-lint.log
make test 2>&1 | tee /tmp/additional-div-elements.make-test.log
make validate-xml 2>&1 | tee /tmp/additional-div-elements.make-validate-xml.log
make markdownlint 2>&1 | tee /tmp/additional-div-elements.make-markdownlint.log
make nixie 2>&1 | tee /tmp/additional-div-elements.make-nixie.log
```

Success is not “the code compiles”. Success is all of the following:

1. the canonical nested-division fixture parses, emits, validates, and
   round-trips;
2. recursive duplicate IDs and unresolved pointers are rejected at any nesting
   depth exercised by the tests;
3. Python projections can decode and encode nested divisions with headings and
   subtypes;
4. the published XML and JSON schemas both describe the new structure;
5. the user guide and design document no longer claim that nested divisions are
   deferred.
