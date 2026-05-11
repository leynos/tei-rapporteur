# Add a `TEI` P5 spoken text iterator

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETED

## Purpose / big picture

Roadmap item `2.2.6` in Episodic depends on `tei-rapporteur` exposing one
authoritative interpretation of "spoken script text" for `TEI` P5 documents.
After this change, a Python caller can pass a complete `TEI` XML document
string to `tei_rapporteur.spoken_text_segments(xml)` and receive an ordered
list of typed segment objects containing normalized spoken text plus stable
provenance.

The observable outcome is that Chrono can replace local XML traversal with this
API. A human can see success by running the Python example in
`docs/users-guide.md` against a valid script containing `<sp>`, `<speaker>`,
`<p>`, `<stage>`, nested `<seg>`, and show-note `<div type="notes">` content.
The returned segments must include only performed speech, must preserve
document order, and must not double count nested inline segmentation.

The approval gate has passed. The user requested implementation on 2026-05-10,
with this document kept current as the work proceeds.

## Context and orientation

The repository is a Rust workspace with these relevant crates:

- `tei-core/` owns the canonical Rust domain model, document validation, and
  pure domain projections. It must not depend on XML parser details or Python.
- `tei-xml/` owns XML parsing, profile schema material, streaming parser
  support, and XML emission.
- `tei-py/` owns the PyO3 module and `msgspec`-friendly Python projection
  layer.
- `tei-serde/` owns JSON and `MessagePack` wrappers and schema generation.
- `tei-test-helpers/` owns shared test helpers.

The current body model is centred on `TeiBody`, `BodyBlock`, `P`, `Utterance`,
`Div`, `List`, `Item`, `Label`, `Head`, `Inline`, `Hi`, and `Pause` in
`tei-core/src/text/body/` and `tei-core/src/text/inline.rs`. The current XML
streaming parser yields `TeiEvent::BodyBlock` events from
`tei-xml/src/streaming/`, and Python exposes those events via
`tei_rapporteur.iter_parse`.

Architectural Decision Record (ADR) 006 broadens the semantic contract beyond
the currently accepted Episodic profile. The profile currently admits `<p>`,
`<u>`, `<div>`, `<list>`, `<item>`, `<label>`, `<head>`, `<hi>`, and `<pause>`,
but it does not yet admit all ADR-006 constructs: `<sp>`, drama-local
`<speaker>`, `<stage>`, `<ab>`, `<l>`, `<seg>`, `<note>`, `<ref>`, `<ptr>`,
`<bibl>`, `<gap>`, or `<break>`. That gap must be closed before valid ADR-006
fixtures can parse through the same parser/profile path as `parse_xml(...)`.

Key terms:

- A spoken segment is one returned unit of normalized text intended to be
  performed aloud.
- Provenance is a stable location for a segment, such as an `xml:id` when
  present and an XPath-like locator such as `/TEI/text/body/sp[1]/p[2]`.
- An excluded element is markup whose descendants never contribute words to
  spoken runtime, such as `<speaker>`, `<stage>`, `<note>`, `<list>`, `<item>`,
  `<label>`, `<head>`, `<ref>`, `<ptr>`, `<bibl>`, and `<div type="notes">`.
- Normalization means trimming segment edges, collapsing XML text whitespace to
  a single ASCII space, treating excluded inline elements and silent markers as
  word boundaries, preserving punctuation, and preserving text inside emphasis.

Relevant documentation and skills to consult while implementing:

- `docs/tei-rapporteur-design-document.md`
- `docs/users-guide.md`
- `docs/workspace-layout.md`
- `docs/rust-testing-with-rstest-fixtures.md`
- `docs/rstest-bdd-users-guide.md`
- `docs/rust-doctest-dry-guide.md`
- `docs/reliable-testing-in-rust-via-dependency-injection.md`
- `docs/complexity-antipatterns-and-refactoring-strategies.md`
- `execplans`, `leta`, `rust-router`, `arch-crate-design`,
  `rust-types-and-apis`, `rust-errors`, `hexagonal-architecture`, and
  `firecrawl-mcp`.

External references checked during planning:

