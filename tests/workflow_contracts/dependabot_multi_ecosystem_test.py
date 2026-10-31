"""Contract test for the multi-ecosystem Dependabot group that pairs maturin and PyO3.

maturin (uv) and PyO3 (Cargo) are versioned independently yet build one
extension, so an update to either alone cannot be built against the other. A
multi-ecosystem group puts the available updates for the two into one pull
request, so when both have one a single CI run builds both. It does not hold
back a bump when only one of the two has an update, and a group cannot choose
compatible versions: it coordinates the update without proving it.

Dependabot builds one job per entry, and an entry that joins a group runs only
that group, over the dependencies its ``patterns`` take (cuprum's first run of
the arrangement showed it: the entry's own groups, ignore rules and every other
dependency were not processed). So each of the two ecosystems has two entries,
the group entry and a plain entry that ignores the pair so that it is handled
once. A mistake here is silent, because Dependabot reports nothing for a stanza
that names a group the file never defines. These tests read the checked-in
configuration back and hold it to the arrangement.

Run via ``make test-workflow-contracts``.
"""

from __future__ import annotations

from pathlib import Path

import pytest
import yaml

CONFIG_PATH = Path(__file__).resolve().parents[2] / ".github" / "dependabot.yml"

#: The group the two stanzas join.
GROUP = "maturin-pyo3"
EXPECTED_LIMIT = 5

#: What each ecosystem offers to that group. The Cargo stanza also offers the
#: PyO3 build-time crate and the serde-pyobject bridge, which release against
#: one PyO3 generation.
MEMBERS = {
    "uv": ["maturin"],
    "cargo": ["pyo3", "pyo3-build-config", "serde-pyobject"],
}


def _document() -> dict[str, object]:
    """Return the parsed Dependabot configuration."""
    document = yaml.safe_load(CONFIG_PATH.read_text(encoding="utf-8"))
    assert isinstance(document, dict), f"{CONFIG_PATH} must parse to a mapping"
    return document


def _entries() -> list[dict[str, object]]:
    """Return every ``updates`` entry, in order."""
    updates = _document().get("updates")
    assert isinstance(updates, list), "the configuration must declare `updates`"
    return updates


def _group_entries() -> list[dict[str, object]]:
    """Return the entries that join a multi-ecosystem group."""
    return [entry for entry in _entries() if "multi-ecosystem-group" in entry]


def _plain_entry(ecosystem: str) -> dict[str, object]:
    """Return the one entry for *ecosystem* that does not join a group."""
    plain = [
        entry
        for entry in _entries()
        if entry["package-ecosystem"] == ecosystem
        and "multi-ecosystem-group" not in entry
    ]
    assert len(plain) == 1, f"{ecosystem} needs exactly one plain entry"
    return plain[0]


def test_the_group_is_defined_with_a_daily_schedule() -> None:
    """Define the group the two entries join, on the estate's daily cadence."""
    groups = _document().get("multi-ecosystem-groups")
    assert isinstance(groups, dict), "`multi-ecosystem-groups` must be defined"
    assert GROUP in groups, f"{GROUP!r} must be defined; got {sorted(groups)}"
    schedule = groups[GROUP].get("schedule", {})
    assert schedule.get("interval") == "daily", (
        f"the {GROUP} group must run daily; got {schedule.get('interval')!r}"
    )


def test_the_group_carries_the_pull_request_bound_not_its_entries() -> None:
    """Bound the group; Dependabot rejects the bound on an entry that joins it."""
    groups = _document()["multi-ecosystem-groups"]
    limit = groups[GROUP].get("open-pull-requests-limit")
    assert type(limit) is int, (
        f"the {GROUP} group needs an integer bound; got {limit!r}"
    )
    assert limit == EXPECTED_LIMIT, (
        f"the {GROUP} group is bounded at {EXPECTED_LIMIT}; got {limit!r}"
    )
    carrying = [
        entry["package-ecosystem"]
        for entry in _group_entries()
        if "open-pull-requests-limit" in entry
    ]
    assert not carrying, (
        f"Dependabot rejects the bound on {carrying}; set it on the group"
    )


@pytest.mark.parametrize(("ecosystem", "members"), sorted(MEMBERS.items()))
def test_each_half_of_the_extension_build_has_one_group_entry(
    ecosystem: str, members: list[str]
) -> None:
    """Name the group and the members in one entry per ecosystem, with no groups."""
    entries = [
        entry for entry in _group_entries() if entry["package-ecosystem"] == ecosystem
    ]
    assert len(entries) == 1, (
        f"{ecosystem} needs exactly one entry joining {GROUP!r}; got {len(entries)}"
    )
    (entry,) = entries
    assert entry["multi-ecosystem-group"] == GROUP, (
        f"the {ecosystem} group entry must join {GROUP!r}"
    )
    assert entry.get("patterns") == members, (
        f"the {ecosystem} group entry must offer exactly {members}; "
        f"got {entry.get('patterns')!r}"
    )
    assert "groups" not in entry, (
        f"the {ecosystem} group entry runs only the group, so it carries no `groups`"
    )


@pytest.mark.parametrize(("ecosystem", "members"), sorted(MEMBERS.items()))
def test_the_plain_entry_ignores_what_the_group_entry_handles(
    ecosystem: str, members: list[str]
) -> None:
    """Keep the pair out of the plain entry, which handles everything else."""
    ignored = [rule["dependency-name"] for rule in _plain_entry(ecosystem)["ignore"]]
    assert ignored == members, (
        f"the plain {ecosystem} entry must ignore exactly {members}; got {ignored}"
    )


def test_only_the_extension_build_ecosystems_join_a_group() -> None:
    """Keep the group to the two halves of the extension build."""
    joined = {entry["package-ecosystem"] for entry in _group_entries()}
    assert joined == set(MEMBERS), (
        f"only {sorted(MEMBERS)} may join {GROUP!r}; got {sorted(joined)}"
    )
