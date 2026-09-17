#!/usr/bin/env python3
"""Experimental Python HAPS v0.4 canonicalization/hash adapter example.

Partial, not production-ready. Passing the finite suite does not establish full
protocol conformance, correct display, or human approval. Input/memory bounds
and process isolation are outside this example; depth is checked after parsing.
"""

from __future__ import annotations

import base64
import hashlib
import json
import sys
from typing import Any

PROTOCOL = "haps-conformance-adapter-v1"
MAX_SAFE_INT = 9_007_199_254_740_991
MAX_DEPTH = 64


class HAPSError(Exception):
    def __init__(self, code: str) -> None:
        super().__init__(code)
        self.code = code


def reject_float(_: str) -> None:
    raise HAPSError("NON_INTEGER_NUMBER")


def parse_integer(token: str) -> int:
    number = int(token)
    if not -MAX_SAFE_INT <= number <= MAX_SAFE_INT:
        raise HAPSError("INTEGER_OUT_OF_RANGE")
    return 0 if number == 0 else number


def reject_constant(_: str) -> None:
    raise HAPSError("SYNTAX")


def object_without_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise HAPSError("DUPLICATE_KEY")
        value[key] = item
    return value


def validate(value: Any, depth: int = 0) -> None:
    if depth > MAX_DEPTH:
        raise HAPSError("DEPTH_EXCEEDED")
    if isinstance(value, str):
        if any(0xD800 <= ord(character) <= 0xDFFF for character in value):
            raise HAPSError("SYNTAX")
    elif isinstance(value, list):
        for item in value:
            validate(item, depth + 1)
    elif isinstance(value, dict):
        for key, item in value.items():
            validate(key, depth + 1)
            validate(item, depth + 1)


def parse_document(document: str) -> Any:
    decoder = json.JSONDecoder(
        parse_float=reject_float,
        parse_int=parse_integer,
        parse_constant=reject_constant,
        object_pairs_hook=object_without_duplicates,
    )
    start = 0
    while start < len(document) and document[start] in " \t\n\r":
        start += 1
    try:
        value, end = decoder.raw_decode(document, start)
    except HAPSError:
        raise
    except (ValueError, RecursionError) as error:
        raise HAPSError("SYNTAX") from error
    while end < len(document) and document[end] in " \t\n\r":
        end += 1
    if end != len(document):
        raise HAPSError("TRAILING_CONTENT")
    validate(value)
    return value


def utf16_key(text: str) -> bytes:
    return text.encode("utf-16-be")


def canonicalize(value: Any) -> str:
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if isinstance(value, list):
        return "[" + ",".join(canonicalize(item) for item in value) + "]"
    if isinstance(value, dict):
        members = (
            canonicalize(key) + ":" + canonicalize(value[key])
            for key in sorted(value, key=utf16_key)
        )
        return "{" + ",".join(members) + "}"
    raise HAPSError("INTERNAL_ERROR")


def digest(canonical: str) -> str:
    raw = hashlib.sha256(canonical.encode("utf-8")).digest()
    encoded = base64.urlsafe_b64encode(raw).rstrip(b"=").decode("ascii")
    return f"sha256:{encoded}"


def error_response(request_id: str, code: str) -> dict[str, Any]:
    return {
        "protocol": PROTOCOL,
        "id": request_id,
        "status": "error",
        "error": {"code": code},
    }


def respond(request: Any) -> dict[str, Any]:
    if not isinstance(request, dict):
        return error_response("", "PROTOCOL_ERROR")
    request_id = request.get("id")
    if not isinstance(request_id, str) or request.get("protocol") != PROTOCOL:
        return error_response("", "PROTOCOL_ERROR")
    if request.get("operation") == "capabilities":
        return {
            "protocol": PROTOCOL,
            "id": request_id,
            "status": "ok",
            "implementation": {"name": "haps-python-example", "version": "0.1.0"},
            "haps_versions": ["0.4"],
            "operations": ["canonicalize_and_hash"],
        }
    if request.get("operation") != "canonicalize_and_hash":
        return error_response(request_id, "UNSUPPORTED_OPERATION")
    document = request.get("document")
    if not isinstance(document, str):
        return error_response(request_id, "PROTOCOL_ERROR")
    try:
        canonical = canonicalize(parse_document(document))
        return {
            "protocol": PROTOCOL,
            "id": request_id,
            "status": "ok",
            "canonical": canonical,
            "hash": digest(canonical),
        }
    except HAPSError as error:
        return error_response(request_id, error.code)


def main() -> int:
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            response = error_response("", "PROTOCOL_ERROR")
        else:
            response = respond(request)
        print(json.dumps(response, ensure_ascii=False, separators=(",", ":")), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
