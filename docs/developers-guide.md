# Developers guide

This guide records internally facing conventions that are not part of the
public user API.

## Spoken text extraction

Spoken-runtime extraction follows Architectural Decision Record (ADR) 006 and
is owned by Rust. Keep the public segment value types and normalization helpers
in `tei-core`, keep XML traversal and profile checks in `tei-xml`, and keep
Python as a thin PyO3 adapter. Python code must not reimplement TEI spoken-text
semantics or parse XML locally.

The current `tei_xml::spoken_text_segments` implementation is a streaming
adapter over `quick-xml`. It returns shared `tei_core::SpokenTextSegment`
values and enforces a narrow spoken-runtime body profile while the broader
canonical `TeiDocument` model is extended in a later milestone. When adding or
changing included, excluded, or silent-boundary elements, update the predicate
helpers in `tei-xml/src/spoken/predicates.rs`, the XML behaviour tests in
`tei-xml/tests/spoken_text.rs`, the Python binding tests, and the user guide in
the same change.

Excluded inline elements and silent markers must be represented as boundaries
that contribute no words. Do not count a nested `<seg>` twice: inline
segmentation contributes to its enclosing spoken block unless it is standalone
in a spoken context.

## XML attribute extraction

`tei-xml/src/attributes.rs` owns shared `quick-xml` attribute extraction for
the spoken-text extractor and the streaming parser. Use
`extract_normalized_attribute` for isolated lookups and
`NormalizedAttributes::from_element` when a handler needs several attributes
from the same `BytesStart`. The latter collects attributes once and serves
repeated lookups from an element-local cache, which keeps hot parser paths from
re-scanning the same start tag for each field.

The helper normalizes attributes with `XmlVersion::Implicit1_0`. The parser
states currently do not retain the XML declaration beside start-element events,
and XML 1.0 is the specification default when a declaration is absent. If the
parsers later preserve XML version state, thread that version into this module
first so spoken-text and streaming behaviour remain aligned.

## Behaviour scenario binding

Prefer `rstest-bdd` name-based scenario binding for behaviour tests. Use the
exact Gherkin `Scenario:` title in each `#[scenario(..., name = "...")]`
attribute so Rust test functions are independent of feature-file ordering. Do
not add new index-based bindings unless an upstream limitation makes them
unavoidable and the same change documents the limitation.

When splitting one feature across several Rust modules, make the module
relationship explicit in module documentation. The parent module should state
which child modules extend its fixtures or shared state, and child modules
should state which parent module they serve.

Binding guard tests that assert parity between feature-file scenarios and Rust
test bindings must call `insta::assert_debug_snapshot!` to record the expected
set of bound scenario names explicitly before the parity assertion. The
snapshot makes the binding set visible in code review, and `insta`'s diff
output surfaces regressions more clearly than a bare `assert_eq!` failure.
Commit snapshot files under the matching `tei-core/tests/**/snapshots/`
directory, such as `tei-core/tests/validation_behaviour/snapshots/` for the
validation guards, and update them with `cargo insta review` or
`INSTA_UPDATE=always cargo test` whenever binding changes are intentional.

## Spoken text observability

`tei_xml::spoken_text_segments` emits `tracing` debug events only. Library
callers own subscriber installation and export. The event schema is stable
enough for benchmark and diagnostic consumers:

- `spoken_text_parse_started`: includes `input_bytes`.
- `spoken_text_element_enter`: includes `element`, `is_empty`, `phase`, and
  `stack_depth`.
- `spoken_text_phase_transition`: includes `from` and `to`.
- `spoken_text_phase_rejected`: includes `phase`, `next`, and `error`.
- `spoken_text_unsupported_body_element`: includes `element`, `phase`, and
  `stack_depth`.
- `spoken_text_segment_started`: includes `element`, `kind`, `locator`, and
  `has_xml_id`.
- `spoken_text_segment_suppressed`: includes `element` and `locator`.
- `spoken_text_segment_emitted`: includes `element`, `locator`, and
  `text_bytes`.
- `spoken_text_parse_finished`: includes `input_bytes`, `segment_count`, and
  `elapsed_microseconds`.
- `spoken_text_parse_error`: includes `error`; parser-state failures also
  include `phase` and `stack_depth`.

