#!/usr/bin/env python3
"""Regenerate the conformance lock after an explicit suite/specification update."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

SPEC_REPOSITORY = "https://github.com/iProov/HAPS"
SPEC_COMMIT = "11ed170b3121177d971ea7f66c87981656d03daa"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    root = Path(__file__).resolve().parent
    vector_root = root.parent / "test_vectors" / "v0.4"
    suite_path = root / "suite-v0.4.json"
    protocol_path = root / "adapter-protocol-v1.md"
    runner_path = root / "haps_conformance.py"
    suite = json.loads(suite_path.read_text(encoding="utf-8"))
    vector_names = sorted(
        str(path.relative_to(vector_root))
        for path in vector_root.rglob("*")
        if path.is_file()
    )

    lock = {
        "schema_version": 1,
        "suite": {
            "id": suite["suite_id"],
            "sha256": digest(suite_path),
        },
        "protocol": {
            "path": protocol_path.name,
            "id": suite["adapter_protocol"],
            "sha256": digest(protocol_path),
        },
        "runner": {
            "version": "0.1.0",
            "sha256": digest(runner_path),
        },
        "haps_version": suite["haps_version"],
        "specification": {
            "repository": SPEC_REPOSITORY,
            "commit": SPEC_COMMIT,
            "status": "draft",
            "provenance_note": (
                "All vendored vector files, including their README, match this specification commit. "
                "Suite, runner and protocol documentation are maintained in HAPS-reference. "
                "Local digest checks do not authenticate upstream provenance."
            ),
        },
        "vectors": {
            name: digest(vector_root / name) for name in vector_names
        },
    }
    output = root / "haps-conformance-v0.4.lock.json"
    output.write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output} with {len(vector_names)} vector digests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
