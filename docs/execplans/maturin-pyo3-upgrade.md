# Update maturin and PyO3 compatibility validation

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: COMPLETE

## Purpose / big picture

This branch updates the Python extension toolchain used by Text Encoding
Initiative (TEI) Rapporteur. The Rust binding crate `tei-py` should build
against the latest verified PyO3 family, and the Python packaging configuration
should use a pinned maturin release so future upgrades are intentional and
testable. A reviewer can observe success by running the Rust quality gates and
by running the new Python maturin tests, which inspect wheel metadata and
packaging layout in the style used by Cuprum commit
`df25f6c09e388cba1a055d167a5a88d13a8826fd`.

The desired end state is that a future maturin or PyO3 upgrade changes a small,
obvious set of pins and either preserves the wheel contract or fails with a
focused test explaining what changed.

## Constraints

The current branch is `chore/maturin-pyo3-upgrade`; work must stay on this
branch. The repository guidance in `AGENTS.md` requires `make check-fmt`,
`make lint`, `make typecheck`, and `make test` to pass before committing code.
Test, lint and format commands must be run sequentially and logged through
`tee` to files under `/tmp`. Rust dependency versions in `Cargo.toml` must use
normal caret requirements. The implementation must not introduce an isolated
Cargo cache and must not kill other agents' processes.

The requested source approach is Cuprum commit
`df25f6c09e388cba1a055d167a5a88d13a8826fd`. This repository does not have
Cuprum's separate `.github/actions/build-wheels/action.yml` or
`.github/workflows/build-wheels.yml`, so imported compatibility checks must be
adapted to the local `pyproject.toml` and `.github/workflows/ci.yml` files.

## Tolerances

If a PyO3 API migration requires broad rewrites outside `tei-py` or changes the
public Python API, stop and record the deviation before proceeding. If the new
wheel build test adds more than one new Python helper module and one test
module, keep it small or escalate. If any required `make` gate fails for an
unrelated pre-existing issue, fix it when it is reasonably scoped; stop only if
the fix would require an unrelated architectural change.

## Risks

PyO3 minor releases can change binding signatures and `Bound<'py, T>` APIs,
which may require small updates to the extension crate. The maturin wheel
snapshot may vary by platform, Python tag or generated metadata, so the test
must normalize platform-specific entries. Invoking a full maturin release build
inside a routinely executed test can be expensive; the imported test should be
skippable when maturin or the Rust toolchain is unavailable and should be
separate from the mandatory Rust gates unless the project later adds a Python
test make target.

## Progress

- [x] 2026-06-05: Loaded the requested `leta` and `rust-router` skills and
  created a Leta workspace for this worktree.
- [x] 2026-06-05: Confirmed the branch is `chore/maturin-pyo3-upgrade`, not a
  main branch.
- [x] 2026-06-05: Inspected Cuprum commit
  `df25f6c09e388cba1a055d167a5a88d13a8826fd` and identified the relevant
  maturin helper, wheel build test and snapshot pattern.
- [x] 2026-06-05: Verified current latest versions: maturin `1.13.3` on PyPI
  and PyO3 `0.28.3` on docs.rs/crates.io.
- [x] 2026-06-05: Updated Rust and Python packaging pins to maturin `1.13.3`,
  PyO3 `0.28.3`, and `serde-pyobject` `0.8.0`.
- [x] 2026-06-05: Added local maturin compatibility and wheel build tests
  under `python/tests`.
- [x] 2026-06-05: Fixed PyO3 0.28 API migration issues around
  `Python::attach`, `Python::detach`, `Py<PyAny>`, `Bound::cast`, and
  `PyModule::from_code`.
- [x] 2026-06-05: Ran the focused Python maturin compatibility test and the
  required gates: `make check-fmt`, `make lint`, `make typecheck`, and
  `make test`.
- [x] 2026-06-05: Committed the validated implementation as `4c9246e`
  (`Update maturin and PyO3 validation`).
- [x] 2026-06-05: Created draft pull request
  <https://github.com/leynos/tei-rapporteur/pull/87>.

## Surprises & Discoveries

The local continuous integration (CI) environment currently installs
`maturin==1.10.1` directly, while `pyproject.toml` permits any maturin from
`1.6` up to but excluding `2.0`. That means the project does not currently have
a single source of truth for the maturin version.