Treat `input_bytes`, `segment_count`, `text_bytes`, and `elapsed_microseconds`
as the metrics surface for spoken extraction. Throughput dashboards should
derive rates from those fields rather than adding counters in the library.

## tei-py build requirements

`tei-py` uses PyO3 for the Python binding layer. The current supported PyO3
minor series is `0.28.x`, with `pyo3` pinned to `0.28.3` and the direct
`pyo3-build-config` build dependency pinned to `0.28.3`.

Keep `pyo3` and `pyo3-build-config` on the same minor series. PyO3 uses the
build configuration crate to resolve interpreter configuration and emit cfg
flags consumed by the runtime crate. Mixing minor series risks subtle build
configuration mismatches, including cfg flags or generated binding assumptions
that no longer match the runtime dependency.

The `tei-py` crate enables `auto-initialize` by default so tests and Rust-side
helpers can attach to an embedded Python interpreter without requiring every
caller to perform explicit interpreter initialization first. Wheel builds
disable default features and use only `extension-module` so PyO3 emits
extension-safe linker flags without linking `libpython`. The
`pyo3-build-config` dependency enables `resolve-config` so `tei-py/build.rs`
can resolve the active Python configuration and apply the PyO3 cfg values
needed by the crate.

The `test-support` feature enables the `test_support` module, which provides
`msgspec` bootstrapping, the process-wide Python import-state lock, and the
`with_python` synchronisation helper used by unit and BDD integration tests. It
is included in `default` features so that `cargo test` activates it
automatically. Wheel builds pass `--no-default-features` and must not activate
`test-support`; doing so would link test infrastructure into the production
extension module.

## Tei-py UI compile tests

`tei-py` uses `trybuild` as a dev-dependency for UI tests that assert
compile-time API behaviour. The harness lives in `tei-py/tests/ui.rs` and calls
`trybuild::TestCases::new().compile_fail("tests/ui/*.rs")`, so each Rust file
under `tei-py/tests/ui/` must fail to compile and must have a committed matching
`.stderr` snapshot.

Use UI tests when a runtime test cannot prove a public or hidden-public Rust
API rejects an invalid type. Name fixtures after the behaviour being guarded,
for example `non_pycallargs_rejected.rs`, and commit the generated
`non_pycallargs_rejected.stderr` beside it. To add a fixture, create the
compile-fail `.rs` file, run `cargo test -p tei-py --test ui`, inspect the
generated snapshot under `tei-py/wip/`, then move the `.stderr` file into
`tei-py/tests/ui/` only when the compiler error demonstrates the intended
contract.

The default nextest profile gives `tei-py::ui` a longer timeout than ordinary
tests because `trybuild` starts a nested Cargo build. In the
`cargo llvm-cov --target-dir ...` CI path, `cargo-llvm-cov` redirects compiled
artefacts without exporting that directory as `CARGO_TARGET_DIR` for child
Cargo processes. The nested `trybuild` invocation therefore cannot see the
coverage wrapper's target directory, starts cold, and can overrun the ordinary
nextest timeout. Export `CARGO_TARGET_DIR` before launching `cargo test` in CI
when the `cargo-llvm-cov` wrapper is in use; that gives the nested build the
same target directory as the outer build. `.config/nextest.toml` allows the UI
harness five minutes so this wrapper-specific cold-build path does not fail
before the shared target directory workaround is applied.

The first fixture, `tei-py/tests/ui/non_pycallargs_rejected.rs`, verifies that
`run_with_kwargs` rejects a plain `String` because `String` does not implement
`RunWithKwargsArgs<'py>`.

## tei-py test-support API

`tei-py/src/test_support/bootstrap.rs` contains the hidden-public
`run_with_kwargs` helper used by the `msgspec` bootstrap path and UI compile
tests:

```rust
#[doc(hidden)]
pub fn run_with_kwargs<'py, A>(
    run: &Bound<'py, PyAny>,
    args: A,
    kwargs: &Bound<'py, PyDict>,
)
where
    A: RunWithKwargsArgs<'py>,
```

