"""Contract tests for the documentation-test wiring in the Makefile.

`ci_doc_tests_test.py` asserts that CI invokes `make test-doc`. That is only
half a contract: it says the target is called, not what the target does. With
only that half, deleting `--all-features` from the recipe, or dropping the
`test-doc` prerequisite from `test`, passes every gate in the repository while
silently restoring the gap this branch closed. Feature-gated examples would
compile nowhere again, and nothing would say so.

These tests read the recipe text and assert the command, for the reason the
estate learned the hard way: an assertion that matches an identifier is
satisfied by a comment above the deleted invocation. Each assertion here names
the flag or the dependency it protects, and each was mutation-tested by
removing exactly that token from the Makefile.

Run via ``make test-workflow-contracts``.
"""

from __future__ import annotations

import re
from pathlib import Path

MAKEFILE_PATH = Path(__file__).resolve().parents[2] / "Makefile"

#: Matches a target line, capturing the target name and its prerequisites.
TARGET_RE = re.compile(r"^(?P<name>[A-Za-z0-9_.-]+):(?P<prereqs>[^=\n]*)$", re.MULTILINE)


def _read_makefile() -> str:
    """Return the Makefile source."""
    return MAKEFILE_PATH.read_text(encoding="utf-8")


def _prerequisites(target: str) -> list[str]:
    """Return the prerequisites declared for ``target``."""
    matches = [
        match
        for match in TARGET_RE.finditer(_read_makefile())
        if match.group("name") == target
    ]
    assert matches, f"the Makefile must declare a {target} target"
    return matches[0].group("prereqs").split("##")[0].split()


def _recipe_lines(following: list[str]) -> list[str]:
    """Collect the recipe lines that follow a target declaration.

    A recipe runs until the first non-blank line that does not begin with a
    tab, which is where the next declaration starts.
    """
    recipe: list[str] = []
    for candidate in following:
        if candidate.startswith("\t"):
            recipe.append(candidate.lstrip("\t"))
        elif candidate.strip():
            break
    return recipe


def _recipe(target: str) -> str:
    """Return the recipe of ``target``, tabs stripped."""
    lines = _read_makefile().splitlines()
    prefix = f"{target}:"
    starts = [index for index, line in enumerate(lines) if line.startswith(prefix)]
    assert starts, f"the Makefile must declare a {target} target"
    return "\n".join(_recipe_lines(lines[starts[0] + 1 :]))


def _doc_test_command() -> str:
    """Return the line of the test-doc recipe that runs the doctests."""
    recipe = _recipe("test-doc")
    command = next((line for line in recipe.splitlines() if "test --doc" in line), "")
    assert command, "test-doc must run cargo test --doc"
    return command


def test_test_depends_on_the_documentation_tests() -> None:
    """A local `make test` must cover the doctests nextest cannot run."""
    assert "test-doc" in _prerequisites("test"), (
        "the test target must declare test-doc as a prerequisite, so a local "
        "test run covers the examples nextest skips"
    )


def test_documentation_tests_activate_every_feature() -> None:
    """The doctest run must enable the features that gate public examples."""
    command = _doc_test_command()
    assert "--workspace" in command, (
        "test-doc must cover the whole workspace, not the root package alone"
    )
    assert "--all-features" in command, (
        "test-doc must pass --all-features: tei_py::test_support compiles only "
        "under cfg(test) or feature test-support, and rustdoc sets neither, so "
        "without it those public examples compile nowhere"
    )


def test_documentation_tests_deny_warnings() -> None:
    """A warning in an example is a defect, not a note."""
    command = _doc_test_command()
    assert 'RUSTFLAGS="-D warnings"' in command, (
        "test-doc must deny warnings, matching every other compiling gate"
    )
