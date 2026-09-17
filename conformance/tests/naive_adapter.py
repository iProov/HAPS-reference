#!/usr/bin/env python3
"""Deliberately non-conformant adapter used as a negative control."""

from __future__ import annotations

import base64
import hashlib
import json
import sys

PROTOCOL = "haps-conformance-adapter-v1"


def respond(request: dict[str, object]) -> dict[str, object]:
    request_id = request.get("id", "")
    if request.get("operation") == "capabilities":
        return {
            "protocol": PROTOCOL,
            "id": request_id,
            "status": "ok",
            "implementation": {"name": "naive-negative-control", "version": "0"},
            "haps_versions": ["0.4"],
            "operations": ["canonicalize_and_hash"],
        }
    try:
        # Intentionally wrong: loses duplicate keys, accepts floats/exponents,
        # and orders astral keys by scalar value rather than UTF-16 units.
        value = json.loads(str(request["document"]))
        canonical = json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
        digest = hashlib.sha256(canonical.encode("utf-8")).digest()
        encoded = base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")
        return {
            "protocol": PROTOCOL,
            "id": request_id,
            "status": "ok",
            "canonical": canonical,
            "hash": f"sha256:{encoded}",
        }
    except (KeyError, TypeError, json.JSONDecodeError):
        return {
            "protocol": PROTOCOL,
            "id": request_id,
            "status": "error",
            "error": {"code": "SYNTAX"},
        }


for raw_line in sys.stdin:
    if raw_line.strip():
        print(json.dumps(respond(json.loads(raw_line)), ensure_ascii=False), flush=True)
