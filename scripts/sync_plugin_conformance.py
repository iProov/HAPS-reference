#!/usr/bin/env python3
"""Sync the portable test kit and the repository's exact license/notice text."""

from __future__ import annotations

import argparse
import filecmp
import shutil
from pathlib import Path

ROOT_FILES = [
    "adapter-protocol-v1.md",
    "haps-conformance-v0.4.lock.json",
    "haps_conformance.py",
    "suite-v0.4.json",
]
LEGAL_FILES = ["LICENSE", "NOTICE"]


def tree_mismatches(source: Path, destination: Path, prefix: str = "") -> list[str]:
    if not destination.is_dir():
        return [prefix or str(destination)]
    source_files = {
        path.relative_to(source) for path in source.rglob("*") if path.is_file()
    }
    destination_files = {
        path.relative_to(destination) for path in destination.rglob("*") if path.is_file()
    }
    mismatches = [f"{prefix}{path}" for path in sorted(source_files ^ destination_files)]
    for relative in sorted(source_files & destination_files):
        if not filecmp.cmp(source / relative, destination / relative, shallow=False):
            mismatches.append(f"{prefix}{relative}")
    return mismatches


def main() -> int:
    parser = argparse.ArgumentParser(description="Sync portable conformance assets into the plugin.")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write", action="store_true")
    args = parser.parse_args()

    repo = Path(__file__).resolve().parent.parent
    source = repo / "conformance"
    vectors = repo / "test_vectors" / "v0.4"
    plugin = repo / "plugins" / "haps-interoperability"
    destination = plugin / "assets" / "conformance"

    missing_legal = [name for name in LEGAL_FILES if not (repo / name).is_file()]
    if missing_legal:
        print("repository licensing files are missing: " + ", ".join(missing_legal))
        return 1

    if args.write:
        destination.mkdir(parents=True, exist_ok=True)
        for name in LEGAL_FILES:
            shutil.copy2(repo / name, plugin / name)
            shutil.copy2(repo / name, destination / name)
        for name in ROOT_FILES:
            shutil.copy2(source / name, destination / name)
        vector_destination = destination / "vectors" / "v0.4"
        if vector_destination.exists():
            shutil.rmtree(vector_destination)
        vector_destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(vectors, vector_destination)
        print(f"synced portable kit to {destination}")
        return 0

    if not destination.is_dir():
        print(f"plugin conformance asset is missing: {destination}")
        return 1
    mismatches = []
    for name in LEGAL_FILES:
        for copy in (plugin / name, destination / name):
            if not copy.is_file() or not filecmp.cmp(repo / name, copy, shallow=False):
                mismatches.append(str(copy.relative_to(plugin)))
    for name in ROOT_FILES:
        if not (destination / name).is_file() or not filecmp.cmp(
            source / name, destination / name, shallow=False
        ):
            mismatches.append(name)
    vector_destination = destination / "vectors" / "v0.4"
    mismatches.extend(
        f"vectors/v0.4/{name}" for name in tree_mismatches(vectors, vector_destination)
    )
    if mismatches:
        print("plugin conformance assets drifted: " + ", ".join(mismatches))
        return 1
    print("plugin conformance assets are in sync")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
