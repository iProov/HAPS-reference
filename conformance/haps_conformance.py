#!/usr/bin/env python3
"""Deterministic HAPS canonicalization/hash conformance runner.

Each request starts a fresh local process, writes one NDJSON request, closes
stdin, and requires exactly one NDJSON response. No shell is involved, but the
adapter is not sandboxed and runs with the invoking user's permissions.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

PROTOCOL = "haps-conformance-adapter-v1"
RUNNER_VERSION = "0.1.0"
MAX_RESPONSE_BYTES = 1_048_576
RESULT_MEANING = (
    "Experimental draft specification and partial reference implementation; not production-ready. "
    "Passing these finite tests does not establish full protocol conformance or that a person "
    "saw and approved the correct action."
)


class ConfigurationError(Exception):
    """The suite, lock, vectors, or command line is invalid."""


class AdapterError(Exception):
    """The adapter violated the process or wire-protocol contract."""


@dataclass(frozen=True)
class CaseResult:
    id: str
    classification: str
    passed: bool
    message: str
    duration_ms: int


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65_536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ConfigurationError(f"cannot read JSON from {path}: {error}") from error
    if not isinstance(value, dict):
        raise ConfigurationError(f"{path} must contain a JSON object")
    return value


def default_vector_root(script_dir: Path) -> Path:
    portable = script_dir / "vectors" / "v0.4"
    if portable.is_dir():
        return portable
    checkout = script_dir.parent / "test_vectors" / "v0.4"
    if checkout.is_dir():
        return checkout
    return portable


def resolve_vector(vector_root: Path, relative: str) -> Path:
    root = vector_root.resolve()
    candidate = (root / relative).resolve()
    if candidate != root and root not in candidate.parents:
        raise ConfigurationError(f"vector path escapes vector root: {relative}")
    return candidate


def verify_lock(suite_path: Path, lock_path: Path, vector_root: Path) -> dict[str, Any]:
    lock = load_json(lock_path)
    if lock.get("schema_version") != 1:
        raise ConfigurationError("unsupported conformance lock schema")

    suite_expected = lock.get("suite", {}).get("sha256")
    if not isinstance(suite_expected, str) or sha256_file(suite_path) != suite_expected:
        raise ConfigurationError(f"suite digest does not match {lock_path}")

    protocol_entry = lock.get("protocol", {})
    protocol_name = protocol_entry.get("path")
    protocol_expected = protocol_entry.get("sha256")
    if not isinstance(protocol_name, str) or not isinstance(protocol_expected, str):
        raise ConfigurationError("lock has no valid protocol digest")
    protocol_path = (suite_path.parent / protocol_name).resolve()
    if not protocol_path.is_file() or sha256_file(protocol_path) != protocol_expected:
        raise ConfigurationError(f"protocol digest does not match {lock_path}")

    runner_expected = lock.get("runner", {}).get("sha256")
    runner_path = Path(__file__).resolve()
    if not isinstance(runner_expected, str) or sha256_file(runner_path) != runner_expected:
        raise ConfigurationError(f"runner digest does not match {lock_path}")

    vectors = lock.get("vectors")
    if not isinstance(vectors, dict) or not vectors:
        raise ConfigurationError("lock has no vector digests")
    for relative, expected in vectors.items():
        if not isinstance(relative, str) or not isinstance(expected, str):
            raise ConfigurationError("lock contains an invalid vector digest entry")
        path = resolve_vector(vector_root, relative)
        if not path.is_file():
            raise ConfigurationError(f"locked vector is missing: {path}")
        if sha256_file(path) != expected:
            raise ConfigurationError(f"vector digest mismatch: {path}")
    return lock


def adapter_request(
    command: list[str], request: dict[str, Any], timeout: float
) -> dict[str, Any]:
    wire = json.dumps(request, ensure_ascii=False, separators=(",", ":")) + "\n"
    try:
        completed = subprocess.run(
            command,
            input=wire,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except FileNotFoundError as error:
        raise AdapterError(f"adapter executable not found: {command[0]}") from error
    except subprocess.TimeoutExpired as error:
        raise AdapterError(f"adapter timed out after {timeout:g}s") from error
    except OSError as error:
        raise AdapterError(f"cannot execute adapter: {error}") from error

    if completed.returncode != 0:
        detail = completed.stderr.strip()[:1000]
        suffix = f": {detail}" if detail else ""
        raise AdapterError(f"adapter exited {completed.returncode}{suffix}")
    if len(completed.stdout.encode("utf-8")) > MAX_RESPONSE_BYTES:
        raise AdapterError("adapter response exceeds 1 MiB")
    lines = [line for line in completed.stdout.splitlines() if line.strip()]
    if len(lines) != 1:
        raise AdapterError(f"adapter must emit exactly one response line; got {len(lines)}")
    try:
        response = json.loads(lines[0])
    except json.JSONDecodeError as error:
        raise AdapterError(f"adapter emitted invalid JSON: {error}") from error
    if not isinstance(response, dict):
        raise AdapterError("adapter response must be a JSON object")
    if response.get("protocol") != PROTOCOL:
        raise AdapterError("adapter response has the wrong protocol")
    if response.get("id") != request["id"]:
        raise AdapterError("adapter response id does not match the request")
    return response


def reversed_compact(document: str) -> str:
    def reverse(value: Any) -> Any:
        if isinstance(value, dict):
            return {key: reverse(item) for key, item in reversed(list(value.items()))}
        if isinstance(value, list):
            return [reverse(item) for item in value]
        return value

    parsed = json.loads(document)
    return json.dumps(reverse(parsed), ensure_ascii=False, separators=(",", ":"))


def expected_hash(case: dict[str, Any], vector_root: Path) -> str | None:
    expected = case.get("expect", {})
    literal = expected.get("hash")
    filename = expected.get("hash_file")
    if isinstance(literal, str):
        return literal
    if isinstance(filename, str):
        path = resolve_vector(vector_root, filename)
        try:
            return path.read_text(encoding="utf-8").strip()
        except (OSError, UnicodeError) as error:
            raise ConfigurationError(f"cannot read expected hash {path}: {error}") from error
    return None


def prefixed_sha256(canonical: str) -> str:
    digest = hashlib.sha256(canonical.encode("utf-8")).digest()
    encoded = base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")
    return f"sha256:{encoded}"


def validate_success(
    response: dict[str, Any], expected: str, command: list[str], case_id: str, timeout: float
) -> str | None:
    if response.get("status") != "ok":
        code = response.get("error", {}).get("code")
        return f"expected success, got error {code!r}"
    canonical = response.get("canonical")
    actual_hash = response.get("hash")
    if not isinstance(canonical, str) or not isinstance(actual_hash, str):
        return "successful response must contain string canonical and hash fields"
    independently_hashed = prefixed_sha256(canonical)
    if actual_hash != independently_hashed:
        return "reported hash does not match SHA-256 of returned canonical UTF-8 bytes"
    if actual_hash != expected:
        return f"hash mismatch: expected {expected}, got {actual_hash}"

    fixed_request = {
        "protocol": PROTOCOL,
        "id": f"{case_id}:fixed-point",
        "operation": "canonicalize_and_hash",
        "document": canonical,
    }
    try:
        fixed = adapter_request(command, fixed_request, timeout)
    except AdapterError as error:
        return f"fixed-point request failed: {error}"
    if fixed.get("status") != "ok":
        return "adapter rejected its own canonical output"
    if fixed.get("canonical") != canonical or fixed.get("hash") != actual_hash:
        return "canonical form is not a fixed point"
    return None


def run_case(
    case: dict[str, Any],
    variant: str,
    document: str,
    command: list[str],
    timeout: float,
    vector_root: Path,
) -> CaseResult:
    base_id = str(case.get("id", "unnamed"))
    case_id = base_id if variant == "source" else f"{base_id}:{variant}"
    classification = str(case.get("classification", "unspecified"))
    started = time.monotonic()
    request = {
        "protocol": PROTOCOL,
        "id": case_id,
        "operation": "canonicalize_and_hash",
        "document": document,
    }
    try:
        response = adapter_request(command, request, timeout)
        expectation = case.get("expect")
        if not isinstance(expectation, dict):
            raise ConfigurationError(f"case {base_id} has no expectation")
        wanted_status = expectation.get("status")
        failure: str | None
        if wanted_status == "ok":
            wanted_hash = expected_hash(case, vector_root)
            if wanted_hash is None:
                raise ConfigurationError(f"case {base_id} has no expected hash")
            failure = validate_success(response, wanted_hash, command, case_id, timeout)
        elif wanted_status == "error":
            wanted_code = expectation.get("code")
            actual_code = response.get("error", {}).get("code")
            failure = None
            if response.get("status") != "error" or actual_code != wanted_code:
                failure = f"expected error {wanted_code!r}, got {response.get('status')!r}/{actual_code!r}"
        else:
            raise ConfigurationError(f"case {base_id} has invalid expected status")
    except AdapterError as error:
        failure = str(error)

    elapsed = round((time.monotonic() - started) * 1000)
    return CaseResult(
        id=case_id,
        classification=classification,
        passed=failure is None,
        message="passed" if failure is None else failure,
        duration_ms=elapsed,
    )


def run_suite(
    suite: dict[str, Any], vector_root: Path, command: list[str], timeout: float
) -> tuple[dict[str, Any], list[CaseResult]]:
    capability_request = {
        "protocol": PROTOCOL,
        "id": "capabilities",
        "operation": "capabilities",
    }
    capabilities = adapter_request(command, capability_request, timeout)
    if capabilities.get("status") != "ok":
        raise AdapterError("adapter rejected the capabilities request")
    if "0.4" not in capabilities.get("haps_versions", []):
        raise AdapterError("adapter does not declare HAPS v0.4 support")
    if "canonicalize_and_hash" not in capabilities.get("operations", []):
        raise AdapterError("adapter does not declare canonicalize_and_hash support")

    raw_cases = suite.get("cases")
    if not isinstance(raw_cases, list) or not raw_cases:
        raise ConfigurationError("suite has no cases")
    results: list[CaseResult] = []
    for case in raw_cases:
        if not isinstance(case, dict):
            raise ConfigurationError("suite contains a non-object case")
        source = case.get("input")
        inline = case.get("document")
        if isinstance(source, str):
            path = resolve_vector(vector_root, source)
            try:
                document = path.read_text(encoding="utf-8")
            except (OSError, UnicodeError) as error:
                raise ConfigurationError(f"cannot read vector {path}: {error}") from error
        elif isinstance(inline, str):
            document = inline
        else:
            raise ConfigurationError(f"case {case.get('id')} has no input")

        variants = case.get("variants", ["source"])
        if not isinstance(variants, list) or not variants:
            raise ConfigurationError(f"case {case.get('id')} has invalid variants")
        for variant in variants:
            if variant == "source":
                variant_document = document
            elif variant == "reversed_compact":
                variant_document = reversed_compact(document)
            else:
                raise ConfigurationError(f"unsupported input variant: {variant}")
            results.append(run_case(case, variant, variant_document, command, timeout, vector_root))
    return capabilities, results


def report_payload(
    suite: dict[str, Any], lock: dict[str, Any], capabilities: dict[str, Any],
    results: list[CaseResult], command: list[str]
) -> dict[str, Any]:
    passed = sum(result.passed for result in results)
    return {
        "schema_version": 1,
        "runner_version": RUNNER_VERSION,
        "suite_id": suite.get("suite_id"),
        "scope": suite.get("scope"),
        "result_meaning": RESULT_MEANING,
        "haps_version": suite.get("haps_version"),
        "specification": lock.get("specification"),
        "adapter_command": command,
        "adapter": capabilities.get("implementation"),
        "passed": passed,
        "failed": len(results) - passed,
        "conformant_for_suite_scope": passed == len(results),
        "results": [asdict(result) for result in results],
    }


def write_json_report(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def write_junit_report(path: Path, payload: dict[str, Any]) -> None:
    results = payload["results"]
    suite = ET.Element(
        "testsuite",
        name=str(payload["suite_id"]),
        tests=str(len(results)),
        failures=str(payload["failed"]),
        time=f"{sum(item['duration_ms'] for item in results) / 1000:.3f}",
    )
    properties = ET.SubElement(suite, "properties")
    for name in ("scope", "result_meaning"):
        ET.SubElement(properties, "property", name=name, value=str(payload[name]))
    for item in results:
        case = ET.SubElement(
            suite,
            "testcase",
            name=item["id"],
            classname=f"haps.{item['classification']}",
            time=f"{item['duration_ms'] / 1000:.3f}",
        )
        if not item["passed"]:
            ET.SubElement(case, "failure", message=item["message"]).text = item["message"]
    path.parent.mkdir(parents=True, exist_ok=True)
    ET.ElementTree(suite).write(path, encoding="utf-8", xml_declaration=True)


def build_parser(script_dir: Path) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run the pinned HAPS v0.4 canonicalization/hash interoperability suite."
    )
    parser.add_argument("--version", action="version", version=RUNNER_VERSION)
    subparsers = parser.add_subparsers(dest="command", required=True)

    def common(subparser: argparse.ArgumentParser) -> None:
        subparser.add_argument("--suite", type=Path, default=script_dir / "suite-v0.4.json")
        subparser.add_argument(
            "--lock", type=Path, default=script_dir / "haps-conformance-v0.4.lock.json"
        )
        subparser.add_argument("--vectors", type=Path, default=default_vector_root(script_dir))

    verify = subparsers.add_parser("verify-lock", help="check local kit digests against the lock")
    common(verify)

    test = subparsers.add_parser("test", help="run an adapter against the suite")
    common(test)
    test.add_argument("--adapter", required=True, help="adapter executable path")
    test.add_argument("--adapter-arg", action="append", default=[], help="adapter argument")
    test.add_argument("--timeout", type=float, default=10.0, help="seconds per adapter request")
    test.add_argument("--json-report", type=Path)
    test.add_argument("--junit-report", type=Path)
    return parser


def main(argv: list[str] | None = None) -> int:
    script_dir = Path(__file__).resolve().parent
    args = build_parser(script_dir).parse_args(argv)
    try:
        suite_path = args.suite.resolve()
        lock_path = args.lock.resolve()
        vector_root = args.vectors.resolve()
        lock = verify_lock(suite_path, lock_path, vector_root)
        if args.command == "verify-lock":
            print(
                f"local kit digests match the lock for {lock['suite']['id']}; recorded specification "
                f"{lock['specification']['commit']} ({len(lock['vectors'])} locked files)"
            )
            return 0

        suite = load_json(suite_path)
        command = [args.adapter, *args.adapter_arg]
        capabilities, results = run_suite(suite, vector_root, command, args.timeout)
        payload = report_payload(suite, lock, capabilities, results, command)
        for result in results:
            marker = "PASS" if result.passed else "FAIL"
            print(f"{marker} {result.id}: {result.message}")
        print(
            f"{payload['passed']} passed, {payload['failed']} failed — "
            f"scope: {payload['scope']}"
        )
        print(RESULT_MEANING)
        if args.json_report:
            write_json_report(args.json_report, payload)
        if args.junit_report:
            write_junit_report(args.junit_report, payload)
        return 0 if payload["conformant_for_suite_scope"] else 1
    except (ConfigurationError, AdapterError, json.JSONDecodeError) as error:
        print(f"haps-conformance: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