- `https://raw.githubusercontent.com/leynos/episodic/refs/heads/docs/execplans/2-2-6-chrono-runtime-estimator/docs/adr/adr-006-chrono-spoken-text-semantics.md`
- `https://www.tei-c.org/release/doc/tei-p5-doc/en/html/DR.html`
- `https://www.tei-c.org/release/doc/tei-p5-doc/en/html/ref-sp.html`
- `https://www.tei-c.org/release/doc/tei-p5-doc/en/html/ref-u.html`
- `https://www.tei-c.org/release/doc/tei-p5-doc/en/html/ref-seg.html`
- `https://www.tei-c.org/release/doc/tei-p5-doc/en/html/ref-stage.html`
- `https://www.tei-c.org/release/doc/tei-p5-doc/en/html/ref-speaker.html`
- `https://docs.rs/quick-xml/latest/quick_xml/`
- `https://teibyexample.org/exist/tutorials/TBED08v00.htm`

## Constraints

These are hard invariants. Violation requires escalation, not workarounds.

- Do not implement until this plan is approved.
- Chrono policy stays out of this repository. `tei-rapporteur` returns
  normalized spoken segments; Chrono owns token counting, words-per-minute
  settings, duration rounding, and estimator metadata.
- Domain extraction rules live in Rust-owned code. Python exposes typed
  structures and functions, but must not reimplement spoken-text semantics.
- The domain layer in `tei-core` must not import `quick_xml`, PyO3, `msgspec`,
  filesystem, process, or other adapter concerns.
- XML parsing and profile validation remain owned by `tei-xml`; Python must call
  into Rust and must not parse XML locally.
- The new Python API must accept a complete XML document string, validate it
  through the same accepted Episodic profile path as `parse_xml(...)`, and
  return typed Python structures suitable for `make typecheck`.
- Existing public APIs such as `parse_xml`, `emit_xml`, `iter_parse`,
  `TeiBody::blocks`, `BodyBlock::Paragraph`, and `BodyBlock::Utterance` must
  remain source-compatible unless explicit approval is obtained.
- Text must be counted exactly once. Nested `<seg>` inside an enclosing spoken
  block contributes to that block, not to an additional segment.
- `<sp>` is a grouping container, not a counted text source by itself.
- `<u>` may produce one direct segment only when it has direct spoken text and
  no child spoken blocks; if child spoken blocks exist, those children define
  the segments.
- `<p>`, `<ab>`, `<l>`, and standalone `<seg>` in a spoken context may produce
  segments, subject to excluded descendants.
- Show-note divisions represented by `<div type="notes">` are excluded from
  spoken runtime extraction.
- Malformed XML, structurally invalid profile XML, and validation failures are
  hard failures. Do not fall back to raw text counting or partial parse results.
- New code files must start with module-level `//!` comments. Public Rust APIs
  need Rustdoc examples where meaningful.
- No single code file may exceed 400 lines. Split new extraction code by
  feature if necessary.
- Documentation and comments must use en-GB Oxford spelling.
- Prefer existing Makefile targets. Run relevant gates sequentially, not in
  parallel.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 35 files net, stop
  and ask for approval with a smaller decomposition.
- API surface: if an existing public Rust or Python API must change rather than
  gaining additive types/functions, stop and ask for approval.
- Dependencies: if a new external crate or Python package is required, stop and
  ask for approval. The expected path uses existing `quick-xml`, PyO3, serde,
  `rstest`, `rstest-bdd`, and `proptest`.
- Profile scope: if supporting ADR-006 requires modelling a broad set of TEI
  performance-text elements beyond `<sp>`, `<speaker>`, `<stage>`, `<ab>`,
  `<l>`, `<seg>`, `<note>`, `<ref>`, `<ptr>`, `<bibl>`, `<gap>`, and `<break>`,
  stop and split that into a separate profile ADR or plan.
- Error taxonomy: if distinguishing malformed XML from profile-invalid XML
  requires a breaking `TeiError` redesign, stop and present options.
- Test iterations: if `make test` still fails after five focused attempts on
  one failure cluster, stop and record the failure in this plan.
