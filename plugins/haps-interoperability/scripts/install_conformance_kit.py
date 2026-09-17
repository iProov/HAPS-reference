#!/usr/bin/env python3
"""Install the plugin's pinned, deterministic HAPS kit into a target repository."""

from __future__ import annotations

import argparse
import filecmp
import shutil
from pathlib import Path


class KitError(Exception):
    """The portable kit cannot be installed without an explicit decision."""


def portable_files(root: Path) -> set[Path]:
    return {
        path.relative_to(root)
        for path in root.rglob("*")
        if path.is_file()
        and "__pycache__" not in path.parts
        and path.suffix not in {".pyc", ".pyo"}
    }


def tree_matches(source: Path, destination: Path) -> bool:
    if not source.is_dir() or not destination.is_dir():
        return False
    source_files = portable_files(source)
    destination_files = portable_files(destination)
    return source_files == destination_files and all(
        filecmp.cmp(source / relative, destination / relative, shallow=False)
        for relative in source_files
    )


def source_for(plugin_root: Path) -> Path:
    source = plugin_root / "assets" / "conformance"
    if not source.is_dir():
        raise KitError(f"plugin asset is missing: {source}")
    return source


def destination_for(target: Path) -> Path:
    resolved = target.resolve()
    if resolved.exists() and not resolved.is_dir():
        raise KitError(f"target is not a directory: {resolved}")
    return resolved / ".haps" / "conformance"


def check_kit(plugin_root: Path, target: Path) -> tuple[Path, bool]:
    source = source_for(plugin_root)
    destination = destination_for(target)
    return destination, tree_matches(source, destination)


def install_kit(
    plugin_root: Path, target: Path, *, update: bool = False
) -> tuple[Path, bool]:
    source = source_for(plugin_root)
    destination = destination_for(target)

    if destination.exists() and not update:
        if destination.is_dir() and tree_matches(source, destination):
            return destination, False
        raise KitError(
            f"{destination} already exists and differs; inspect it, then pass "
            "--update only for an explicit kit upgrade"
        )

    if destination.exists():
        shutil.rmtree(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(
        source,
        destination,
        ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "*.pyo"),
    )
    return destination, True


def main() -> int:
    parser = argparse.ArgumentParser(description="Install the pinned HAPS conformance kit.")
    parser.add_argument("--target", type=Path, required=True, help="target repository root")
    parser.add_argument(
        "--update",
        action="store_true",
        help="replace an existing vendored kit after an explicit version update",
    )
    parser.add_argument("--check", action="store_true", help="check that the installed kit matches")
    args = parser.parse_args()

    plugin_root = Path(__file__).resolve().parent.parent

    try:
        if args.check:
            destination, matches = check_kit(plugin_root, args.target)
            if not matches:
                print(f"installed HAPS kit differs at {destination}")
                return 1
            print(f"installed HAPS kit matches at {destination}")
            return 0
        destination, changed = install_kit(plugin_root, args.target, update=args.update)
    except (KitError, OSError) as error:
        parser.error(str(error))

    state = "installed" if changed else "already current"
    print(f"HAPS conformance kit {state} at {destination}")
    print("Experimental draft kit; not production-ready. Passing tests does not establish full")
    print("protocol conformance or that a person saw and approved the correct action.")
    print(f"check local kit digests with: python3 {destination / 'haps_conformance.py'} verify-lock")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
