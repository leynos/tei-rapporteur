"""Contract test for the CI documentation-test step.

Coverage is measured with ``cargo llvm-cov nextest``, and nextest does not
execute doctests. Without a dedicated step the examples in the public API
documentation compile nowhere and rot unnoticed: a broken example in
``tei-test-helpers`` survived on ``main`` precisely because nothing ran it.

These tests assert the *command*, not a step name. A contract that matched a
step's ``name:`` would still pass with the invocation deleted, so the assertion
reads the ``run:`` body and requires the exact target, and requires it to run
before the coverage step that would otherwise be blamed for the gap.

Run via ``make test-workflow-contracts``.
"""

from __future__ import annotations

import re
from collections.abc import Callable
from pathlib import Path

import yaml

WORKFLOW_PATH = Path(__file__).resolve().parents[2] / ".github" / "workflows" / "ci.yml"

#: Matches an invocation of the documentation-test target as its own command,
#: so a longer target name such as ``make test-docs-site`` does not satisfy it.
DOC_TEST_RE = re.compile(r"(?m)^\s*make\s+test-doc\s*$")

#: Identifies the coverage step by the shared action it calls.
COVERAGE_ACTION = "leynos/shared-actions/.github/actions/generate-coverage@"


def _build_test_steps() -> list[dict[str, object]]:
    """Return the steps of the single CI job."""
    workflow = yaml.safe_load(WORKFLOW_PATH.read_text(encoding="utf-8"))
    jobs = workflow.get("jobs")
    assert isinstance(jobs, dict), "the workflow must declare a jobs mapping"
    job = jobs.get("build-test")
    assert isinstance(job, dict), "the workflow must declare a build-test job"
    steps = job.get("steps")
    assert isinstance(steps, list), "build-test must declare a list of steps"
    return steps


def _index_of(predicate: Callable[[dict[str, object]], bool]) -> int:
    """Return the index of the first step satisfying ``predicate``."""
    for index, step in enumerate(_build_test_steps()):
        if predicate(step):
            return index
    return -1


def _runs_doc_tests(step: dict[str, object]) -> bool:
    """Report whether a step invokes the documentation-test target."""
    run = step.get("run")
    return isinstance(run, str) and DOC_TEST_RE.search(run) is not None


def _is_coverage_step(step: dict[str, object]) -> bool:
    """Report whether a step calls the shared coverage action."""
    uses = step.get("uses")
    return isinstance(uses, str) and uses.startswith(COVERAGE_ACTION)


def test_ci_runs_the_documentation_tests() -> None:
    """CI must invoke the documentation-test target, not merely name a step."""
    assert _index_of(_runs_doc_tests) >= 0, (
        "build-test must contain a step whose run: body invokes 'make test-doc'"
    )


def test_documentation_tests_run_before_coverage() -> None:
    """The doctest step must precede the coverage step that cannot run them."""
    doc_index = _index_of(_runs_doc_tests)
    coverage_index = _index_of(_is_coverage_step)
    assert doc_index >= 0, "build-test must run the documentation tests"
    assert coverage_index >= 0, "build-test must call the shared coverage action"
    assert doc_index < coverage_index, (
        "the documentation tests must run before coverage, which cannot run them"
    )