- Property testing: if the generator work expands beyond spoken extraction and
  schema fixtures, keep a narrow example-based suite and record the deferred
  property coverage rather than redesigning arbitrary document generation.
- Formal methods: if the implementation introduces unsafe code or a proof-like
  invariant that cannot be covered by unit, behaviour, and property tests, stop
  and decide whether `kani` or `verus` is warranted. No such need is expected.

## Risks

- Risk: the current profile does not accept several ADR-006 elements.
  Severity: high. Likelihood: certain. Mitigation: begin with an explicit
  profile/model milestone that updates ODD, Relax NG copies, parser fixtures,
  core types, and Python projections before implementing extraction.

- Risk: preserving excluded descendants in the canonical model could bloat the
  body model with elements that are only needed for extraction boundaries.
  Severity: medium. Likelihood: medium. Mitigation: model them as narrow,
  purpose-named domain variants where they affect accepted XML or traversal
  semantics, and keep policy helpers private unless callers need them.

- Risk: provenance may drift if it is computed separately in different layers.
  Severity: medium. Likelihood: medium. Mitigation: construct provenance in one
  Rust projection path and expose the same value to Rust tests and Python.

- Risk: `parse_xml` currently wraps deserialization errors as `TeiError::Xml`,
  which may not distinguish malformed XML from profile-invalid structure well
  enough for Chrono. Severity: medium. Likelihood: medium. Mitigation: add
  deterministic error categories only if they can be done additively; otherwise
  document stable message prefixes and escalate before breaking `TeiError`.

- Risk: adjacent text nodes, entity references, excluded inline elements, and
  pause-like markers can create subtle whitespace bugs. Severity: medium.
  Likelihood: high. Mitigation: put normalization in a dedicated Rust helper
  with table-driven `rstest` cases and property tests for idempotence and
  boundary handling.

- Risk: adding `BodyBlock` or `Inline` variants creates compile errors across
  projection, schema, emitter, parser, and tests. Severity: low. Likelihood:
  high. Mitigation: use compiler exhaustiveness as the work queue and keep
  commits small.

## Progress

- [x] 2026-05-10: Drafted this ExecPlan from ADR-006, repository inspection,
  Firecrawl research, and Wyvern team delegation.
- [x] 2026-05-10: Approval gate passed; user requested implementation of
  this ExecPlan and ongoing progress updates.
- [x] Milestone 1: establish failing tests and fixtures for ADR-006 semantics.
- [x] Milestone 2: closed by implementing a validated streaming adapter and
  recording full canonical body-model expansion as future work.
- [x] Milestone 3: implement spoken-segment domain projection and
  normalization.
- [x] Milestone 4: expose Rust XML and Python APIs.
- [x] Milestone 5: update documentation and schemas.
- [x] Milestone 6: run full gates and commit the approved implementation.
- [x] 2026-05-10: Addressed follow-up review comments covering spoken parser
  tag constants, validation-path tests, nested utterance tests, Python binding
  runtime-type/error coverage, and documentation style.
- [x] 2026-05-10: Rejected invalid `teiHeader` shells before returning spoken
  segments by validating the collected header subtree against `TeiHeader`.
- [x] 2026-05-10: Re-ran gates for the invalid-header fix:
  `make check-fmt`, `make lint`, `make test`, and targeted
  `markdownlint docs/execplans/tei-p5-spoken-element-iterator.md` passed.
- [x] 2026-05-10: CodeRabbit flagged duplicated header-root reset logic in
  `HeaderRecorder`; extracted `reset_if_header_root` and re-ran
  `make check-fmt`, `make lint`, and `make test`.
- [x] 2026-05-10: CodeRabbit's second follow-up requested private helper
  comments in `HeaderRecorder`; added concise Rustdoc and re-ran
  `make check-fmt`, `make lint`, and `make test`.
- [x] 2026-05-10: CodeRabbit's third follow-up requested removal of trivial
  serialization wrappers and unit coverage for `HeaderRecorder`; removed the
  wrappers, added focused unit tests, and ran the targeted header test filter.
