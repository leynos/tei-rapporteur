"""Compatibility tests for the maturin-built Python extension wheel."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

from maturin_compat import (
    build_native_wheel_artifact,
    installed_maturin_version,
    read_expected_maturin_version,
    read_maturin_pins,
    repo_root,
    toolchain_available,
    wheel_build_snapshot,
)

EXPECTED_WHEEL_SNAPSHOT = {
    "entries": [
        "tei_rapporteur-<version>.dist-info/METADATA",
        "tei_rapporteur-<version>.dist-info/RECORD",
        "tei_rapporteur-<version>.dist-info/WHEEL",
        "tei_rapporteur-<version>.dist-info/licenses/LICENSE",
        "tei_rapporteur-<version>.dist-info/sboms/<sbom>.cyclonedx.json",
        "tei_rapporteur/__init__.py",
        "tei_rapporteur/__init__.pyi",
        "tei_rapporteur/py.typed",
        "tei_rapporteur/structs.pyi",
        "tei_rapporteur/tei_rapporteur.cpython-<platform>.so",
    ],
    "generator": "1.13.3",
    "metadata": {
        "classifiers": [
            "License :: OSI Approved :: ISC License (ISCL)",
            "Operating System :: POSIX",
            "Operating System :: Unix",
            "Programming Language :: Python",
            "Programming Language :: Python :: 3",
            "Programming Language :: Python :: 3 :: Only",
            "Programming Language :: Rust",
        ],
        "name": "tei-rapporteur",
        "requires_dist": [
            "msgspec>=0.19,<0.20",
            "msgspec>=0.19,<0.20 ; extra == 'dev'",
        ],
        "requires_python": ">=3.11",
        "version": "0.1.0",
    },
    "wheel": {
        "root_is_purelib": "false",
        "tag": "<platform-tag>",
    },
}


def test_maturin_pins_are_synchronized() -> None:
    """Maturin version pins stay aligned across CI and packaging metadata."""

    pins = read_maturin_pins(repo_root())
    assert len(set(pins.values())) == 1, f"Expected one maturin pin, found {pins!r}"


def test_installed_maturin_matches_expected_pin() -> None:
    """The active maturin CLI matches the pinned development dependency."""

    installed = installed_maturin_version()
    if installed is None:
        pytest.skip("maturin is not installed.")
    expected = read_expected_maturin_version(repo_root())
    assert installed == expected, (
        f"Expected maturin {expected}, but {installed} is installed"
    )


def test_maturin_wheel_build_snapshot(tmp_path: Path) -> None:
    """Native wheel metadata and layout match the expected maturin output."""

    root = repo_root()
    expected = read_expected_maturin_version(root)
    if not toolchain_available():
        pytest.skip("Rust toolchain or maturin unavailable.")
    if sys.version_info >= (3, 15):
        pytest.skip(f"maturin {expected} does not support this Python version.")

    wheel_path = build_native_wheel_artifact(root, tmp_path / "wheelhouse")
    snapshot = wheel_build_snapshot(wheel_path)
    assert snapshot["generator"] == expected, (
        f"Expected generator {expected!r}, found {snapshot['generator']!r}"
    )
    assert snapshot == EXPECTED_WHEEL_SNAPSHOT, (
        f"Wheel snapshot mismatch.\nActual:   {snapshot!r}\n"
        f"Expected: {EXPECTED_WHEEL_SNAPSHOT!r}"
    )
