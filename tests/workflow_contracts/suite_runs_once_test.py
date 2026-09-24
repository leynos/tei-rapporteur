"""Contract that each pull request runs the test suite once.

The coverage step in ``ci.yml``'s ``build-test`` job runs the Rust suite
through ``cargo llvm-cov nextest`` and, because the repository is a mixed
Rust-and-Python project, the Python suite through pytest over the
``testpaths`` in ``pyproject.toml``. That collects
``python/tests/test_maturin_build.py``, in an environment synced with the
``dev`` group, so the pinned maturin is present and its tests run rather than
skip. ``build-test`` used to run that file again in a step of its own; it
repeated what coverage had already run, and was removed.

These tests hold the split:

- no step in any workflow runs the suite, in any spelling
  ``suite_commands`` recognizes: pytest however wrapped, ``cargo test``,
  ``cargo nextest``, ``cargo llvm-cov``, or ``make`` with no target or a
  suite target;
- ``ci.yml`` runs on every pull request, and ``build-test`` runs the coverage
  action in one unguarded step;
- ``pyproject.toml`` points pytest at ``python/tests`` and keeps maturin in
  the ``dev`` group, which is what makes coverage run the maturin tests.

``make test-doc`` and ``make test-workflow-contracts`` are not suite runs:
coverage cannot run doctests, and the contracts run without the project
installed.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest
import yaml
from suite_commands import runs_suite

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
COVERAGE_ACTION = "leynos/shared-actions/.github/actions/generate-coverage@"
PYTHON_TESTS = "python/tests"
#: A requirement naming maturin itself, not a longer package name.
MATURIN_REQUIREMENT = re.compile(r"maturin(?![\w.-])")


def _workflow_steps() -> list[tuple[str, dict]]:
    """Return every workflow step, labelled by workflow and job."""
    found: list[tuple[str, dict]] = []
    for path in sorted([*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml")]):
        document = yaml.safe_load(path.read_text(encoding="utf-8"))
        for name, job in (document.get("jobs") or {}).items():
            label = f"{path.name}:{name}"
            found.extend((label, step) for step in job.get("steps") or [])
    return found


def _build_test() -> dict:
    """Return ``ci.yml``'s ``build-test`` job, refusing a conditional one."""
    document = yaml.safe_load((WORKFLOWS / "ci.yml").read_text(encoding="utf-8"))
    job = (document.get("jobs") or {}).get("build-test")
    assert job is not None, "ci.yml must define build-test"
    assert "if" not in job, "build-test must run on every pull request"
    return job


def _ci_triggers() -> dict:
    """Return ``ci.yml``'s triggers, read under ``on`` or its boolean form."""
    document = yaml.safe_load((WORKFLOWS / "ci.yml").read_text(encoding="utf-8"))
    value = document.get("on", document.get(True))
    if isinstance(value, dict):
        return value
    return dict.fromkeys(value if isinstance(value, list) else [value])


def _pyproject() -> dict:
    """Parse the repository's ``pyproject.toml``."""
    return tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))


@pytest.mark.parametrize(
    ("command", "expected"),
    [
        ("uv run --group dev pytest python/tests/test_maturin_build.py -q", True),
        ("pytest", True),
        ("python -m pytest python/tests", True),
        ("uvx --with pyyaml pytest tests", True),
        ("make test", True),
        ("make", True),
        ('make "test"', True),
        ("make -C . all", True),
        ("make coverage", True),
        ("make lint&&make test", True),
        ("cargo test --all-features", True),
        ("cargo +nightly nextest run", True),
        ("cargo llvm-cov nextest --lcov", True),
        ("RUSTFLAGS='-D warnings' cargo test", True),
        ("env RUN_ACT_VALIDATION=1 make test", True),
        ("python3.13 -m pytest", True),
        ("make \\\ntest", True),
        ("make lint # then\nmake test", True),
        ("echo 'pre;make test;post'", False),
        ('echo "a && pytest"', False),
        ("# make test", False),
        ("make test-doc", False),
        ("make test-workflow-contracts", False),
        ("maturin build --release --out dist", False),
        ("python -c 'import tei_rapporteur'", False),
        ("cargo +1.95.0 install merman-cli --locked", False),
        ("cargo run -- test", False),
        ("echo pytest", False),
    ],
)
def test_the_suite_pattern(command: str, *, expected: bool) -> None:
    """Recognize every spelling of a suite run, and nothing longer."""
    assert runs_suite(command) is expected, command


def test_no_step_runs_the_suite_outside_coverage() -> None:
    """Refuse any step that runs the suite beside the coverage action."""
    repeated = [
        (label, str(step.get("run")).strip())
        for label, step in _workflow_steps()
        if runs_suite(str(step.get("run", "")))
    ]
    assert not repeated, f"the suite runs outside coverage in {repeated!r}"


def test_build_test_runs_coverage_on_every_event() -> None:
    """Require one unconditional coverage step in ``build-test``."""
    steps = [
        step
        for step in _build_test().get("steps") or []
        if str(step.get("uses", "")).startswith(COVERAGE_ACTION)
    ]
    assert len(steps) == 1, "build-test must run the coverage action once"
    assert "if" not in steps[0], "the coverage step must run on every event"


def test_ci_runs_on_every_pull_request() -> None:
    """Require ``ci.yml``'s pull-request trigger, with no branch or path filter."""
    triggers = _ci_triggers()
    assert "pull_request" in triggers, "ci.yml must run on pull requests"
    trigger = triggers["pull_request"] or {}
    filters = sorted(set(trigger) - {"types"})
    assert not filters, f"filters {filters} would skip some pull requests"


def test_coverage_collects_the_python_tests() -> None:
    """Require pytest's ``testpaths`` to include the Python tests."""
    options = _pyproject().get("tool", {}).get("pytest", {}).get("ini_options", {})
    assert PYTHON_TESTS in (options.get("testpaths") or []), (
        f"coverage's pytest run must collect {PYTHON_TESTS}"
    )


def test_the_dev_group_carries_maturin() -> None:
    """Require maturin in the ``dev`` group, so coverage runs its tests."""
    group = _pyproject().get("dependency-groups", {}).get("dev") or []
    pinned = [spec for spec in group if MATURIN_REQUIREMENT.match(str(spec))]
    assert pinned, "without maturin the coverage run skips the maturin tests"