- [x] 2026-05-10: After the final `HeaderRecorder` changes, reran
  `markdownlint docs/execplans/tei-p5-spoken-element-iterator.md`,
  `make check-fmt`, `make lint`, and `make test`; the full suite reported 382
  passing tests.
- [x] 2026-05-10: CodeRabbit's fourth pass found a real validation-state bug
  for repeated root headers. Reset `validated` with the header buffer, added
  edge-case tests for repeated headers, escaped attributes, empty headers, and
  deeper nested header content, then ran the targeted header test filter.
- [x] 2026-05-10: Re-ran `markdownlint
  docs/execplans/tei-p5-spoken-element-iterator.md`, `make check-fmt`,
  `make lint`, and `make test` after the validation-state fix; the full suite
  reported 386 passing tests.
- [x] 2026-05-10: CodeRabbit's final pass requested non-Oxford Rustdoc
  spelling. Rejected as invalid because repository instructions require en-GB
  Oxford `-ize` spelling.
- [x] 2026-05-10: Reconciled the completed ExecPlan status and Oxford spelling
  review, extracted shared PyO3 module registration setup for binding tests,
  reran local gates, and received a zero-finding CodeRabbit follow-up review.
- [x] 2026-05-10: Addressed the next review pass by replacing the remaining
  non-Oxford tokenizer spelling, extracting shared `HeaderRecorder` start-like
  element recording, enforcing TEI shell state before setting document flags,
  and consolidating negative spoken-text document tests.
- [x] 2026-05-10: Re-ran focused spoken/header tests, `markdownlint`,
  `make check-fmt`, `make lint`, `make test`, and CodeRabbit for the shell
  validation follow-up; all passed and CodeRabbit reported zero findings.
- [x] 2026-05-10: Addressed follow-up documentation crate identifiers and
  restored `sys.modules` before asserting the Python missing-structs error.
- [x] 2026-05-10: Re-ran focused PyO3 binding test, Markdown lint,
  `make check-fmt`, `make lint`, `make test`, and CodeRabbit for the
  documentation/Python follow-up; all passed and CodeRabbit reported zero
  findings.
- [x] 2026-05-11: Corrected the shipped testing acceptance criterion, extracted
  spoken segment lifecycle code from `tei-xml/src/spoken/mod.rs`, and replaced
  document-shell booleans with an ordered parser phase.
- [x] 2026-05-11: Addressed CodeRabbit's extracted-segment lifecycle follow-up
  by adding focused `SegmentCollector` unit coverage, rerunning the full local
  gates, and receiving a zero-finding CodeRabbit review.
- [x] 2026-05-11: Addressed follow-up documentation heading, test fixture
  duplication, Python `sys.modules` restoration, and acceptance-criteria
  wording comments; focused tests, full gates, and CodeRabbit all passed.
- [x] 2026-05-11: Extracted the `HeaderRecorder` start/empty element shell into
  `record_element_with_post_step` to clear the remaining CodeScene duplication
  finding.
- [x] 2026-05-11: Extracted the spoken XML parser's shared start/empty element
  entry path into `enter_start_like_element`.
- [x] 2026-05-11: Folded header element serialization into the
  `record_element_with_post_step` callbacks and removed the now-dead
  `append_header_element` helper.
- [x] 2026-05-11: Replaced the spoken parser start/empty and document-phase
  duplication clusters with `handle_element` and `advance_phase_if`.
- [x] 2026-05-11: Addressed follow-up documentation heading and header
  validation comment findings; verified the Python `sys.modules` restoration
  finding was stale for the current PyO3 API.
- [x] 2026-05-11: Attempted CodeRabbit agent review for the follow-up change;
  the service returned a recoverable rate-limit error before review started.
- [x] 2026-05-11: Added roadmap item 2.2.6 completion notes and tracing-based
  debug observability for spoken extraction parser state, segment decisions,
  errors, and latency/throughput fields.
- [x] 2026-05-11: Addressed CodeRabbit's observability follow-up by making
  duplicate or misplaced `<text>` state-machine errors more precise.
- [x] 2026-05-11: Addressed CodeRabbit's state-machine cleanup follow-up by
  making the `SawHeader` to `SawText` transition explicit.