`run_with_kwargs` is `#[doc(hidden)] pub` rather than `pub(crate)` because
trybuild compiles each `tests/ui/*.rs` fixture as an **independent external
crate**; `pub(crate)` items are invisible to external crates, so the fixture's
`use tei_py::test_support::run_with_kwargs` would fail to resolve without full
`pub` visibility. `#[doc(hidden)]` suppresses the item from rustdoc so it does
not appear as a documented stable API.

Any future caller must pass an argument value that implements
`RunWithKwargsArgs<'py>`. That wrapper delegates to PyO3's `PyCallArgs` bound
used by `PyAnyMethods::call`, so the helper stays hidden-public while the docs
keep the crate-owned trait name. When the intended Python call receives one
positional argument, wrap that argument in a Rust one-tuple, such as
`(args_tuple,)`.

`RunWithKwargsArgs<'py>` is deliberately single-use. It exists so this crate,
not PyO3, owns the `#[diagnostic::on_unimplemented]` message that drives the
compile-fail snapshot. Binding `run_with_kwargs` directly to
`pyo3::call::PyCallArgs<'py>` would make the expected stderr depend on PyO3's
diagnostic wording, so a PyO3 minor release could silently break the committed
snapshot without any `tei-py` API change. The diagnostic notes on
`RunWithKwargsArgs<'py>` are therefore the source of
`non_pycallargs_rejected.stderr`, not a copy of upstream output. Do not remove
the wrapper unless the UI test is intentionally moved to a different
crate-owned compile-fail boundary.

Only `ensure_msgspec_available` and `with_python` are documented public
exports from this module. They are thread-safe:
`ensure_msgspec_available` delegates to the `Once`-guarded bootstrap while
attached to the Python interpreter through the shared import-state lock, and
`with_python` acquires the same lock before calling `Python::attach`.
`run_with_kwargs` and `RunWithKwargsArgs` are hidden-public bootstrap helpers:
the `msgspec` installer path uses them to call `subprocess.run`, and UI
compile tests use them to lock down the crate-owned argument diagnostic.

The test-only coverage for this surface lives in
`tei-py/src/test_support/tests.rs` and the colocated
`tei-py/src/test_support/tests/` submodules. The parent test module owns the
deterministic checks for subprocess invocation shapes, `uv`/`pip` fallback
behaviour, availability reporting, and the import-state locking contract.
`subprocess_mocks.rs` owns the `subprocess.run` monkeypatch and restoration
guards, while `shutil_mocks.rs` owns the `shutil.which` restore guard used by
`has_uv` tests.

Tests that monkeypatch Python process-global state must hold the relevant RAII
guard for the full patch lifecycle. Lock acquisition must tolerate poisoning
with `unwrap_or_else(std::sync::PoisonError::into_inner)` or equivalent so one
failed test does not cascade into unrelated failures. Restoration guards should
surface cleanup failures when a test is otherwise succeeding, but must check
`std::thread::panicking()` and log to stderr instead of panicking again during
unwind.

### Thread safety for tests that mutate Python import state

Any test that modifies `sys.modules`, installs or removes entries from
`sys.meta_path`, or otherwise mutates the shared Python interpreter's import
state must acquire the process-wide `python_import_state_lock()` guard before
entering the `Python::attach` block:

```rust
let _import_state_lock = python_import_state_lock();
Python::attach(|py| {
    // mutate sys.modules here
});
```

The guard is an RAII `MutexGuard<'static, ()>` backed by a process-wide
`static Mutex`. Holding it prevents a concurrent test thread from observing a
partially-modified module registry. Release happens automatically when the
guard drops at the end of the enclosing scope.

`python_import_state_lock()` is intentionally `pub(super)` inside
`test_support`: it is visible to sibling modules such as `test_support::tests`,
but it is not part of the public `test_support` API and is not re-exported for
BDD integration tests. BDD tests should use `with_python`, which is the public
wrapper that acquires the same lock.

Prefer `with_python(|py| { ... })` over the raw `python_import_state_lock()` +
`Python::attach(...)` pair. `with_python` acquires the lock and attaches in one
call, making it impossible to forget the guard. Only reach for
`python_import_state_lock()` directly when the guard must outlive a single
`Python::attach` block.

If a test panics while holding the lock, the `Mutex` is poisoned. The
implementation recovers from a poisoned state by calling
`unwrap_or_else(|e| e.into_inner())` so subsequent tests are not blocked.

