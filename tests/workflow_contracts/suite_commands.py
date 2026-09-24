"""Recognize shell commands that run the Rust or Python test suite.

The suite-once contract needs to know whether a workflow line runs the suite
outside the coverage action. A substring check is both too wide (``echo
pytest``) and too narrow (``make "test"``, ``make lint&&make test``), so a
command is split into segments at the shell separators that sit outside quotes
and escapes, each segment is split into words as the shell would, and the
program each segment runs is read before its arguments, through any ``env``,
``uv run``, ``uvx`` or ``python -m`` wrapper.
"""

from __future__ import annotations

import re
import shlex

#: Make options that take their value as the next word.
MAKE_VALUE_OPTIONS = frozenset(
    {"-C", "-f", "-I", "-o", "-W", "--directory", "--file", "--makefile"}
)
#: Make targets that run the suite: ``test``, ``all`` (which runs it),
#: ``coverage`` and the fast local variants. A bare ``make`` runs the default
#: goal, ``all``, so it counts too.
SUITE_TARGETS = frozenset({"test", "all", "coverage", "dev-test", "test-fast"})
#: ``uv run`` options that take their value as the next word.
UV_VALUE_OPTIONS = frozenset(
    {"--with", "--python", "-p", "--group", "--extra", "--from", "--project"}
)
#: ``env`` options that take their value as the next word.
ENV_VALUE_OPTIONS = frozenset({"-u", "--unset", "-C", "--chdir"})
#: The shell's view of a command, one piece at a time: a comment (dropped), a
#: line continuation (read as a space), a separator between commands, or text,
#: where quoted strings and escaped characters are kept whole so a separator
#: inside them does not split the command.
TOKENS = re.compile(
    r"""
    (?P<comment>(?:^|(?<=[\s;&|]))\#[^\n]*)
    |(?P<continuation>\\\n)
    |(?P<separator>[;&|\n])
    |(?P<text>'[^']*'|"(?:\\.|[^"\\])*"|\\.|[^'"\\;&|\n\#]+|\#)
    """,
    re.VERBOSE | re.DOTALL | re.MULTILINE,
)
#: Python interpreters by name: ``python``, ``python3`` and ``python3.13``.
PYTHON_PROGRAM = re.compile(r"python(?:3(?:\.\d+)?)?")
#: Programs that are pytest itself.
PYTEST_PROGRAMS = frozenset({"pytest", "py.test"})
#: Cargo options that take their value as the next word.
CARGO_VALUE_OPTIONS = frozenset(
    {"--config", "-Z", "-C", "--manifest-path", "--color", "--target-dir"}
)
#: Cargo subcommands that run the suite.
SUITE_SUBCOMMANDS = frozenset({"test", "nextest", "llvm-cov"})


def _segments(command: str) -> list[str]:
    """Split a command at the separators outside quotes, escapes and comments."""
    segments = [""]
    for token in TOKENS.finditer(command):
        if token.lastgroup == "separator":
            segments.append("")
        elif token.lastgroup == "text":
            segments[-1] += token.group()
        elif token.lastgroup == "continuation":
            segments[-1] += " "
    return segments


def _is_assignment(word: str) -> bool:
    """Report whether a word is a leading ``NAME=value`` assignment."""
    return "=" in word and not word.startswith("-")


def _without_assignments(words: list[str]) -> list[str]:
    """Return the words from the first one that is not an assignment."""
    while words and _is_assignment(words[0]):
        words = words[1:]
    return words


def _words(segment: str) -> list[str]:
    """Split one segment into words as the shell would, less assignments."""
    try:
        words = shlex.split(segment, comments=True)
    except ValueError:
        words = segment.split()
    return _without_assignments(words)


def _operands(words: list[str], value_options: frozenset[str]) -> list[str]:
    """Return a command's operands: its words less options and their values."""
    found: list[str] = []
    skip = False
    for word in words:
        if skip:
            skip = False
        elif word in value_options:
            skip = True
        elif not word.startswith(("-", "+")) and "=" not in word:
            found.append(word)
    return found


def _unwrap(words: list[str]) -> list[str]:
    """Strip an ``env``, ``uv run``, ``uvx`` or ``python -m`` wrapper."""
    program = words[0].rsplit("/", 1)[-1] if words else ""
    if program == "env":
        after = _after_options(words[1:], ENV_VALUE_OPTIONS)
        return _unwrap(_without_assignments(after))
    if program == "uv" and words[1:2] == ["run"]:
        return _unwrap(_after_options(words[2:], UV_VALUE_OPTIONS))
    if program == "uvx":
        return _unwrap(_after_options(words[1:], UV_VALUE_OPTIONS))
    if PYTHON_PROGRAM.fullmatch(program) and words[1:2] == ["-m"]:
        return words[2:]
    return words


def _after_options(words: list[str], value_options: frozenset[str]) -> list[str]:
    """Return the words from the first operand on."""
    index = 0
    while index < len(words) and words[index].startswith("-"):
        index += 2 if words[index] in value_options else 1
    return words[index:]


def _cargo_runs_suite(arguments: list[str]) -> bool:
    """Report whether cargo's arguments name a suite-running subcommand."""
    operands = _operands(arguments, CARGO_VALUE_OPTIONS)
    return bool(operands) and operands[0] in SUITE_SUBCOMMANDS


def _make_runs_suite(arguments: list[str]) -> bool:
    """Report whether make's arguments run the default goal or a suite target."""
    targets = _operands(arguments, MAKE_VALUE_OPTIONS)
    return not targets or bool(SUITE_TARGETS & set(targets))


def _segment_runs_suite(segment: str) -> bool:
    """Report whether one shell segment runs the suite."""
    words = _unwrap(_words(segment))
    program = words[0].rsplit("/", 1)[-1] if words else ""
    readers = {"cargo": _cargo_runs_suite, "make": _make_runs_suite}
    if program in PYTEST_PROGRAMS:
        return True
    reader = readers.get(program)
    return reader is not None and reader(words[1:])


def runs_suite(command: str) -> bool:
    """Report whether a shell command runs the suite, in any spelling.

    Parameters
    ----------
    command : str
        A workflow step's ``run`` text, which may span several lines.

    Returns
    -------
    bool
        True when any segment runs pytest, ``cargo test``, ``cargo nextest``
        or ``cargo llvm-cov``, or runs ``make`` with no target or a suite
        target.

    Examples
    --------
    >>> runs_suite("make lint&&make test")
    True
    >>> runs_suite("echo pytest")
    False
    """
    return any(_segment_runs_suite(part) for part in _segments(command))