Cuprum pins maturin as a development dependency and synchronizes that exact pin
with wheel-building CI. TEI Rapporteur currently has no Python test dependency
group apart from the `dev` extra, so the imported pin check should avoid
assuming Cuprum's full Python tooling stack.

PyO3 0.28 removes `Python::with_gil` and `Python::allow_threads`, replacing
them with `Python::attach` and `Python::detach`. It also removes the root
`PyObject` alias and deprecates `Bound::downcast` in favour of `Bound::cast`.

`maturin build --manifest-path tei-py/Cargo.toml` did not apply the
`pyproject.toml` environment override in a way that prevented linking against
`libpython`. The compatibility helper therefore passes
`TEI_PY_BUILD_EXTENSION=1` explicitly when building the validation wheel.

Detecting `CARGO_FEATURE_EXTENSION_MODULE` in `tei-py/build.rs` makes
`cargo test --all-features` try to link Rust test binaries as extension-module
builds. The build script must therefore reserve extension configuration for
explicit wheel-build signals and keep the default test path using PyO3's
`auto-initialize` feature.

## Decision Log

The maturin pin will be set to `1.13.3` because PyPI lists it as the latest
available release on 2026-06-05, and the pinned Cuprum approach also uses this
version. The PyO3 family will be updated to `0.28.3` because docs.rs lists it
as the latest PyO3 release and it aligns `pyo3`, `pyo3-build-config`,
`pyo3-ffi` and `pyo3-macros` transitively.

The imported compatibility tests will live under `python/tests` rather than a
new top-level `tests` package because this repository already keeps its Python
smoke and type-stub tests there. The helper will be local to that package so it
does not affect the installed `tei_rapporteur` package.

The Python development dependency group now includes `pytest>=8.0,<9.0` because
the repository already has Python pytest-style tests and the new compatibility
tests should be runnable from a fresh `uv` environment.

The `tei-py` crate now has explicit `auto-initialize` and `extension-module`
features. `auto-initialize` remains the default so Rust tests and examples can
embed Python normally. Maturin builds use `no-default-features = true` plus the
`extension-module` feature so wheels do not link against `libpython`.

## Implementation plan

First, update `pyproject.toml`, `.github/workflows/ci.yml`, `tei-py/Cargo.toml`,
`Cargo.lock`, and `uv.lock` so the maturin and PyO3 pins are current and
internally consistent. Then add a Python helper modelled on Cuprum's
`tests/helpers/maturin.py`, adapted for the TEI Rapporteur package name,
extension module path, metadata and CI pin locations. Add Python tests that
verify the maturin pins stay synchronized, the installed maturin version
matches the pin when present, and a locally built wheel has expected normalised
metadata and entries.

After implementation, run focused compatibility checks first, then run the
required sequential gates:

```sh
make check-fmt 2>&1 | tee /tmp/check-fmt-tei-rapporteur-chore-maturin-pyo3-upgrade.out
make lint 2>&1 | tee /tmp/lint-tei-rapporteur-chore-maturin-pyo3-upgrade.out
make typecheck 2>&1 | tee /tmp/typecheck-tei-rapporteur-chore-maturin-pyo3-upgrade.out
make test 2>&1 | tee /tmp/test-tei-rapporteur-chore-maturin-pyo3-upgrade.out
```

If formatting changes are required, run `make fmt` and repeat `make check-fmt`.
When gates pass, commit the logical change. Finally, inspect the full branch
diff, push the branch, and create a draft pull request using the `pr-creation`
skill.

## Outcomes & Retrospective

The implementation updates the Python extension toolchain to maturin `1.13.3`,
PyO3 `0.28.3`, `pyo3-build-config` `0.28.3`, and `serde-pyobject` `0.8.0`. It
adds compatibility tests that verify the maturin pin is synchronized across
`pyproject.toml` and CI, checks the installed maturin version when present, and
builds a normalized wheel snapshot to catch future metadata or layout drift.

The required validation passed on 2026-06-05:

```sh
uv run --group dev pytest python/tests/test_maturin_build.py -q
make check-fmt
make lint
make typecheck
make test
```

`make fmt` was required before the final format check because the new execplan
and existing Markdown files needed repository-standard wrapping.

The implementation commit is `4c9246e` and the draft pull request is
<https://github.com/leynos/tei-rapporteur/pull/87>.
