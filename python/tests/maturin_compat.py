"""Shared helpers for maturin compatibility and wheel build tests."""

from __future__ import annotations

import importlib.metadata as metadata
import importlib.util
import os
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path
from typing import Any

MATURIN_VERSION_RE = re.compile(r"maturin==(\d+\.\d+\.\d+)")
CI_MATURIN_VERSION_RE = re.compile(r'MATURIN_VERSION:\s*"(\d+\.\d+\.\d+)"')
GENERATOR_RE = re.compile(r"^Generator:\s*maturin\s*\(([^)]+)\)\s*$", re.MULTILINE)
EXTENSION_MODULE_RE = re.compile(r"^tei_rapporteur/tei_rapporteur\.cpython-[^/]+\.so$")
DIST_INFO_SUFFIXES: dict[str, str] = {
    ".dist-info/METADATA": "tei_rapporteur-<version>.dist-info/METADATA",
    ".dist-info/RECORD": "tei_rapporteur-<version>.dist-info/RECORD",
    ".dist-info/WHEEL": "tei_rapporteur-<version>.dist-info/WHEEL",
    ".dist-info/licenses/LICENSE": (
        "tei_rapporteur-<version>.dist-info/licenses/LICENSE"
    ),
}


def repo_root() -> Path:
    """Return the repository root for tests run from any working directory."""

    return Path(__file__).resolve().parents[2]


def read_maturin_pins(root: Path) -> dict[str, str]:
    """Read maturin pins from local packaging and CI configuration files."""

    pyproject = (root / "pyproject.toml").read_text(encoding="utf-8")
    workflow = (root / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    return {
        "pyproject.toml": require_pin(
            MATURIN_VERSION_RE.search(pyproject),
            "pyproject.toml",
        ),
        ".github/workflows/ci.yml": require_pin(
            CI_MATURIN_VERSION_RE.search(workflow),
            ".github/workflows/ci.yml",
        ),
    }


def require_pin(match: re.Match[str] | None, location: str) -> str:
    """Extract a version from a regex match or raise a focused assertion."""

    if match is None:
        msg = f"Could not locate maturin version pin in {location}"
        raise AssertionError(msg)
    return match.group(1)


def read_expected_maturin_version(root: Path) -> str:
    """Read the maturin version pinned in ``pyproject.toml``."""

    pyproject = (root / "pyproject.toml").read_text(encoding="utf-8")
    return require_pin(MATURIN_VERSION_RE.search(pyproject), "pyproject.toml")


def installed_maturin_version() -> str | None:
    """Return the installed maturin package version, if maturin is present."""

    if shutil.which("maturin") is None:
        return None
    try:
        return metadata.version("maturin")
    except metadata.PackageNotFoundError:
        return None


def toolchain_available() -> bool:
    """Return whether the Rust toolchain and Python maturin module are usable."""

    return (
        shutil.which("cargo") is not None
        and shutil.which("rustc") is not None
        and importlib.util.find_spec("maturin") is not None
    )


def build_native_wheel_artifact(root: Path, out_dir: Path) -> Path:
    """Build a native wheel with the pinned maturin backend."""

    out_dir.mkdir(parents=True, exist_ok=True)
    command = [
        sys.executable,
        "-m",
        "maturin",
        "build",
        "--release",
        "--out",
        str(out_dir),
        "--manifest-path",
        str(root / "tei-py/Cargo.toml"),
    ]
    env = os.environ.copy()
    env["TEI_PY_BUILD_EXTENSION"] = "1"
    subprocess.run(command, check=True, cwd=root, env=env)
    wheels = sorted(out_dir.glob("*.whl"))
    if len(wheels) != 1:
        msg = f"Expected exactly one wheel in {out_dir}, found {wheels!r}"
        raise AssertionError(msg)
    return wheels[0]


def wheel_build_snapshot(whl_path: Path) -> dict[str, Any]:
    """Return normalised wheel metadata and layout for compatibility checks."""

    with zipfile.ZipFile(whl_path) as archive:
        entry_names = archive.namelist()
        wheel_name = locate_wheel_metadata(entry_names)
        metadata_name = wheel_name.replace("/WHEEL", "/METADATA")
        wheel_payload = archive.read(wheel_name).decode("utf-8")
        metadata_payload = archive.read(metadata_name).decode("utf-8")
    generator, root_is_purelib = parse_wheel_header(wheel_payload, whl_path)
    return {
        "entries": sorted(normalise_wheel_entry(name) for name in entry_names),
        "generator": generator,
        "metadata": parse_metadata(metadata_payload),
        "wheel": {
            "root_is_purelib": root_is_purelib,
            "tag": "<platform-tag>",
        },
    }


def locate_wheel_metadata(entry_names: list[str]) -> str:
    """Return the ``.dist-info/WHEEL`` entry name from a wheel archive."""

    wheel_name = next(
        (name for name in entry_names if name.endswith(".dist-info/WHEEL")),
        None,
    )
    if wheel_name is None:
        msg = "wheel is missing .dist-info/WHEEL metadata"
        raise AssertionError(msg)
    return wheel_name


def parse_wheel_header(wheel_payload: str, whl_path: Path) -> tuple[str, str]:
    """Extract maturin generator and purelib metadata from a wheel header."""

    generator_match = GENERATOR_RE.search(wheel_payload)
    if generator_match is None:
        msg = f"Could not parse maturin generator from WHEEL metadata: {whl_path}"
        raise AssertionError(msg)
    root_is_purelib = next(
        (
            line.removeprefix("Root-Is-Purelib: ")
            for line in wheel_payload.splitlines()
            if line.startswith("Root-Is-Purelib:")
        ),
        None,
    )
    if root_is_purelib is None:
        msg = "wheel is missing Root-Is-Purelib metadata"
        raise AssertionError(msg)
    return generator_match.group(1), root_is_purelib


def parse_metadata(raw_metadata: str) -> dict[str, Any]:
    """Parse RFC 2822-style package metadata into a stable dictionary."""

    headers: dict[str, list[str]] = {}
    current_key: str | None = None
    for line in raw_metadata.splitlines():
        if line.startswith((" ", "\t")) and current_key is not None:
            headers[current_key][-1] = f"{headers[current_key][-1]} {line.strip()}"
            continue
        if ":" not in line:
            break
        key, value = line.split(":", 1)
        current_key = key.strip()
        headers.setdefault(current_key, []).append(value.strip())

    return {
        "classifiers": sorted(headers.get("Classifier", [])),
        "name": header_value(headers, "Name"),
        "requires_dist": sorted(headers.get("Requires-Dist", [])),
        "requires_python": header_value(headers, "Requires-Python"),
        "version": header_value(headers, "Version"),
    }


def header_value(headers: dict[str, list[str]], key: str) -> str | None:
    """Return the first header value for ``key`` when one is present."""

    values = headers.get(key)
    if not values:
        return None
    return values[0]


def normalise_wheel_entry(name: str) -> str:
    """Normalize versioned or platform-specific wheel entry names."""

    if EXTENSION_MODULE_RE.match(name):
        return "tei_rapporteur/tei_rapporteur.cpython-<platform>.so"
    if "/sboms/" in name:
        return "tei_rapporteur-<version>.dist-info/sboms/<sbom>.cyclonedx.json"
    for suffix, normalised in DIST_INFO_SUFFIXES.items():
        if name.endswith(suffix):
            return normalised
    return name
