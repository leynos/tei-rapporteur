# ExecPlan: Streaming Parser Performance Benchmarks

**Roadmap Reference:** Phase 3, Step 3.3 (Streaming Parser), Task 3 **Status:** Complete **Branch:** `terragon/streaming-parser-benchmarks-aqvdaf`

## Objective

Write performance benchmarks using criterion comparing the memory and time
usage of the full-document parser (`parse_xml`) versus the streaming parser
(`TeiPullParser`) for large TEI files.

## Prerequisites

- [x] `TeiPullParser` iterator implemented (commit bf34a10)
- [x] Python `iter_parse()` exposed (commit c46d273)
- [x] criterion added to workspace dependencies

## Critical Files

### Files to Modify

| File                                       | Change                                        |
| ------------------------------------------ | --------------------------------------------- |
| `/root/repo/Cargo.toml`                    | Add criterion to workspace dependencies       |
| `/root/repo/tei-xml/Cargo.toml`            | Add bench target and criterion dev-dependency |
| `/root/repo/tei-xml/src/fixtures/bench.rs` | Add benchmark fixture generator               |
| `/root/repo/Makefile`                      | Add `bench` target                            |
| `/root/repo/docs/users-guide.md`           | Add benchmarking section                      |
| `/root/repo/docs/roadmap.md`               | Mark task as complete                         |

### Files Created

| File                                              | Purpose                   |
| ------------------------------------------------- | ------------------------- |
| `/root/repo/tei-xml/benches/parser_comparison.rs` | Criterion benchmark suite |
| `/root/repo/tei-xml/examples/bench_memory.rs`     | Memory measurement helper |

Note: Behaviour-driven development (BDD) behavioural tests for the benchmark
fixtures are provided in `tei-xml/tests/benchmark_fixtures_behaviour.rs`. Unit
tests in `fixtures.rs` provide additional coverage for the benchmark fixture
generator.

## Implementation Steps

### Step 1: Add Criterion Infrastructure

**Goal:** Configure workspace for criterion benchmarks.

1. Add to `/root/repo/Cargo.toml` workspace dependencies:

   ```toml
   criterion = { version = "0.5.1", features = ["html_reports"] }
   ```

2. Update `/root/repo/tei-xml/Cargo.toml`:

   ```toml
   [dev-dependencies]
   criterion = { workspace = true }

   [[bench]]
   name = "parser_comparison"
   harness = false
   required-features = ["streaming"]
   ```

3. Add to `/root/repo/Makefile`:

   ```makefile
   bench: ## Run performance benchmarks
   	$(CARGO) bench --package tei-xml --features streaming $(BUILD_JOBS)
   ```

**Verification:** `make build` succeeds with new dependencies.

### Step 2: Implement Benchmark Fixture Generator

**Goal:** Extend `/root/repo/tei-xml/src/fixtures.rs` with scalable document
generation for benchmarks.

1. Add `BenchFixtureConfig` struct with size presets:
   - `SMALL`: 10 utterances (~2 KB)
   - `MEDIUM`: 100 utterances (~20 KB)
   - `LARGE`: 1,000 utterances (~200 KB)
   - `VERY_LARGE`: 10,000 utterances (~2 MB)

2. Implement `generate_benchmark_document(config)` that creates:
   - 2-4 speakers (host, guest1, guest2, narrator)
   - Utterances with varied but realistic text lengths
   - Interspersed paragraphs at regular intervals

3. Implement `generate_benchmark_xml(config)` wrapper.

4. Add unit tests verifying:
   - Generated documents pass validation
   - Correct utterance/paragraph counts
   - XML round-trips through streaming parser

**Verification:** `make test` passes with new fixture tests.

### Step 3: Implement Criterion Benchmarks

**Goal:** Create `/root/repo/tei-xml/benches/parser_comparison.rs`.

1. Implement three benchmark groups:

   ```rust
   fn bench_full_document_parser(c: &mut Criterion)
   fn bench_streaming_parser(c: &mut Criterion)
   fn bench_streaming_vs_full(c: &mut Criterion)
   ```

2. Each group benchmarks all four size categories with:
   - `Throughput::Bytes` for bytes/second measurement
   - `BenchmarkId` for named comparison in reports

3. Use `black_box` to prevent dead code elimination.

4. Handle strict linting with `#[expect(...)]` for benchmark `expect()` calls.

**Verification:** `make bench` runs and produces HTML report at
`target/criterion/report/index.html`.

### Step 4: Add Memory Measurement Helper