### Restoring `sys.modules` entries with RAII guards

Any test that inserts, replaces, or removes a `sys.modules` entry must restore
the registry to its exact pre-test state on scope exit, including panic unwind.
Manual `del_item` or `set_item` calls in test teardown are forbidden; they are
silently skipped when the test panics and leak state into every subsequent
in-process test.

Use an RAII guard that snapshots the entry in its constructor and restores it in
`Drop`. The canonical pattern in `tei-py` is `RestoreStructs` in
`tei-py/src/tests/bindings_tests.rs`:

```rust
struct RestoreStructs<'py> {
    sys_modules: Bound<'py, pyo3::types::PyAny>,
    previous: Option<Bound<'py, pyo3::types::PyAny>>,
}

impl<'py> RestoreStructs<'py> {
    /// Snapshots `sys.modules["tei_rapporteur.structs"]` and removes the
    /// entry so the test starts from a clean state.  `Drop` restores the saved
    /// entry when one existed, or deletes the key when it was absent.
    fn new(sys_modules: &Bound<'py, pyo3::types::PyAny>) -> Self {
        let previous = sys_modules.get_item("tei_rapporteur.structs").ok();
        sys_modules.del_item("tei_rapporteur.structs").ok();
        Self {
            sys_modules: sys_modules.clone(),
            previous,
        }
    }
}

impl Drop for RestoreStructs<'_> {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            self.sys_modules
                .set_item("tei_rapporteur.structs", previous)
                .ok();
        } else {
            self.sys_modules.del_item("tei_rapporteur.structs").ok();
        }
    }
}
```

The `Drop` implementation has two branches:

- **`Some(previous)`** — the entry existed before the test; restore it.
- **`None`** — the entry was absent before the test; delete any entry the test
  body may have inserted, returning the registry to its original state.

Call sites construct the guard via `RestoreStructs::new` and bind it to a
`let _restore` binding. The guard drops at the end of the enclosing scope, so
no explicit teardown call is needed:

```rust
with_python(|py| {
    let sys_modules = py.import("sys")?.getattr("modules")?;
    let _restore = RestoreStructs::new(&sys_modules);
    // test body — mutate sys.modules freely here
    Ok(())
});
```

When introducing a similar guard for a different `sys.modules` key, apply the
same two-branch `Drop` logic. Name the guard after the key it protects (e.g.
`RestoreMsgspec` for `msgspec`) and follow the same `new`-constructor pattern.

As per the coding guidelines: *global mutable state, lazy singletons,
process-wide registries, and static caches require explicit justification and
reset behaviour for tests.* An RAII guard satisfies the reset requirement
structurally, making it impossible to forget.

### Rust/Python test boundary patterns

The `msgspec` bootstrap path anchors shared state to
`static MSGSPEC_INIT: Once`. Use `OnceExt::call_once_py_attached` for this
implicit serialization rather than adding `#[serial]` to every test that
touches Python or wrapping the bootstrap in an external `Mutex`. The `Once`
guard ensures exactly one thread runs the installer, and `OnceExt` releases the
Python GIL while blocked threads wait. The
`ensure_msgspec_installed_is_safe_under_concurrent_access` test validates the
contract directly: it starts eight threads and asserts an upper bound on
`subprocess.run` calls so the bootstrap can tolerate short-circuiting,
best-effort retries, and concurrent execution without becoming order brittle.
Reserve `#[serial]` for cases where separate test functions must exclude
multiple distinct statics or process-global side effects; the single `Once`
owns the whole `msgspec` bootstrap critical section.

Tests that monkeypatch Python standard-library functions must bind restoration
to RAII guards. `SubprocessRestoreGuard` restores `subprocess.run`, and
`ShutilRestoreGuard` restores `shutil.which`; both carry the `Python<'py>` GIL
token and globals dictionary, then delegate their `Drop` implementation to the
existing restore function. This guarantees cleanup during panic unwinding, which
a final manual `restore_*` call at the end of a test body cannot provide. Name
future guards after what they undo, compose multiple guards in one scope when a
test patches multiple globals, and rely on reverse declaration order to mirror a
conventional `try`/`finally` stack.
