"""Contract test for the multi-ecosystem Dependabot group that pairs maturin and PyO3.

maturin (uv) and PyO3 (Cargo) are versioned independently yet build one
extension, so an update to either alone cannot be built against the other. A
multi-ecosystem group puts the available updates for the two into one pull
request, so when both have one a single CI run builds both. It does not hold
back a bump when only one of the two has an update, and a group cannot choose
compatible versions: it coordinates the update without proving it.

A mistake here is silent, because Dependabot reports nothing for a stanza that
names a group the file never defines. These tests read the checked-in
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


def _stanzas() -> dict[str, dict[str, object]]:
    """Return each update stanza keyed by its package ecosystem."""
    updates = _document().get("updates")
    assert isinstance(updates, list), "the configuration must declare `updates`"
    return {str(stanza["package-ecosystem"]): stanza for stanza in updates}


def test_the_group_carries_the_pull_request_bound_not_its_entries() -> None:
    """Bound the group; Dependabot rejects the bound on an entry that joins it."""
    groups = _document()["multi-ecosystem-groups"]
    limit = groups[GROUP].get("open-pull-requests-limit")
    assert isinstance(limit, int), f"the {GROUP} group needs an integer bound; got {limit!r}"
    assert limit > 0, f"the {GROUP} group needs a positive bound; got {limit!r}"
    carrying = [
        ecosystem
        for ecosystem, stanza in _stanzas().items()
        if "multi-ecosystem-group" in stanza and "open-pull-requests-limit" in stanza
    ]
    assert not carrying, f"Dependabot rejects the bound on {carrying}; set it on the group"


def test_the_group_is_defined_with_a_daily_schedule() -> None:
    """Define the group the two stanzas join, on the estate's daily cadence."""
    groups = _document().get("multi-ecosystem-groups")
    assert isinstance(groups, dict), "`multi-ecosystem-groups` must be defined"
    assert GROUP in groups, f"{GROUP!r} must be defined; got {sorted(groups)}"
    schedule = groups[GROUP].get("schedule", {})
    assert schedule.get("interval") == "daily", (
        f"the {GROUP} group must run daily; got {schedule.get('interval')!r}"
    )


@pytest.mark.parametrize(("ecosystem", "members"), sorted(MEMBERS.items()))
def test_each_half_of_the_extension_build_joins_the_group(
    ecosystem: str, members: list[str]
) -> None:
    """Name the group and the members in the stanza, or the pair arrives split."""
    stanza = _stanzas()[ecosystem]
    assert stanza.get("multi-ecosystem-group") == GROUP, (
        f"the {ecosystem} stanza must join {GROUP!r}; "
        f"got {stanza.get('multi-ecosystem-group')!r}"
    )
    assert stanza.get("patterns") == members, (
        f"the {ecosystem} stanza must offer exactly {members} to the group; "
        f"got {stanza.get('patterns')!r}"
    )


def test_no_other_stanza_joins_a_multi_ecosystem_group() -> None:
    """Keep the group to the two halves of the extension build."""
    joined = {
        ecosystem
        for ecosystem, stanza in _stanzas().items()
        if "multi-ecosystem-group" in stanza
    }
    assert joined == set(MEMBERS), (
        f"only {sorted(MEMBERS)} may join {GROUP!r}; got {sorted(joined)}"
    )
