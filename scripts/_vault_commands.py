"""Helpers for interacting with HashiCorp Vault during bootstrap."""

from __future__ import annotations

import json
from typing import Any, Dict


class VaultBootstrapError(RuntimeError):
    """Error raised when Vault bootstrap fails."""


def _generate_secret_id(payload: str) -> str:
    """Extract a secret-id from a Vault response payload."""
    try:
        data: Dict[str, Any] = json.loads(payload)
    except json.JSONDecodeError as exc:  # pragma: no cover - defensive guard
        msg = "Failed to decode Vault response whilst generating a secret-id"
        raise VaultBootstrapError(msg) from exc

    if secret_id := data.get("data", {}).get("secret_id"):
        return secret_id
    raise VaultBootstrapError("Vault response missing secret_id field")