**Goal:** Create `/root/repo/tei-xml/examples/bench_memory.rs`.

1. Create example binary that:
   - Accepts `full` or `streaming` mode argument
   - Generates `VERY_LARGE` fixture
   - Parses using the selected mode
   - Prints block count to prevent optimization

2. Add `bench-memory` Makefile target using `/usr/bin/time -v`.

**Verification:** `make bench-memory` outputs "Maximum resident set size" for
both parsers.

### Step 5: Add BDD Behavioural Tests

**Goal:** Add rstest-bdd scenarios for benchmark fixture generation.

1. Create `/root/repo/tei-xml/tests/features/benchmark_fixtures.feature`:

   ```gherkin
   Feature: Benchmark fixture generation

     Scenario: Generate a small benchmark fixture
       Given a small benchmark configuration
       When a benchmark document is generated
       Then the document passes validation
       And the document contains 10 utterances

     Scenario: Generated XML parses with streaming parser
       Given a medium benchmark configuration
       When benchmark XML is generated
       And the XML is parsed with the streaming parser
       Then all events are yielded without errors
   ```

2. Implement step definitions in
   `/root/repo/tei-xml/tests/benchmark_fixtures_behaviour.rs`.

**Verification:** `make test` runs BDD scenarios successfully.

### Step 6: Update Documentation

**Goal:** Document benchmarking capability for users.

1. Add "Performance benchmarks" section to `/root/repo/docs/users-guide.md`:
   - Running benchmarks (`make bench`)
   - Interpreting results (throughput, latency)
   - Memory profiling with external tools
   - Benchmark size categories table

2. Update `/root/repo/docs/roadmap.md`:
   - Change `[ ]` to `[x]` for the benchmark task (line 170)

**Verification:** Markdown validates with `make markdownlint`.

### Step 7: Final Validation

**Goal:** Ensure all quality gates pass.

1. Run full validation suite:

   ```bash
   set -o pipefail
   make check-fmt 2>&1 | tee /tmp/check-fmt.log
   make lint 2>&1 | tee /tmp/lint.log
   make test 2>&1 | tee /tmp/test.log
   make bench 2>&1 | tee /tmp/bench.log
   ```

2. Review benchmark output for reasonable performance characteristics.

3. Commit with descriptive message referencing roadmap step.

## Design Decisions

### D1: Programmatic Fixture Generation

**Decision:** Generate benchmark documents programmatically rather than storing
large fixture files.

**Rationale:**

- Avoids repository bloat
- Allows easy adjustment of document characteristics
- Reproducible with deterministic generation
- Follows existing pattern in `fixtures.rs`

### D2: External Memory Measurement

**Decision:** Use `/usr/bin/time -v` for memory measurement rather than
integrating dhat or similar.

**Rationale:**

- Criterion does not natively support memory metrics
- External tools provide accurate peak RSS measurement
- Simpler implementation; dhat can be added later if needed
- Streaming parser's advantage is most visible with large documents

### D3: Four Size Categories

**Decision:** Benchmark with small (10), medium (100), large (1,000), and very
large (10,000) utterance counts.

**Rationale:**

- Covers realistic use cases from unit tests to multi-episode compilations
- Reveals scaling characteristics of both parsers
- Very large category demonstrates streaming parser's memory advantage

### D4: Rust-Only Benchmarks

**Decision:** Benchmark only Rust parsers, not Python bindings.

**Rationale:**

- Python binding performance includes foreign function interface (FFI) overhead,
  not parser performance
- Rust benchmarks measure core parsing capability
- Python benchmarks could be a separate follow-up task

## Risks and Mitigations

| Risk                                     | Mitigation                                            |
| ---------------------------------------- | ----------------------------------------------------- |
| Benchmark variance from system load      | Run multiple iterations; criterion handles statistics |
| Very large fixture generation overhead   | Pre-generate fixtures before benchmark timing         |
| Clippy lint violations in benchmark code | Use `#[expect(...)]` with documented reasons          |

## Acceptance Criteria

- [x] `make bench` runs criterion benchmarks for both parsers
- [x] Benchmarks cover four document size categories (small/medium/large/very
  large)
- [x] `make bench-memory` measures peak resident set size (RSS) for both parsers
- [x] Unit tests validate benchmark fixture generation
- [x] BDD behavioural tests validate benchmark fixture integration
- [x] `docs/users-guide.md` documents benchmarking
- [x] `docs/roadmap.md` marks Step 3.3 task 3 as complete
- [x] `make check-fmt`, `make lint`, `make test` all pass
