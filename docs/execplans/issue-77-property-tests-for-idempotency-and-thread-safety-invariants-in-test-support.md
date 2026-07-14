# Issue 77 bootstrap property tests review

This ExecPlan guides the review-feedback pass for GitHub issue #77, which adds
property-based coverage around the `tei-py` `msgspec` bootstrap helper. The
observable outcome is that the bootstrap tests still prove idempotency and
thread-safety invariants, while their Python monkeypatching is isolated,
panic-safe, and checked by the repository gates.

The main implementation files are `tei-py/src/test_support.rs` and the sibling
test modules under `tei-py/src/test_support_tests/`. The production helper
`ensure_msgspec_installed` uses `pyo3::sync::OnceExt::call_once_py_attached` to
run bootstrap logic at most once per process. The tests replace Python
`subprocess.run` and import behaviour so they can exercise that bootstrap path
without making network calls.

## Constraints

All changes must follow the repository `AGENTS.md` instructions for issue #77.
Use `make` targets for validation. Test, lint, typecheck, and formatting
commands must run sequentially and log through `tee` into `/tmp`. Do not
silence warnings or lints. Keep file sizes under 400 lines.

## Tolerances

Stop and ask for direction if a fix requires changing public production APIs,
adding new non-dev dependencies, deleting the property tests, or exceeding a
small review-feedback patch. Stop if repository gates fail for issues that
cannot be fixed without broad unrelated refactoring.

## Risks

The tests mutate Python interpreter global state. The mitigation is to hold a
static Rust mutex for the full monkeypatch lifecycle and restore Python state
in RAII guards. The bootstrap path also uses a process-wide `Once`, so each
property test case must continue to respect that the installer path can only be
forced once per test process.

## Progress

- [x] Scope is limited to the issue #77 `tei-py` bootstrap helper and its
  sibling test-support modules.
- [x] `ensure_msgspec_installed` delegates repeated subprocess setup to
  extracted helpers so the public helper remains small.
- [x] Bootstrap tests record mocked subprocess calls, serialize Python global
  monkeypatches, and restore state with RAII guards.
- [x] Property coverage checks idempotency and thread-safety invariants without
  relying on real package installation.

## Surprises & Discoveries

The review finding about `_calls` is valid in `bootstrap_mocks.rs`: helper
functions read `_calls`, while the mock only increments the Rust counter.

Workspace Clippy caught two local guard-shape issues during `make lint`: the
lock guard field exists for drop timing rather than direct reads, and an
explicitly dropped binding must not use an underscore-prefixed name. Both were
fixed without suppressing lints.

## Decision Log

Use RAII guards rather than success-path cleanup for Python monkeypatches. This
is required because `prop_assert!` exits the property body early when a case
fails and proptest begins shrinking.

Use one static mutex around the full `subprocess.run` monkeypatch lifecycle.
Locking only during setup and restore would still allow overlapping tests to
observe each other's patched state.

Defer Kani harnesses for this bootstrap path. The behaviour under review crosses
`pyo3::sync::OnceExt::call_once_py_attached` and the embedded Python
interpreter, so a useful proof would need a broader abstraction around PyO3 or
upstream instrumentation rather than a small local harness around the test
helper.

## Implementation Plan

First, make the mechanical production refactor requested by review:
`make_subprocess_kwargs` builds the repeated Python keyword dictionary, and
`do_bootstrap` contains the installer sequence. `ensure_msgspec_installed`
delegates its `Once` closure to `do_bootstrap`.

Second, fix test isolation. The bootstrap mock initializes `_calls`, records
each mocked `subprocess.run` invocation, and exposes an explicit
`restore_msgspec_blocker` helper. The subprocess restore path returns a
`PyResult` and the RAII guard calls `.expect()` so restoration failures are not
hidden. Property tests create a restore guard before any property assertion can
exit the test body.

Third, validate. Run `cargo clippy -p tei-py`, then the required gates:
`make check-fmt`, `make lint`, `make typecheck`, and `make test`, each with
output logged to `/tmp`.

## Outcomes & Retrospective

The review-feedback pass leaves `ensure_msgspec_installed` delegating the
`Once` bootstrap closure to `do_bootstrap`, reducing the inline complexity of
the public helper. The bootstrap tests record Python subprocess calls, force
the test bootstrap path without mutating Python import hooks, keep subprocess
monkeypatches serialized for their full lifecycle, and use RAII cleanup in the
property tests.

Durable validation expectations for this work are the focused
`cargo clippy -p tei-py` check plus the repository gates: `make check-fmt`,
`make lint`, `make typecheck`, `make test`, and `make markdownlint` when
documentation changes.
