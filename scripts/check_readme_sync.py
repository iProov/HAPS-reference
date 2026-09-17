#!/usr/bin/env python3
"""Fail when README inventories drift from repository-owned sources of truth."""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
README_PATH = ROOT / "README.md"


def require(readme: str, snippet: str, description: str, errors: list[str]) -> None:
    if snippet not in readme:
        errors.append(f"missing {description}: {snippet!r}")


def workspace_packages() -> list[tuple[str, Path]]:
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    packages: list[tuple[str, Path]] = []
    for member in workspace["workspace"]["members"]:
        member_path = ROOT / member
        manifest = tomllib.loads((member_path / "Cargo.toml").read_text(encoding="utf-8"))
        packages.append((manifest["package"]["name"], member_path))
    return packages


def annotation_count(package_path: Path, annotation: str) -> int:
    pattern = re.compile(rf"#\s*\[\s*{re.escape(annotation)}\s*\]")
    return sum(
        len(pattern.findall(source.read_text(encoding="utf-8")))
        for source in package_path.rglob("*.rs")
    )


def suite_execution_count() -> int:
    suite = json.loads(
        (ROOT / "conformance" / "suite-v0.4.json").read_text(encoding="utf-8")
    )
    return sum(len(case.get("variants") or ["source"]) for case in suite["cases"])


def main() -> int:
    readme = README_PATH.read_text(encoding="utf-8")
    errors: list[str] = []
    packages = workspace_packages()

    for package, _ in packages:
        require(readme, package, f"workspace package {package}", errors)

    core_tests = 0
    workspace_tests = 0
    proof_count = 0
    for package, package_path in packages:
        tests = annotation_count(package_path, "test")
        proofs = annotation_count(package_path, "kani::proof")
        workspace_tests += tests
        proof_count += proofs
        if package == "haps-canon":
            core_tests = tests
        if proofs:
            require(
                readme,
                f"cargo kani -p {package}",
                f"Kani command for proof-bearing package {package}",
                errors,
            )

    require(readme, f"Core tests — {core_tests},", "core test count", errors)
    require(
        readme,
        f"currently runs {workspace_tests} tests",
        "workspace test count",
        errors,
    )
    require(readme, f"Kani — {proof_count} harnesses", "Kani harness count", errors)
    require(readme, "haps init", "language-neutral initialization command", errors)

    skills_root = ROOT / "plugins" / "haps-interoperability" / "skills"
    skills = sorted(path.parent.name for path in skills_root.glob("*/SKILL.md"))
    for skill in skills:
        require(readme, f"`{skill}`", f"plugin skill {skill}", errors)

    # Keep the human-readable source revision aligned with the portable lock.
    # This checks consistency, not authenticity of the claimed upstream commit.
    lock = json.loads((ROOT / "conformance/haps-conformance-v0.4.lock.json").read_text())
    source_note = (ROOT / "test_vectors/SOURCE.md").read_text()
    require(source_note, f"copied from commit: {lock['specification']['commit']}",
            "vector source revision matching the lock", errors)

    executions = suite_execution_count()
    require(
        readme,
        f"currently executes {executions} checks",
        "conformance execution count",
        errors,
    )

    if errors:
        print("README.md is out of sync:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        "README.md is synchronized with "
        f"{len(packages)} workspace packages, {workspace_tests} Rust tests, "
        f"{proof_count} Kani harnesses, {len(skills)} plugin skills, and "
        f"{executions} conformance executions."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