## Surprises & discoveries

- Firecrawl confirmed that the current TEI Guidelines describe `sp` as an
  individual speech in performance text, `u` as a stretch of speech, `speaker`
  as a specialized heading or label, `stage` as stage direction, and `seg` as
  below-chunk text segmentation. This supports ADR-006's distinction between
  grouping containers, spoken leaves, inline segmentation, and excluded
  labels/directions.
- The current Episodic ODD includes the `spoken` module only for `u` and
  `pause`, and the `core` module only for `p`, `hi`, `title`, `desc`, `list`,
  `item`, `label`, and `head`. ADR-006 fixtures using `<sp>`, `<speaker>`,
  `<stage>`, `<ab>`, `<l>`, `<seg>`, `<note>`, `<ref>`, `<ptr>`, `<bibl>`,
  `<gap>`, or `<break>` will not be valid until the profile is extended.
- The existing streaming parser is intentionally high-level and yields complete
  `BodyBlock` values. It is useful prior art, but the spoken extraction API
  should not depend on Python consumers reconstructing policy from streaming
  events.
- `quick-xml` is already a good fit for the XML adapter boundary because its
  documented model is a StAX-like streaming API for large documents; no new XML
  parser dependency is expected.
- TEI ODD customizations and generated schemas are the correct place to record
  profile changes. The implementation must update both checked-in Relax NG
  copies consistently, as previous plans have recorded this as a drift source.
- The first red `make test` run on 2026-05-10 failed for the expected missing
  public symbols after the test harness was corrected:
  `tei_core::SpokenTextSegment` and `tei_xml::spoken_text_segments` do not yet
  exist. Evidence is in `/tmp/test-tei-p5-spoken-element-iterator-red.out`.
- A first green implementation run on 2026-05-10 passed `make test` with 369
  tests. Evidence is in `/tmp/test-tei-p5-spoken-element-iterator-py.out`.
- CodeRabbit review was run after the first implementation milestone. Valid
  findings were addressed through helper-module extraction, cached parser
  state, Rustdoc examples, entity-reference coverage, predicate extraction, and
  spelling fixes. The final remaining spelling request was rejected as invalid
  because this repository's `AGENTS.md` requires en-GB Oxford `-ize` spelling.
  Date: 2026-05-10.
- Documentation now describes the public Python `spoken_text_segments` API in
  `docs/users-guide.md`, the adapter boundary in
  `docs/tei-rapporteur-design-document.md`, and the internal convention in
  `docs/developers-guide.md`. Date: 2026-05-10.
- Final validation passed on 2026-05-10:
  `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
  `make nixie`. Logs are under `/tmp/*tei-p5-spoken-element-iterator-final.out`.
- A follow-up review identified that `spoken_text_segments` accepted
  `<teiHeader/>` because the parser treated header presence as profile
  validity. The parser now buffers the header subtree and deserializes it as
  `TeiHeader`; the focused regression passed with
  `cargo test -p tei-xml --test spoken_text`. Date: 2026-05-10.
- `make fmt` successfully formatted Rust but still reported unrelated
  pre-existing Markdown line-length failures in other documentation files. The
  unrelated formatting side effect in
  `docs/execplans/3-3-3-streaming-parser-performance-benchmarks.md` was
  reverted, and the touched execplan file passed targeted `markdownlint`. Date:
  2026-05-10.
- CodeRabbit's follow-up concern was limited to duplicated root-header buffer
  reset logic. Extracting a helper kept behaviour unchanged and left all
  required gates green. Date: 2026-05-10.
- CodeRabbit's second follow-up was documentation-only for private helper
  purpose and error behaviour. The comments were added without behavioural
  changes. Date: 2026-05-10.
- The `HeaderRecorder` unit tests now cover root reset, nested depth changes,
  no-op content recording outside headers, raw text/CDATA/entity recording
  inside headers, successful validation, and invalid UTF-8 failure. Date:
  2026-05-10.
- Resetting the header buffer must also reset `HeaderRecorder::validated`;
  otherwise a valid first header can leave a later invalid header looking
  accepted if parsing continues far enough to query final state. Date:
  2026-05-10.
- CodeRabbit can conflict with the repository spelling policy on
  `-ize`/`-ise` forms. For this branch, keep `Serializes` in Rustdoc because
  `AGENTS.md` and prior review resolution require en-GB Oxford spelling. Date:
  2026-05-10.

## Decision Log

- Decision: expose an additive Python API named
  `tei_rapporteur.spoken_text_segments(xml: str)` returning a list of
  `tei_rapporteur.structs.SpokenTextSegment` objects. Rationale: ADR-006 asks
  for a Python-callable API that accepts a complete XML document string. A list
  is simple for Chrono and keeps the first contract deterministic. Date:
  2026-05-10.

- Decision: define a Rust domain type for spoken output in `tei-core`, likely
  `SpokenTextSegment { text, provenance }`, and keep XML/Python conversions in
  `tei-xml` and `tei-py`. Rationale: this protects the hexagonal boundary:
  domain semantics are pure, adapters only parse XML or expose FFI. Date:
  2026-05-10.

- Decision: compute text normalization in one Rust helper shared by all
  extraction paths. Rationale: whitespace, excluded-inline boundaries, and
  pause-like markers are policy, not adapter trivia. A single helper prevents
  Chrono or Python from drifting. Date: 2026-05-10.

- Decision: use property tests for normalization and no-double-count invariants,
  but do not plan `kani` or `verus`. Rationale: this change introduces a
  range-based traversal invariant, which suits `proptest`. It does not
  introduce unsafe code or a mathematical axiom requiring deductive proof.
  Date: 2026-05-10.

- Decision: treat profile expansion as part of the feature, not as a deferred
  prerequisite. Rationale: ADR-006 requires valid `TEI` P5 fixtures.
  Implementing extraction without profile support would either reject the
  required examples or tempt a raw XML bypass. Date: 2026-05-10.

- Decision: implement the first public API as a dedicated `tei-xml` streaming
  adapter returning shared `tei-core` segment types, while deferring full
  canonical body-model expansion outside this completed plan. Rationale: the
  existing `parse_xml` model cannot deserialize `<sp>`, `<ab>`, `<l>`, `<seg>`,
  notes, stage directions, and references without broad enum and projection
  changes. The adapter still enforces a narrow ADR-006 body profile, rejects
  malformed documents and unsupported body elements, and keeps Python out of
  XML semantics. This validates the external Chrono-facing API before
  undertaking wider canonical-model work. Date: 2026-05-10.

## Implementation plan

### Milestone 1: establish executable expectations

Add failing tests before implementation. Create focused `rstest` unit tests in
`tei-core` for the pure normalization and extraction policy. Cover at least:

- `<p>Hello <seg>there</seg></p>` produces one segment, `Hello there`.
- `<sp><speaker>Host</speaker><p>First.</p><p>Second.</p></sp>` produces two
  segments and never counts `Host`.
- `<p>Hello <note>editorial</note>there.</p>` produces `Hello there.` with a
  word boundary where the note appeared.
- emphasis text contributes while `<hi>` markup does not.
- `<pause/>`, `<gap/>`, and `<break/>` contribute a boundary but no word.
- `<div type="notes"><p>Link dump</p></div>` produces no segments.
- `<u>` with direct text produces one segment, while `<u>` containing child
  spoken blocks delegates to the children.
- non-Latin text is preserved in extraction even though Chrono's first tokenizer
  may not count it.

Add behaviour tests with `rstest-bdd` where the behaviour is externally visible:

- `tei-xml/tests/features/parse_xml.feature` or a new
  `spoken_text.feature` validates full XML fixtures containing the ADR-006
  structures.
- `tei-py/tests/features/python_module.feature` adds Python scenarios for
  `spoken_text_segments`, including happy paths and malformed/profile-invalid
  XML failures.
- `python/tests/test_type_stubs.py` confirms the new Python function and
  `SpokenTextSegment` type are visible to type checking.

Expected red state: the new tests fail because the profile lacks the elements
and no spoken extraction API exists yet.

### Milestone 2: close the profile and canonical model decision

Status: closed by decision. The implemented API uses a validated `tei-xml`
streaming adapter and records full canonical body-model expansion as future
work.

Update `schemas/tei-episodic-profile.odd` to admit only the ADR-006 body and
inline constructs needed for spoken runtime extraction. Update both generated
Relax NG copies:

- `schemas/tei-episodic-profile.rng`
- `tei-xml/resources/tei-episodic-profile.rng`

Represent the new constructs in `tei-core` with the narrowest useful domain
types. Expected additions are:

- speech grouping for `<sp>` with optional speaker label and child spoken
  blocks;
- spoken block variants for `<ab>` and `<l>`;
- inline `<seg>` that contributes text inside a spoken block without creating a
  second segment;
- excluded or silent-boundary representations for `<speaker>`, `<stage>`,
  `<note>`, `<ref>`, `<ptr>`, `<bibl>`, `<gap>`, and `<break>` where they can
  appear inside otherwise accepted content.

Keep existing `P`, `Utterance`, `Div`, `List`, `Item`, `Label`, `Head`, `Hi`,
and `Pause` behaviour source-compatible. If a new enum variant is needed, let
the compiler identify projection, emitter, and validation sites that must
handle it.

### Milestone 3: implement spoken extraction in the domain

Add a pure domain projection, for example in `tei-core/src/text/spoken.rs`, and
re-export the public output type from `tei-core/src/lib.rs`.

The projection should traverse `TeiDocument::text().body()` in document order,
recursing through `Div` unless the division is `type="notes"`. It should select
the outermost counted spoken block and then normalize only included inline
content. Lists, headings, labels, notes, references, bibliography, stage
directions, speaker labels, stand-off metadata, and header metadata are
excluded.

The returned Rust type should be precise and small. A likely shape is:

```rust
pub struct SpokenTextSegment {
    text: String,
    provenance: SpokenTextProvenance,
}

pub struct SpokenTextProvenance {
    xml_id: Option<XmlId>,
    locator: String,
}
```

Keep constructors private if that helps preserve invariants. Public accessors
should return borrowed values where possible.

Normalization rules:

- trim segment edges;
- collapse XML whitespace runs to one ASCII space;
- preserve punctuation;
- preserve text inside emphasis;
- treat excluded inline descendants and silent markers as word boundaries;
- omit empty normalized segments unless the approved contract later requires
  empty pause-only segments.

Add `proptest` coverage for normalization idempotence and for generated nested
inline trees where each generated text leaf appears in at most one returned
segment.

### Milestone 4: expose Rust XML and Python APIs

In `tei-xml`, add an XML-facing helper that accepts `&str`, validates using the
same profile path as `parse_xml(...)`, and returns `Vec<SpokenTextSegment>`.
The helper should call `parse_xml(xml)?` unless an approved prototype proves a
streaming implementation can share validation without bypassing the canonical
parser.

In `tei-py`, project the Rust output into Python structures:

- add a `SpokenTextSegment` `msgspec.Struct` in `tei-py/python/structs.py` and
  matching generated/static stubs in `python/tei_rapporteur/structs.pyi`;
- expose `spoken_text_segments(xml: str)` in `tei-py/src/bindings.rs`;
- map Rust errors to Python `ValueError` using the existing `wrap_tei_result`
  pattern;
- update `python/tei_rapporteur/__init__.pyi` so type checkers see the
  function.

The Python-facing segment should expose `text: str`, `locator: str`, and
`xml_id: str | None`. If a nested provenance object is easier to evolve,
document that choice in the design document before implementing it.

### Milestone 5: update documentation and schemas

Update user-facing documentation:

- `docs/users-guide.md` documents `spoken_text_segments`, gives a short Python
  example, lists included and excluded elements, and explains error behaviour.
- `README.md` receives a concise mention only if it already lists Python public
  APIs.

Update internal documentation:

- `docs/tei-rapporteur-design-document.md` records the spoken-text projection,
  provenance choice, and boundary placement.
- `docs/developers-guide.md` should be created if it still does not exist, or
  updated if it exists, with the convention that spoken extraction semantics
  live in `tei-core` and adapters must not duplicate them.
- If implementation choices materially change ADR-006 semantics, write a new
  local ADR under `docs/adr/` and reference it from the design document. If the
  implementation merely follows ADR-006, no new ADR is required.

Regenerate any generated schema artefacts using the repository target rather
than hand-editing snapshots:

```sh
make json-schema 2>&1 | tee /tmp/json-schema-tei-rapporteur-feat-tei-spoken-iterator-plan.out
```

### Milestone 6: validate and commit

Run gates sequentially. Use `/tmp` for logs and review failures from the log
files before making another change.

For documentation changes:

```sh
make fmt 2>&1 | tee /tmp/fmt-tei-rapporteur-feat-tei-spoken-iterator-plan.out
make markdownlint 2>&1 | tee /tmp/markdownlint-tei-rapporteur-feat-tei-spoken-iterator-plan.out
make nixie 2>&1 | tee /tmp/nixie-tei-rapporteur-feat-tei-spoken-iterator-plan.out
```

For all code changes:

```sh
make check-fmt 2>&1 | tee /tmp/check-fmt-tei-rapporteur-feat-tei-spoken-iterator-plan.out
make lint 2>&1 | tee /tmp/lint-tei-rapporteur-feat-tei-spoken-iterator-plan.out
make test 2>&1 | tee /tmp/test-tei-rapporteur-feat-tei-spoken-iterator-plan.out
```

If XML profile fixtures are changed and `jing` is installed, also run:

```sh
make validate-xml 2>&1 | tee /tmp/validate-xml-tei-rapporteur-feat-tei-spoken-iterator-plan.out
```

Commit only after the relevant gates pass. Keep commits atomic:

1. profile/model support;
2. spoken extraction projection and tests;
3. Python API and type stubs;
4. documentation/schema updates if not naturally coupled to the earlier
   commits.

## Acceptance criteria

- `tei_rapporteur.spoken_text_segments(xml)` exists, is typed, and returns
  ordered `SpokenTextSegment` structures with normalized text and provenance.
- The API accepts full valid Episodic `TEI` P5 XML and rejects malformed or
  profile-invalid XML without raw-text fallback.
- `<speaker>`, `<stage>`, notes, lists, labels, headings, references,
  bibliography, show-note divisions, header metadata, stand-off metadata, and
  revision/source metadata do not contribute words.
- `<p>`, `<ab>`, `<l>`, direct spoken `<u>` content, and standalone `<seg>` in
  spoken context can produce segments.
- Nested `<seg>` inside a counted block never creates a duplicate segment.
- Excluded inline descendants and pause/gap/break-like markers create word
  boundaries but contribute no words.
- Unit and behaviour tests use `rstest`, Rust and Python unit coverage is
  present, and property tests for normalization/no-double-count invariants are
  deferred.
- `docs/users-guide.md`, `docs/tei-rapporteur-design-document.md`, and
  `docs/developers-guide.md` describe the new public and internal contracts.
- `make check-fmt`, `make lint`, and `make test` pass before any code commit.

## Outcomes & Retrospective

Implemented an additive Rust and Python spoken-text extraction API:
`tei_xml::spoken_text_segments(xml)` and
`tei_rapporteur.spoken_text_segments(xml)`. The Rust return contract is
`tei_core::SpokenTextSegment` with normalized text and provenance, while Python
callers receive `tei_rapporteur.structs.SpokenTextSegment` values with `text`,
`locator`, and `xml_id` fields.

The main planned deviation is that the first implementation uses a dedicated
streaming adapter in `tei-xml` rather than expanding the entire canonical
`TeiDocument` body model in this commit. That keeps the Chrono-facing API
available and tested while leaving full profile/model expansion as future work.
The adapter still validates the complete TEI shell, rejects malformed XML and
unsupported body elements, excludes ADR-006 non-spoken content, preserves
entity resolution, and avoids nested `<seg>` double counts.

Final gates passed:

- `make check-fmt`
- `make lint`
- `make test`
- `make markdownlint`
- `make nixie`

CodeRabbit was run repeatedly after the implementation milestone. All valid
findings were fixed; the final review after the segment-lifecycle extraction
reported zero findings.
